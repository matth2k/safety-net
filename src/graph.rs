/*!

  Graph utils for the `graph` module.

*/

use crate::circuit::{Instantiable, Net};
use crate::error::Error;
#[cfg(feature = "graph")]
use crate::netlist::Connection;
use crate::netlist::{NetRef, Netlist};
#[cfg(feature = "graph")]
use petgraph::graph::DiGraph;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

/// A common trait of analyses than can be performed on a netlist.
/// An analysis becomes stale when the netlist is modified.
pub trait Analysis<'a, I: Instantiable>
where
    Self: Sized + 'a,
{
    /// Construct the analysis to the current state of the netlist.
    fn build(netlist: &'a Netlist<I>) -> Result<Self, Error>;
}

/// A table that maps nets to the circuit nodes they drive
pub struct FanOutTable<'a, I: Instantiable> {
    /// A reference to the underlying netlist
    _netlist: &'a Netlist<I>,
    /// Maps a net to the list of nodes it drives
    net_fan_out: HashMap<Net, Vec<NetRef<I>>>,
    /// Maps a node to the list of nodes it drives
    node_fan_out: HashMap<NetRef<I>, Vec<NetRef<I>>>,
    /// Contains nets which are outputs
    is_an_output: HashSet<Net>,
}

impl<I> FanOutTable<'_, I>
where
    I: Instantiable,
{
    /// Returns an iterator to the circuit nodes that use `net`.
    pub fn get_net_users(&self, net: &Net) -> impl Iterator<Item = NetRef<I>> {
        self.net_fan_out
            .get(net)
            .into_iter()
            .flat_map(|users| users.iter().cloned())
    }

    /// Returns an iterator to the circuit nodes that use `node`.
    pub fn get_node_users(&self, node: &NetRef<I>) -> impl Iterator<Item = NetRef<I>> {
        self.node_fan_out
            .get(node)
            .into_iter()
            .flat_map(|users| users.iter().cloned())
    }

    /// Returns `true` if the net has any used by any cells in the circuit
    /// This does incude nets that are only used as outputs.
    pub fn net_has_uses(&self, net: &Net) -> bool {
        (self.net_fan_out.contains_key(net) && !self.net_fan_out.get(net).unwrap().is_empty())
            || self.is_an_output.contains(net)
    }
}

impl<'a, I> Analysis<'a, I> for FanOutTable<'a, I>
where
    I: Instantiable,
{
    fn build(netlist: &'a Netlist<I>) -> Result<Self, Error> {
        let mut net_fan_out: HashMap<Net, Vec<NetRef<I>>> = HashMap::new();
        let mut node_fan_out: HashMap<NetRef<I>, Vec<NetRef<I>>> = HashMap::new();
        let mut is_an_output: HashSet<Net> = HashSet::new();

        // This can only be fully-correct on a verified netlist.
        netlist.verify()?;

        for c in netlist.connections() {
            if let Entry::Vacant(e) = net_fan_out.entry(c.net()) {
                e.insert(vec![c.target().unwrap()]);
            } else {
                net_fan_out
                    .get_mut(&c.net())
                    .unwrap()
                    .push(c.target().unwrap());
            }

            if let Entry::Vacant(e) = node_fan_out.entry(c.src().unwrap()) {
                e.insert(vec![c.target().unwrap()]);
            } else {
                node_fan_out
                    .get_mut(&c.src().unwrap())
                    .unwrap()
                    .push(c.target().unwrap());
            }
        }

        for (o, n) in netlist.outputs() {
            is_an_output.insert(o.as_net().clone());
            is_an_output.insert(n);
        }

        Ok(FanOutTable {
            _netlist: netlist,
            net_fan_out,
            node_fan_out,
            is_an_output,
        })
    }
}

/// A simple example to analyze the logic levels of a netlist.
/// This analysis checks for cycles, but it doesn't check for registers.
/// Result of combinational depth analysis for a single net.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CombDepthResult {
    /// Signal has no driver
    Undefined,
    /// Signal is along a cycle
    CombCycle,
    /// Integer logic level
    Depth(usize),
}

/// Computes the combinational depth of each net in a netlist.
///
/// Each net is classified as having a defined depth, being undefined,
/// or participating in a combinational cycle.
pub struct SimpleCombDepth<'a, I: Instantiable> {
    _netlist: &'a Netlist<I>,
    results: HashMap<NetRef<I>, CombDepthResult>,
    /// Max will be None whenever no outputs in the whole netlist have a well defined combinational depth
    /// for example if they are all undefined or they all partake in a cycle
    max_depth: Option<usize>,
}

impl<I> SimpleCombDepth<'_, I>
where
    I: Instantiable,
{
    /// Returns the logic level of a node in the circuit.
    pub fn get_comb_depth(&self, node: &NetRef<I>) -> Option<CombDepthResult> {
        self.results.get(node).copied()
    }

    /// Returns the maximum logic level of the circuit.
    pub fn get_max_depth(&self) -> Option<usize> {
        self.max_depth
    }
}
impl<'a, I> Analysis<'a, I> for SimpleCombDepth<'a, I>
where
    I: Instantiable,
{
    fn build(netlist: &'a Netlist<I>) -> Result<Self, Error> {
        let mut results: HashMap<NetRef<I>, CombDepthResult> = HashMap::new();
        let mut visiting: HashSet<NetRef<I>> = HashSet::new();
        let mut max_depth: Option<usize> = None;

        fn compute<I: Instantiable>(
            node: NetRef<I>,
            netlist: &Netlist<I>,
            results: &mut HashMap<NetRef<I>, CombDepthResult>,
            visiting: &mut HashSet<NetRef<I>>,
        ) -> CombDepthResult {
            // Memoized result
            if let Some(&r) = results.get(&node) {
                return r;
            }

            // Cycle detection
            if visiting.contains(&node) {
                for n in visiting.iter() {
                    results.insert(n.clone(), CombDepthResult::CombCycle);
                }
                return CombDepthResult::CombCycle;
            }

            // Input nodes have depth 0
            if node.is_an_input() {
                let r = CombDepthResult::Depth(0);
                results.insert(node.clone(), r);
                return r;
            }

            visiting.insert(node.clone());

            let mut max_depth = 0;
            let mut is_undefined = false;

            for i in 0..node.get_num_input_ports() {
                let driver = match netlist.get_driver(node.clone(), i) {
                    Some(d) => d,
                    None => {
                        is_undefined = true;
                        continue;
                    }
                };

                if let Some(inst) = driver.get_instance_type()
                    && inst.is_seq()
                {
                    continue;
                }

                match compute(driver, netlist, results, visiting) {
                    CombDepthResult::Depth(d) => {
                        max_depth = max_depth.max(d);
                    }
                    CombDepthResult::Undefined => {
                        is_undefined = true;
                    }
                    CombDepthResult::CombCycle => {
                        let r = CombDepthResult::CombCycle;
                        results.insert(node.clone(), r);
                        visiting.remove(&node);
                        return r;
                    }
                }
            }

            visiting.remove(&node);
            let r = if is_undefined {
                CombDepthResult::Undefined
            } else {
                CombDepthResult::Depth(max_depth + 1)
            };
            results.insert(node.clone(), r);
            r
        }

        for (driven, _) in netlist.outputs() {
            let node = driven.unwrap();
            let r = compute(node, netlist, &mut results, &mut visiting);

            if let CombDepthResult::Depth(d) = r {
                max_depth = Some(max_depth.map_or(d, |m| m.max(d)));
            }
        }

        for node in netlist.matches(|inst| inst.is_seq()) {
            results.insert(node.clone(), CombDepthResult::Depth(0));
            for i in 0..node.get_num_input_ports() {
                if let Some(driver) = netlist.get_driver(node.clone(), i) {
                    if driver.get_instance_type().is_some_and(|inst| inst.is_seq()) {
                        continue;
                    }

                    let r = compute(driver, netlist, &mut results, &mut visiting);
                    if let CombDepthResult::Depth(d) = r {
                        max_depth = Some(max_depth.map_or(d, |m| m.max(d)));
                    }
                }
            }
        }

        Ok(SimpleCombDepth {
            _netlist: netlist,
            results,
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

    /// Iterates through a [greedy feedback arc set](https://doi.org/10.1016/0020-0190(93)90079-O) for the graph.
    pub fn greedy_feedback_arcs(&self) -> impl Iterator<Item = Connection<I>> {
        petgraph::algo::feedback_arc_set::greedy_feedback_arc_set(&self.graph)
            .map(|e| match e.weight() {
                Edge::Connection(c) => c,
                _ => unreachable!("Outputs should be sinks"),
            })
            .cloned()
    }
}

#[cfg(feature = "graph")]
impl<'a, I> Analysis<'a, I> for MultiDiGraph<'a, I>
where
    I: Instantiable,
{
    fn build(netlist: &'a Netlist<I>) -> Result<Self, Error> {
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
            let t_id = graph.add_node(Node::Pseudo(format!("Output({n})")));
            graph.add_edge(s_id, t_id, Edge::Pseudo(o.as_net().clone()));
        }

        Ok(Self {
            _netlist: netlist,
            graph,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{format_id, netlist::*};

    fn full_adder() -> Gate {
        Gate::new_logical_multi(
            "FA".into(),
            vec!["CIN".into(), "A".into(), "B".into()],
            vec!["S".into(), "COUT".into()],
        )
    }

    fn ripple_adder() -> GateNetlist {
        let netlist = Netlist::new("ripple_adder".to_string());
        let bitwidth = 4;

        // Add the the inputs
        let a = netlist.insert_input_escaped_logic_bus("a".to_string(), bitwidth);
        let b = netlist.insert_input_escaped_logic_bus("b".to_string(), bitwidth);
        let mut carry: DrivenNet<Gate> = netlist.insert_input("cin".into());

        for (i, (a, b)) in a.into_iter().zip(b.into_iter()).enumerate() {
            // Instantiate a full adder for each bit
            let fa = netlist
                .insert_gate(full_adder(), format_id!("fa_{i}"), &[carry, a, b])
                .unwrap();

            // Expose the sum
            fa.expose_net(&fa.get_net(0)).unwrap();

            carry = fa.find_output(&"COUT".into()).unwrap();

            if i == bitwidth - 1 {
                // Last full adder, expose the carry out
                fa.get_output(1).expose_with_name("cout".into()).unwrap();
            }
        }

        netlist.reclaim().unwrap()
    }

    #[test]
    fn fanout_table() {
        let netlist = ripple_adder();
        let analysis = FanOutTable::build(&netlist);
        assert!(analysis.is_ok());
        let analysis = analysis.unwrap();
        assert!(netlist.verify().is_ok());

        for item in netlist.objects().filter(|o| !o.is_an_input()) {
            // Sum bit has no users (it is a direct output)
            assert!(
                analysis
                    .get_net_users(&item.find_output(&"S".into()).unwrap().as_net())
                    .next()
                    .is_none(),
                "Sum bit should not have users"
            );

            assert!(
                item.get_instance_name().is_some(),
                "Item should have a name. Filtered inputs"
            );

            let net = item.find_output(&"COUT".into()).unwrap().as_net().clone();
            let mut cout_users = analysis.get_net_users(&net);
            if item.get_instance_name().unwrap().to_string() != "fa_3" {
                assert!(cout_users.next().is_some(), "Carry bit should have users");
            }

            assert!(
                cout_users.next().is_none(),
                "Carry bit should have 1 or 0 user"
            );
        }
    }
}
