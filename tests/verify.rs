use circuit::circuit::Identifier;
use circuit::netlist::Gate;
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

#[test]
fn test_bus_operations() {
    let netlist = GateNetlist::new("test_bus_operations".to_string());

    // Create a 4-bit input bus
    let input_bus = netlist.insert_input_escaped_logic_bus("input_bus".to_string(), 4);

    // Check that bus has been instantiated correctly
    assert_eq!(input_bus.len(), 4);
    for (i, bit) in input_bus.iter().enumerate() {
        assert!(bit.is_an_input());
        let identifier = bit.get_identifier();
        assert!(identifier.is_escaped());
        assert_eq!(identifier.get_name(), format!("input_bus[{i}]"));
    }

    // Test that we can connect bus bits to gates
    let buffer_gate = Gate::new_logical(
        Identifier::from("buf1"),
        vec!["input_bus[0]".to_string()],
        "buf_out".to_string(),
    );

    let buffer_1 = netlist
        .insert_gate(
            buffer_gate,
            Identifier::from("buf1"),
            &[input_bus[0].clone()],
        )
        .expect("Failed to connect bus bit to buffer");

    // Verify the netlist
    buffer_1.expose_with_name("buf_out".into());
    assert!(netlist.verify().is_ok());
}
