use circuit::netlist::GateNetlist;

#[test]

fn inputs_w_same_name() {
    let netlist = GateNetlist::new("inputs_w_same_name".to_string());
    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("a".into());
    // Errors because both inputs have the same identifier
    assert!(netlist.verify().is_err());
    // Renaming fixes it
    b.as_net_mut().set_identifier("b".into());
    assert!(netlist.verify().is_err());
    a.expose_with_name("y".into());
    b.expose_with_name("z".into());
    assert!(netlist.verify().is_ok());
}
