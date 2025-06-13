/*!

  Graph utils for the `graph` module.

*/

use crate::circuit::{Instantiable, Net, Object};
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

        for output in netlist.get_output_ports() {
            is_an_output.insert(output.clone());
        }

        for o in netlist.outputs() {
            is_an_output.insert(o.get_net().clone());
        }

        Ok(FanOutTable {
            _netlist: netlist,
            fan_out,
            is_an_output,
        })
    }
}

/// Returns a petgraph representation of the netlist as a directed multi-graph with type [DiGraph<Object, NetLabel>].
pub struct MultiDiGraph<'a, I: Instantiable> {
    _netlist: &'a Netlist<I>,
    graph: DiGraph<Object<I>, Net>,
}

impl<I> MultiDiGraph<'_, I>
where
    I: Instantiable,
{
    /// Return a reference to the graph constructed by this analysis
    pub fn get_graph(&self) -> &DiGraph<Object<I>, Net> {
        &self.graph
    }
}

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
            let id = graph.add_node(obj.get_obj().clone());
            mapping.insert(obj.to_string(), id);
        }

        for connection in netlist.connections() {
            let source = connection.src().unwrap().get_obj().to_string();
            let target = connection.target().unwrap().get_obj().to_string();
            let s_id = mapping[&source];
            let t_id = mapping[&target];
            let net = connection.net();
            graph.add_edge(s_id, t_id, net);
        }

        Ok(Self {
            _netlist: netlist,
            graph,
        })
    }
}
