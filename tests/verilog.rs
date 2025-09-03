use safety_net::{
    assert_verilog_eq, logic,
    netlist::{Gate, GateNetlist, Netlist},
};
use std::rc::Rc;

fn and_gate() -> Gate {
    Gate::new_logical("AND".into(), vec!["A".into(), "B".into()], "Y".into())
}

fn get_simple_example() -> GateNetlist {
    let netlist = Netlist::new("example".to_string());

    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());

    let instance = netlist
        .insert_gate(and_gate(), "inst_0".into(), &[a, b])
        .unwrap();

    instance.expose_with_name("y".into());

    netlist.reclaim().unwrap()
}

#[test]
fn min_module() {
    let netlist = GateNetlist::new("min_module".to_string());
    let a = netlist.insert_input("a".into());
    a.expose_with_name("y".into());
    assert!(netlist.verify().is_ok());
    assert_verilog_eq!(
        netlist.to_string(),
        "module min_module (
           a,
           y
         );
           input a;
           wire a;
           output y;
           wire y;
           assign y = a;
         endmodule\n"
    );
}

#[test]
fn test_netlist_first() {
    let netlist = GateNetlist::new("min_module".to_string());
    let a = netlist.insert_input("a".into());
    a.clone().expose_with_name("y".into());
    let a_too = netlist.last().unwrap();
    let also_a = netlist.first().unwrap();
    assert_eq!(a_too, also_a);
    assert_eq!(a.unwrap(), also_a);
}

#[test]
fn test_netlist_find() {
    let netlist = GateNetlist::new("min_module".to_string());
    let a = netlist.insert_input("a".into());
    a.expose_with_name("y".into());
    assert!(netlist.find_net(&"a".into()).is_some());
    assert!(netlist.find_net(&"b".into()).is_none());
}

#[test]
fn simple_gate_module() {
    let netlist = get_simple_example();
    assert!(netlist.verify().is_ok());
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
           AND inst_0 (
             .A(a),
             .B(b),
             .Y(inst_0_Y)
           );
           assign y = inst_0_Y;
         endmodule\n"
    );
}

#[test]
fn dont_touch_gate() {
    let netlist = get_simple_example();
    assert!(netlist.verify().is_ok());
    netlist
        .last()
        .unwrap()
        .set_attribute("dont_touch".to_string());
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
           (* dont_touch *)
           AND inst_0 (
             .A(a),
             .B(b),
             .Y(inst_0_Y)
           );
           assign y = inst_0_Y;
         endmodule\n"
    );
}

#[test]
fn simple_gate_attribute() {
    let netlist = get_simple_example();
    assert!(netlist.verify().is_ok());
    let gate = netlist.last().unwrap();
    gate.insert_attribute("dont_touch".to_string(), "true".to_string());
    gate.clear_attribute(&"dont_touch".to_string());
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
           AND inst_0 (
             .A(a),
             .B(b),
             .Y(inst_0_Y)
           );
           assign y = inst_0_Y;
         endmodule\n"
    );
}

#[test]
fn constant_output() {
    let netlist: Rc<GateNetlist> = Netlist::new("top".to_string());
    let vdd = netlist.insert_constant(logic::Logic::True, "unemitted".into());
    assert!(vdd.is_ok());
    let vdd = vdd.unwrap();
    vdd.expose_with_name("y".into());
    assert_verilog_eq!(
        netlist.to_string(),
        "module top (
           y
         );
           output y;
           wire y;
           assign y = 1'b1;
         endmodule\n"
    );
}

#[test]
fn constant_driver() {
    let netlist: Rc<GateNetlist> = Netlist::new("top".to_string());
    let vdd = netlist.insert_constant(logic::Logic::True, "unemitted".into());
    assert!(vdd.is_ok());
    let vdd = vdd.unwrap();
    let not_gate = Gate::new_logical("NOT".into(), vec!["A".into()], "Y".into());
    let gnd = netlist.insert_gate(not_gate, "inst_0".into(), &[vdd]);
    assert!(gnd.is_ok());
    let gnd = gnd.unwrap();
    gnd.expose_with_name("y".into());
    assert_verilog_eq!(
        netlist.to_string(),
        "module top (
           y
         );
           output y;
           wire y;
           wire inst_0_Y;
           NOT inst_0 (
             .A(1'b1),
             .Y(inst_0_Y)
           );
           assign y = inst_0_Y;\n"
    );
}
