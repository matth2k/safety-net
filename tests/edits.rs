use safety_net::netlist::Gate;
use safety_net::netlist::GateNetlist;
use safety_net::netlist::Netlist;
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
fn test_clean() {
    let netlist = get_simple_example();
    assert!(netlist.verify().is_ok());
    assert!(!netlist.clean().unwrap());
    let inputs: Vec<_> = netlist.inputs().collect();
    assert_eq!(inputs.len(), 2);
    let _new_cell = netlist
        .insert_gate(and_gate(), "inst_1".into(), &inputs)
        .unwrap();
    assert!(netlist.verify().is_ok());
    assert_eq!(netlist.objects().count(), 4);
    assert!(netlist.clean().unwrap());
    assert_eq!(netlist.objects().count(), 3);
    assert!(!netlist.clean().unwrap());
}
