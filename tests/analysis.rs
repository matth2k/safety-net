use safety_net::attribute::dont_touch_filter;
use safety_net::circuit::Net;
use safety_net::format_id;
use safety_net::graph::FanOutTable;
use safety_net::graph::SimpleCombDepth;
use safety_net::netlist::Gate;
use safety_net::netlist::GateNetlist;
use safety_net::netlist::Netlist;
use safety_net::netlist::iter::DFSIterator;
use std::rc::Rc;

fn and_gate() -> Gate {
    Gate::new_logical("AND".into(), vec!["A".into(), "B".into()], "Y".into())
}

fn get_simple_example() -> Rc<GateNetlist> {
    let netlist = Netlist::new("example".to_string());

    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());

    let instance = netlist
        .insert_gate(and_gate(), "inst_0".into(), &[a, b])
        .unwrap();

    instance.expose_with_name("y".into());

    netlist
}

#[test]
fn test_detect_cycles() {
    let netlist = get_simple_example();

    // No cycles yet.
    let dfs_iter = DFSIterator::new(&netlist, netlist.last().unwrap());
    assert!(!dfs_iter.detect_cycles());

    let input = netlist.inputs().next().unwrap();
    let inverter = Gate::new_logical("INV".into(), vec!["I".into()], "O".into());
    let inverted = netlist
        .insert_gate(inverter, "inst_0".into(), std::slice::from_ref(&input))
        .unwrap();
    assert!(
        netlist
            .replace_net_uses(input, &inverted.get_output(0))
            .is_ok()
    );

    // Now there is a cycle.
    // We replaced the inverter input with invert output.
    // Simple combinational loop.
    let dfs_iter = DFSIterator::new(&netlist, netlist.last().unwrap());
    assert!(dfs_iter.detect_cycles());
}

#[test]
fn test_attr_filter() {
    let netlist = GateNetlist::new("example".to_string());

    let a: Vec<_> = (0..4)
        .map(|i| netlist.insert_input(Net::new_logic(format_id!("input_{}", i))))
        .collect::<Vec<_>>();

    let inst_0 = netlist
        .insert_gate(and_gate(), "inst_0".into(), &[a[0].clone(), a[1].clone()])
        .unwrap();

    let inst_1 = netlist
        .insert_gate(and_gate(), "inst_1".into(), &[a[1].clone(), a[2].clone()])
        .unwrap();

    netlist
        .insert_gate(
            and_gate(),
            "inst_2".into(),
            &[inst_0.into(), inst_1.clone().into()],
        )
        .unwrap();

    inst_1.set_attribute("dont_touch".into());
    for dt in dont_touch_filter(&*netlist) {
        assert!(dt == inst_1);
    }

    let filter = dont_touch_filter(&*netlist);

    for obj in netlist.objects() {
        if obj == inst_1 {
            assert!(filter.has(&obj));
        } else {
            assert!(!filter.has(&obj));
        }
    }

    assert_eq!(filter.keys().len(), 1)
}

#[cfg(feature = "graph")]
#[test]
fn test_petgraph() {
    use safety_net::graph::MultiDiGraph;

    let netlist = get_simple_example();

    let petgraph = netlist.get_analysis::<MultiDiGraph<_>>();
    assert!(petgraph.is_ok());
    let petgraph = petgraph.unwrap();
    let graph = petgraph.get_graph();
    // Outputs are a pseudo node
    assert_eq!(graph.node_count(), 4);
    assert_eq!(graph.edge_count(), 3);
}

#[test]
fn test_comb_depth() {
    let netlist = get_simple_example();
    let depth_info = netlist.get_analysis::<SimpleCombDepth<_>>();
    assert!(depth_info.is_ok());
    let depth_info = depth_info.unwrap();

    let gate = netlist.last().unwrap();

    assert_eq!(depth_info.get_comb_depth(&gate), Some(1));
    assert_eq!(depth_info.get_max_depth(), 1);
}

#[test]
fn test_fanout_table() {
    let netlist = get_simple_example();
    let fanout_table = netlist.get_analysis::<FanOutTable<_>>();
    assert!(fanout_table.is_ok());
    let fanout_table = fanout_table.unwrap();
    let gate = netlist.last().unwrap();
    // Outputs don't have users that are nodes
    assert_eq!(fanout_table.get_node_users(&gate).count(), 0);
}
