use safety_net::assert_verilog_eq;
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

#[test]
fn test_replace() {
    let netlist = get_simple_example();
    let input = netlist.inputs().next().unwrap();
    let inverter = Gate::new_logical("INV".into(), vec!["I".into()], "O".into());
    let inverted = netlist
        .insert_gate(inverter, "inst_0".into(), std::slice::from_ref(&input))
        .unwrap();
    assert!(netlist.replace_net_uses(input.unwrap(), &inverted).is_ok());
    assert_verilog_eq!(
        netlist.to_string(),
        "module example (
           a,
           b,
           y
         );
           input a;
           wire a;
           input b;
           wire b;
           output y;
           wire y;
           wire inst_0_Y;
           wire inst_0_O;
           AND inst_0 (
             .A(inst_0_O),
             .B(b),
             .Y(inst_0_Y)
           );
           INV inst_0 (
             .I(inst_0_O),
             .O(inst_0_O)
           );
           assign y = inst_0_Y;
         endmodule\n"
    );
}

#[test]
fn test_replace2() {
    let netlist = get_simple_example();
    let input = netlist.inputs().next().unwrap();
    let inverter = Gate::new_logical("INV".into(), vec!["I".into()], "O".into());
    let inverted = netlist
        .insert_gate_disconnected(inverter, "inst_0".into())
        .unwrap();
    // This errors, because input is not safe to delete. No replace is done.
    assert!(
        netlist
            .replace_net_uses(input.clone().unwrap(), &inverted)
            .is_err()
    );
    inverted.find_input(&"I".into()).unwrap().connect(input);
    assert_verilog_eq!(
        netlist.to_string(),
        "module example (
           a,
           b,
           y
         );
           input a;
           wire a;
           input b;
           wire b;
           output y;
           wire y;
           wire inst_0_Y;
           wire inst_0_O;
           AND inst_0 (
             .A(a),
             .B(b),
             .Y(inst_0_Y)
           );
           INV inst_0 (
             .I(a),
             .O(inst_0_O)
           );
           assign y = inst_0_Y;
         endmodule\n"
    );
}
