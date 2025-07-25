use safety_net::circuit::Net;
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
fn test_io() {
    let netlist = get_simple_example();
    let netlist = netlist.reclaim().unwrap();

    assert_eq!(netlist.inputs().count(), 2);
    assert_eq!(netlist.outputs().len(), 1);

    let (output, o_net) = netlist.outputs().first().unwrap().clone();
    let o_port = output.get_port();
    let o_port_alt = output
        .clone()
        .unwrap()
        .get_instance_type()
        .unwrap()
        .get_single_output_port()
        .clone();

    // The port
    assert_eq!(o_port, o_port_alt);
    let correct = Net::new_logic("Y".into());
    assert_eq!(o_port, correct);

    // The output net
    let correct = Net::new_logic("y".into());
    assert_eq!(o_net, correct);

    // The cell output
    let correct = Net::new_logic("inst_0_Y".into());
    assert_eq!(output.as_net().clone(), correct);
}
