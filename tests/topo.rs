use safety_net::{Gate, GateNetlist};
use safety_net::{Net, SimpleCombDepth};
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
fn get_dag() -> (Rc<GateNetlist>, HashMap<Net, usize>) {
    let netlist = GateNetlist::new("comb_loop".to_string());
    let mut map = HashMap::new();

    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());

    map.insert(a.as_net().clone(), 0);
    map.insert(b.as_net().clone(), 0);

    let c = netlist
        .insert_gate(inv(), "inst_0".into(), &[a.clone()])
        .unwrap()
        .get_output(0);
    map.insert(c.as_net().clone(), 1);

    let and_a_b = netlist
        .insert_gate(and(), "inst_1".into(), &[a.clone(), b])
        .unwrap()
        .get_output(0);
    map.insert(and_a_b.as_net().clone(), 1);

    let or_gate = netlist
        .insert_gate(or3(), "inst_2".into(), &[c.clone(), and_a_b, a.clone()])
        .unwrap()
        .get_output(0);
    map.insert(or_gate.as_net().clone(), 2);

    let out_gate = netlist
        .insert_gate(or3(), "inst_2".into(), &[c, or_gate, a])
        .unwrap();

    map.insert(out_gate.as_net().clone(), 3);

    out_gate.expose_with_name("y".into());

    (netlist, map)
}

#[test]
fn test_comb_loop() {
    let netlist = get_comb_loop();
    let depth_info = netlist.get_analysis::<SimpleCombDepth<_>>();

    // Even though we have cycles, the combinational depth analysis should complete
    assert!(depth_info.is_ok());
    let depth_info = depth_info.unwrap();

    let gate = netlist.last().unwrap();

    // The gate is part of a loop, so it should return None
    assert_eq!(depth_info.get_comb_depth(&gate), None);

    let input = netlist.inputs().next().unwrap();
    assert_eq!(depth_info.get_comb_depth(&input.unwrap()), Some(0));
}

#[test]
fn test_dag() {
    let (netlist, map) = get_dag();
    let depth_info = netlist.get_analysis::<SimpleCombDepth<_>>();

    assert!(depth_info.is_ok());
    let depth_info = depth_info.unwrap();

    // Verify the analysis against expected values
    for netref in netlist.objects() {
        assert_eq!(
            depth_info.get_comb_depth(&netref),
            map.get(&netref.as_net()).cloned()
        );
    }
}
