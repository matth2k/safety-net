/*!

  Graph utils for the `graph` module.

*/

use crate::circuit::{Instantiable, Net};
#[cfg(feature = "graph")]
use crate::netlist::Connection;
use crate::netlist::iter::DFSIterator;
use crate::netlist::{NetRef, Netlist};
#[cfg(feature = "graph")]
use petgraph::graph::DiGraph;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

/// A common trait of analyses than can be performed on a netlist.
pub trait Analysis<'a, I: Instantiable>
where
    Self: Sized + 'a,
{
    /// Construct the analysis
    fn build(netlist: &'a Netlist<I>) -> Result<Self, String>;
}

/// A table that maps nets to the circuit nodes they drive
pub struct FanOutTable<'a, I: Instantiable> {
    // A reference to the underlying netlist
    _netlist: &'a Netlist<I>,
    // Maps a net to the list of nets it drives
    fan_out: HashMap<Net, Vec<NetRef<I>>>,
    /// Contains nets which are outputs
    is_an_output: HashSet<Net>,
}

impl<I> FanOutTable<'_, I>
where
    I: Instantiable,
{
    /// Returns an iterator to the circuit nodes that use `net`.
    pub fn get_users(&self, net: &Net) -> impl Iterator<Item = NetRef<I>> {
        self.fan_out
            .get(net)
            .into_iter()
            .flat_map(|users| users.iter().cloned())
    }

    /// Returns `true` if the net has any used by any cells in the circuit
    /// This does incude nets that are only used as outputs.
    pub fn has_uses(&self, net: &Net) -> bool {
        (self.fan_out.contains_key(net) && !self.fan_out.get(net).unwrap().is_empty())
            || self.is_an_output.contains(net)
    }
}

impl<'a, I> Analysis<'a, I> for FanOutTable<'a, I>
where
    I: Instantiable,
{
    fn build(netlist: &'a Netlist<I>) -> Result<Self, String> {
        let mut fan_out: HashMap<Net, Vec<NetRef<I>>> = HashMap::new();
        let mut is_an_output: HashSet<Net> = HashSet::new();

        for c in netlist.connections() {
            if let Entry::Vacant(e) = fan_out.entry(c.net()) {
                e.insert(vec![c.target().unwrap()]);
            } else {
                fan_out.get_mut(&c.net()).unwrap().push(c.target().unwrap());
            }
        }

        for (o, n) in netlist.outputs() {
            is_an_output.insert(o.get_net().clone());
            is_an_output.insert(n);
        }

        Ok(FanOutTable {
            _netlist: netlist,
            fan_out,
            is_an_output,
        })
    }
}

/// An simple example to analyze the logic levels of a netlist.
/// This analysis checks for cycles, but it doesn't check for registers.
pub struct SimpleCombDepth<'a, I: Instantiable> {
    // A reference to the underlying netlist
    _netlist: &'a Netlist<I>,
    // Maps a net to its logic level as a DAG
    comb_depth: HashMap<Net, usize>,
    /// The maximum depth of the circuit
    max_depth: usize,
}

impl<I> SimpleCombDepth<'_, I>
where
    I: Instantiable,
{
    /// Returns the logic level of a net in the circuit.
    pub fn get_comb_depth(&self, net: &Net) -> Option<usize> {
        self.comb_depth.get(net).cloned()
    }

    /// Returns the maximum logic level of the circuit.
    pub fn get_max_depth(&self) -> usize {
        self.max_depth
    }
}

impl<'a, I> Analysis<'a, I> for SimpleCombDepth<'a, I>
where
    I: Instantiable,
{
    fn build(netlist: &'a Netlist<I>) -> Result<Self, String> {
        let mut comb_depth: HashMap<Net, usize> = HashMap::new();

        let mut nodes = Vec::new();
        for (driven, _) in netlist.outputs() {
            let mut dfs = DFSIterator::new(netlist, driven.unwrap());
            while let Some(n) = dfs.next() {
                if dfs.check_cycles() {
                    return Err("Cycle detected in the netlist".to_string());
                }
                nodes.push(n);
            }
        }
        nodes.reverse();

        for node in nodes {
            if node.is_an_input() {
                comb_depth.insert(node.as_net().clone(), 0);
            } else {
                // TODO(matth2k): get_driver_net() relies on a weak reference. Rewrite without it.
                let max_depth: usize = (0..node.get_num_input_ports())
                    .filter_map(|i| netlist.get_driver_net(node.clone(), i))
                    .filter_map(|n| comb_depth.get(&n))
                    .max()
                    .cloned()
                    .unwrap_or(usize::MAX);
                for net in node.nets() {
                    comb_depth.insert(net, max_depth + 1);
                }
            }
        }

        let max_depth = comb_depth.values().max().cloned().unwrap_or(0);

        Ok(SimpleCombDepth {
            _netlist: netlist,
            comb_depth,
            max_depth,
        })
    }
}

/// An enum to provide pseudo-nodes for any misc user-programmable behavior.
#[cfg(feature = "graph")]
#[derive(Debug, Clone)]
pub enum Node<I: Instantiable, T: Clone + std::fmt::Debug + std::fmt::Display> {
    /// A 'real' circuit node
    NetRef(NetRef<I>),
    /// Any other user-programmable node
    Pseudo(T),
}

#[cfg(feature = "graph")]
impl<I, T> std::fmt::Display for Node<I, T>
where
    I: Instantiable,
    T: Clone + std::fmt::Debug + std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Node::NetRef(nr) => nr.fmt(f),
            Node::Pseudo(t) => std::fmt::Display::fmt(t, f),
        }
    }
}

/// An enum to provide pseudo-edges for any misc user-programmable behavior.
#[cfg(feature = "graph")]
#[derive(Debug, Clone)]
pub enum Edge<I: Instantiable, T: Clone + std::fmt::Debug + std::fmt::Display> {
    /// A 'real' circuit connection
    Connection(Connection<I>),
    /// Any other user-programmable node
    Pseudo(T),
}

#[cfg(feature = "graph")]
impl<I, T> std::fmt::Display for Edge<I, T>
where
    I: Instantiable,
    T: Clone + std::fmt::Debug + std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Edge::Connection(c) => c.fmt(f),
            Edge::Pseudo(t) => std::fmt::Display::fmt(t, f),
        }
    }
}

/// Returns a petgraph representation of the netlist as a directed multi-graph with type [DiGraph<Object, NetLabel>].
#[cfg(feature = "graph")]
pub struct MultiDiGraph<'a, I: Instantiable> {
    _netlist: &'a Netlist<I>,
    graph: DiGraph<Node<I, String>, Edge<I, Net>>,
}

#[cfg(feature = "graph")]
impl<I> MultiDiGraph<'_, I>
where
    I: Instantiable,
{
    /// Return a reference to the graph constructed by this analysis
    pub fn get_graph(&self) -> &DiGraph<Node<I, String>, Edge<I, Net>> {
        &self.graph
    }
}

#[cfg(feature = "graph")]
impl<'a, I> Analysis<'a, I> for MultiDiGraph<'a, I>
where
    I: Instantiable,
{
    fn build(netlist: &'a Netlist<I>) -> Result<Self, String> {
        // If we verify, we can hash by name
        netlist.verify()?;
        let mut mapping = HashMap::new();
        let mut graph = DiGraph::new();

        for obj in netlist.objects() {
            let id = graph.add_node(Node::NetRef(obj.clone()));
            mapping.insert(obj.to_string(), id);
        }

        for connection in netlist.connections() {
            let source = connection.src().unwrap().get_obj().to_string();
            let target = connection.target().unwrap().get_obj().to_string();
            let s_id = mapping[&source];
            let t_id = mapping[&target];
            graph.add_edge(s_id, t_id, Edge::Connection(connection));
        }

        // Finally, add the output connections
        for (o, n) in netlist.outputs() {
            let s_id = mapping[&o.clone().unwrap().get_obj().to_string()];
            let t_id = graph.add_node(Node::Pseudo(format!("Output({})", n)));
            graph.add_edge(s_id, t_id, Edge::Pseudo(o.get_net().clone()));
        }

        Ok(Self {
            _netlist: netlist,
            graph,
        })
    }
}
