use safety_net::circuit::Identifier;
use safety_net::circuit::Instantiable;
use safety_net::circuit::Net;
use safety_net::netlist::Gate;
use safety_net::netlist::GateNetlist;

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
        vec!["\\input[0]".into()],
        "buf_out".into(),
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

#[cfg(feature = "serde")]
#[test]
fn test_basic_serialize() {
    use safety_net::netlist::serde::netlist_deserialize;
    use std::{io::Cursor, rc::Rc};

    let netlist = GateNetlist::new("top".to_string());
    {
        let input = netlist.insert_input(Net::new_logic("in".into()));
        netlist.expose_net_with_name(input, "out".into());
    }

    let mut buf: Vec<u8> = Vec::new();
    let netlist = netlist.reclaim().unwrap();
    assert!(netlist.serialize(&mut buf).is_ok());

    let reader = Cursor::new(buf);
    let netlist: Result<Rc<GateNetlist>, serde_json::Error> = netlist_deserialize(reader);

    assert!(netlist.is_ok());
    let netlist = netlist.unwrap();
    assert_eq!(netlist.objects().count(), 1);
    assert_eq!(netlist.inputs().count(), 1);

    let inst = netlist.last().unwrap();
    assert!(inst.get_instance_type().is_none());
    assert_eq!(*inst.as_net(), "in".into());
}

#[test]
fn test_empty_netlist() {
    let netlist = GateNetlist::new("min_module".to_string());
    let a = netlist.insert_input("a".into());
    // The designed behavior here should maybe change.
    // Should the output get delete alongside the driving netref?
    a.clone().expose_with_name("y".into());
    assert!(!netlist.outputs().is_empty());
    netlist.delete_net_uses(a.unwrap()).unwrap();
    assert!(netlist.outputs().is_empty());
}
