use safety_net::Net;
use safety_net::graph::{CombDepthInfo, CombDepthResult};
use safety_net::{Gate, GateNetlist, Netlist};
use std::collections::HashMap;
use std::rc::Rc;

fn and() -> Gate {
    Gate::new_logical("AND2".into(), vec!["A".into(), "B".into()], "Y".into())
}

fn or3() -> Gate {
    Gate::new_logical(
        "OR3".into(),
        vec!["A".into(), "B".into(), "C".into()],
        "Y".into(),
    )
}

fn inv() -> Gate {
    Gate::new_logical("INV".into(), vec!["A".into()], "Y".into())
}

fn get_simple_example() -> Rc<GateNetlist> {
    let netlist = Netlist::new("example".to_string());

    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());

    let instance = netlist
        .insert_gate(and(), "inst_0".into(), &[a, b])
        .unwrap();

    instance.expose_with_name("y".into());

    netlist
}

fn get_comb_loop() -> Rc<GateNetlist> {
    let netlist = GateNetlist::new("comb_loop".to_string());

    let a = netlist.insert_input("a".into());

    let instance = netlist.insert_gate_disconnected(and(), "inst_0".into());

    instance.get_input(0).connect(a);
    instance.get_input(1).connect(instance.get_output(0));
    instance.expose_with_name("y".into());

    netlist
}

/// Returns the netlist and a map of expected combinational depths
fn get_dag() -> (Rc<GateNetlist>, HashMap<Net, CombDepthResult>) {
    let netlist = GateNetlist::new("comb_loop".to_string());
    let mut map = HashMap::new();

    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());

    map.insert(a.as_net().clone(), CombDepthResult::Depth(0));
    map.insert(b.as_net().clone(), CombDepthResult::Depth(0));

    let c = netlist
        .insert_gate(inv(), "inst_0".into(), std::slice::from_ref(&a))
        .unwrap()
        .get_output(0);
    map.insert(c.as_net().clone(), CombDepthResult::Depth(1));

    let and_a_b = netlist
        .insert_gate(and(), "inst_1".into(), &[a.clone(), b])
        .unwrap()
        .get_output(0);
    map.insert(and_a_b.as_net().clone(), CombDepthResult::Depth(1));

    let or_gate = netlist
        .insert_gate(or3(), "inst_2".into(), &[c.clone(), and_a_b, a.clone()])
        .unwrap()
        .get_output(0);
    map.insert(or_gate.as_net().clone(), CombDepthResult::Depth(2));

    let out_gate = netlist
        .insert_gate(or3(), "inst_3".into(), &[c, or_gate, a])
        .unwrap();

    map.insert(out_gate.as_net().clone(), CombDepthResult::Depth(3));

    out_gate.expose_with_name("y".into());

    (netlist, map)
}

#[test]
fn test_comb_loop() {
    let netlist = get_comb_loop();
    let depth_info = netlist.get_analysis::<CombDepthInfo<_>>();

    // Even though we have cycles, the combinational depth analysis should complete
    assert!(depth_info.is_ok());
    let depth_info = depth_info.unwrap();

    let gate = netlist.last().unwrap();

    // The gate is part of a loop, so it should return CombDepthResult::CombCycle
    assert_eq!(
        depth_info.get_comb_depth(&gate),
        Some(CombDepthResult::CombCycle)
    );

    let input = netlist.inputs().next().unwrap();
    assert_eq!(
        depth_info.get_comb_depth(&input.unwrap()),
        Some(CombDepthResult::Depth(0))
    );
}

#[test]
fn test_dag() {
    let (netlist, map) = get_dag();
    let depth_info = netlist.get_analysis::<CombDepthInfo<_>>();

    assert!(depth_info.is_ok());
    let depth_info = depth_info.unwrap();

    // Verify the analysis against expected values
    for netref in netlist.objects() {
        assert_eq!(
            depth_info.get_comb_depth(&netref),
            map.get(&netref.as_net()).copied()
        );
    }
}

#[test]
fn test_comb_depth() {
    let netlist = get_simple_example();
    let depth_info = netlist.get_analysis::<CombDepthInfo<_>>();
    assert!(depth_info.is_ok());
    let depth_info = depth_info.unwrap();

    let gate = netlist.last().unwrap();

    assert_eq!(
        depth_info.get_comb_depth(&gate).unwrap(),
        CombDepthResult::Depth(1)
    );
    assert_eq!(depth_info.get_max_depth(), Some(1));
}

#[test]
fn test_comb_depth_dag_shared_subgraph() {
    let netlist = Netlist::new("dag".to_string());

    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());
    let c = netlist.insert_input("c".into());

    let and = netlist
        .insert_gate(and(), "and".into(), &[a.clone(), b.clone()])
        .unwrap();

    let or = netlist
        .insert_gate(
            Gate::new_logical("OR".into(), vec!["A".into(), "B".into()], "Y".into()),
            "or".into(),
            &[and.clone().into(), c.clone()],
        )
        .unwrap();

    or.expose_with_name("y".into());
    let or_node = netlist.last().unwrap();

    let depth_info = netlist.get_analysis::<CombDepthInfo<_>>().unwrap();

    assert_eq!(
        depth_info.get_comb_depth(&and).unwrap(),
        CombDepthResult::Depth(1)
    );
    assert_eq!(
        depth_info.get_comb_depth(&or_node).unwrap(),
        CombDepthResult::Depth(2)
    );
    assert_eq!(depth_info.get_max_depth(), Some(2));

    let p = depth_info.get_crit_input(&or_node);
    assert!(p.is_some());

    let p = depth_info.build_critical_path().unwrap();
    assert!(p.len() == 2);
    assert_eq!(p[0], or_node);
    assert_eq!(p[1], and);
}

#[test]
fn test_comb_depth_incomplete() {
    let netlist = Netlist::new("incomplete".to_string());

    let a = netlist.insert_input("a".into());

    // Create AND gate but do NOT connect all inputs
    let and = netlist.insert_gate_disconnected(and(), "and".into());

    // Connect only one input
    and.find_input(&"A".into()).unwrap().connect(a);
    // "B" is left unconnected → incomplete

    and.expose_with_name("y".into());

    let depth_info = netlist.get_analysis::<CombDepthInfo<_>>().unwrap();
    let and_node = netlist.last().unwrap();
    assert_eq!(
        depth_info.get_comb_depth(&and_node).unwrap(),
        CombDepthResult::Undefined
    );
}

#[test]
fn test_comb_depth_cycle() {
    let netlist = Netlist::new("cycle".to_string());

    let inv = netlist.insert_gate_disconnected(
        Gate::new_logical("INV".into(), vec!["I".into()], "O".into()),
        "inv".into(),
    );

    let input = inv.find_input(&"I".into()).unwrap();
    let output = inv.get_output(0);

    // Create combinational loop
    input.connect(output);

    inv.expose_with_name("y".into());

    let depth_info = netlist.get_analysis::<CombDepthInfo<_>>().unwrap();
    let inv_node = netlist.last().unwrap();

    assert_eq!(
        depth_info.get_comb_depth(&inv_node).unwrap(),
        CombDepthResult::CombCycle
    );
}
