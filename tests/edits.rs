use safety_net::assert_verilog_eq;
use safety_net::netlist::Gate;
use safety_net::netlist::GateNetlist;
use safety_net::netlist::Netlist;
use std::rc::Rc;

fn and_gate() -> Gate {
    Gate::new_logical("AND".into(), vec!["A".into(), "B".into()], "Y".into())
}

fn or_gate() -> Gate {
    Gate::new_logical("OR".into(), vec!["A".into(), "B".into()], "Y".into())
}

fn two_out_gate() -> Gate {
    Gate::new_logical_multi(
        "DUP".into(),
        vec!["I".into()],
        vec!["O0".into(), "O1".into()],
    )
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
    assert!(netlist.replace_net_uses(input, &inverted.into()).is_ok());
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
    let inverted = netlist.insert_gate_disconnected(inverter, "inst_0".into());
    assert!(
        netlist
            .replace_net_uses(input.clone(), &inverted.clone().into())
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

#[test]
fn test_replace_single_single() {
    let netlist = Netlist::new("example".into());
    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());
    let and_inst = netlist
        .insert_gate(and_gate(), "and_0".into(), &[a.clone(), b.clone()])
        .unwrap();
    let and_out = and_inst.get_output(0);
    let or_inst = netlist
        .insert_gate(or_gate(), "or_0".into(), &[a.clone(), and_out.clone()])
        .unwrap();
    drop(and_inst);
    assert!(
        netlist
            .replace_net_uses(and_out, &or_inst.clone().into())
            .is_ok()
    );
    or_inst.get_output(0).expose_with_name("y".into());
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
            wire and_0_Y;
            wire or_0_Y;
            AND and_0 (
              .A(a),
              .B(b),
              .Y(and_0_Y)
            );
            OR or_0 (
              .A(a),
              .B(or_0_Y),
              .Y(or_0_Y)
            );
            assign y = or_0_Y;
          endmodule"
    );
}
#[test]
fn test_replace_single_multiple() {
    let netlist = Netlist::new("example".into());
    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());

    let and_inst = netlist
        .insert_gate(and_gate(), "and_0".into(), &[a.clone(), b.clone()])
        .unwrap();

    let dup = netlist
        .insert_gate(two_out_gate(), "dup0".into(), &[a])
        .unwrap();

    dup.get_output(1).expose_with_name("y".into());

    let and_out = and_inst.get_output(0);
    let dup_out1 = dup.get_output(1);
    drop(dup);
    netlist.replace_net_uses(dup_out1, &and_out).unwrap();
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
          wire and_0_Y;
          wire dup0_O0;
          wire dup0_O1;
          AND and_0 (
            .A(a),
            .B(b),
            .Y(and_0_Y)
          );
          DUP dup0 (
            .I(a),
            .O0(dup0_O0),
            .O1(dup0_O1)
          );
          assign y = and_0_Y;
        endmodule"
    );
}

#[test]
fn test_replace_multiple_single() {
    let netlist = Netlist::new("example".into());
    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());

    let and_inst = netlist
        .insert_gate(and_gate(), "and_0".into(), &[a.clone(), b.clone()])
        .unwrap();

    let dup = netlist
        .insert_gate(two_out_gate(), "dup0".into(), &[a])
        .unwrap();

    and_inst.get_output(0).expose_with_name("y".into());

    let and_out = and_inst.get_output(0);
    let dup_out0 = dup.get_output(0);
    drop(and_inst);
    netlist.replace_net_uses(and_out, &dup_out0).unwrap();
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
          wire and_0_Y;
          wire dup0_O0;
          wire dup0_O1;
          AND and_0 (
            .A(a),
            .B(b),
            .Y(and_0_Y)
          );
          DUP dup0 (
            .I(a),
            .O0(dup0_O0),
            .O1(dup0_O1)
          );
          assign y = dup0_O0;
        endmodule"
    );
}

#[test]
fn test_replace_multiple_multiple() {
    let netlist = Netlist::new("example".into());
    let a = netlist.insert_input("a".into());

    let dup2 = netlist
        .insert_gate(two_out_gate(), "dup2".into(), &[a])
        .unwrap();

    dup2.get_output(1).expose_with_name("y".into());
    let dup2_out0 = dup2.get_output(0);
    let dup2_out1 = dup2.get_output(1);

    drop(dup2);
    netlist.replace_net_uses(dup2_out1, &dup2_out0).unwrap();
    assert!(netlist.verify().is_ok());
    assert_verilog_eq!(
        netlist.to_string(),
        "module example (
            a,
            y
            );
            input a;
            wire a;
            output y;
            wire y;
            wire dup2_O0;
            wire dup2_O1;
            DUP dup2 (
              .I(a),
              .O0(dup2_O0),
              .O1(dup2_O1)
            );
            assign y = dup2_O0;
          endmodule"
    );
}
