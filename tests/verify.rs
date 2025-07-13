use circuit::circuit::Identifier;
use circuit::circuit::Instantiable;
use circuit::netlist::Gate;
use circuit::netlist::GateNetlist;
use circuit::circuit::Net;
use std::io::Write;

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
        vec!["input[0]".to_string()],
        "buf_out".to_string(),
    );

    let buffer_1 = netlist
        .insert_gate(
            buffer_gate,
            Identifier::from("buf1_inst"),
            &[input_bus[0].clone()],
        )
        .expect("Failed to connect bus bit to buffer");

    // Verify the netlist
    assert!(
        buffer_1
            .get_instance_type()
            .unwrap()
            .get_input_port(0)
            .get_identifier()
            .is_escaped()
    );
    buffer_1.expose_with_name("buf_out".into());
    assert!(netlist.verify().is_ok());
}

#[test]
fn driven_net() {
    let netlist = GateNetlist::new("driven_net".to_string());
    
    let in_0 = netlist.insert_input(Net::new_logic("in_0".to_string()));
    let in_1 = netlist.insert_input(Net::new_logic("in_1".to_string()));
    let in_2 = netlist.insert_input(Net::new_logic("in_2".to_string()));

    let gate_single = Gate::new_logical(
        Identifier::from("gate_single"),
        vec!["in_0".to_string(), "in_1".to_string()],
        "out_0".to_string()
    );

    let gate_s = netlist.insert_gate(
        gate_single,
        Identifier::from("gate_single"),
        &[in_0, in_1]
    ).expect("Failed to insert gate with single output");

    assert_eq!(gate_s.get_output(0).get_port().get_identifier().get_name(), 
                "out_0");
    
    let gate_multi = Gate::new_logical_multi(
        Identifier::from("gate_multi"),
        vec!["multi".to_string()],
        vec!["out_1".to_string(), "out_2".to_string()]
    );

    let gate_m = netlist.insert_gate_disconnected(
        gate_multi,
        Identifier::from("gate_multi")
    ).expect("Failed to insert gate with multiple outputs");

    // check if DrivenNet identifier can be changed
    in_2.as_net_mut().set_identifier(Identifier::from("name"));
    assert_eq!(in_2.get_identifier().get_name(), 
                "name");
    in_2.as_net_mut().set_identifier(Identifier::from("in_2"));

    // connect, disconnect, and reconnect DrivenNet and InputPort
    in_2.connect(gate_m.get_input(0));
    assert_eq!(gate_m.get_input(0).get_driver()
                    .expect("get_driver didn't return the driving DrivenNet")
                    .get_identifier().get_name(), "in_2");
    let disconnected = gate_m.get_input(0).disconnect()
                            .expect("Failed to disconnect InputPort");
    assert_eq!(disconnected.as_net().get_identifier().get_name(), "in_2");  
    gate_m.get_input(0).connect(disconnected);
    assert_eq!(gate_m.get_input(0).get_driver()
                    .expect("get_driver didn't return the driving DrivenNet")
                    .as_net().get_identifier().get_name(), "in_2");

    // expose outputs
    let out_0 = netlist.expose_net_with_name(gate_s.get_output(0), 
                                            Identifier::from("out_0"));
    let out_1 = netlist.expose_net(gate_m.get_output(0))
        .expect("Failed to expose net with multiple outputs");
    let out_2 = netlist.expose_net(gate_m.get_output(1))
        .expect("Failed to expose net with multiple outputs");
    assert!(out_0.is_top_level_output());
    assert!(out_1.is_top_level_output());
    assert!(out_2.is_top_level_output());

    for connection in netlist.connections() {
        std::io::stdout().lock().write_all(
            format!("Connection: {} -> {} on net {}\n",
                    connection.src().get_identifier().get_name(),
                    connection.target().get_port().get_identifier().get_name(),
                    connection.net().get_identifier().get_name()).as_bytes()
        ).expect("Failed to write connection info to stdout");
    }

    assert!(netlist.verify().is_ok());
}