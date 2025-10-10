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
    // This errors, because input is not safe to delete. No replace is done.
    assert!(
        netlist
            .replace_net_uses(input.clone(), &inverted.get_output(0))
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
// Testing edits for replace_net_uses using single and multiple output netrefs that have a
// DrivenNet that is fed as an argument
// TEST 1: SINGLE , SINGLE
// TEST2: SINGLE, MULTIPLE
// TEST3 MULTIPLE, SINGLE
// TEST4 MULTOPLE, MULTIPLE

#[test]
fn test_replace_single_single(){
  let netlist = Netlist::new("example".into());
  let a = netlist.insert_input("a".into());
  let b = netlist.insert_input("b".into());
  let and_inst = netlist.insert_gate(and_gate(), "and_0".into(), &[a.clone(), b]).unwrap();
  let and_out = and_inst.get_output(0); // DrivenNet from a net ref of single output
  // Ensure the netlist has a top-level output for verification
  let and_out = and_out.expose_with_name("y".into());
  // Drop the source node handle to satisfy the strong-count guard
  drop(and_inst);
  assert!(netlist.replace_net_uses(and_out, &a.clone()).is_ok());
  // TODO: CHECK actual output
  assert!(netlist.verify().is_ok());
  println!("{}", netlist.to_string());
}

#[test]
fn test_replace_single_single_v2(){
  let netlist = Netlist::new("example".into());
  let a = netlist.insert_input("a".into());
  let b = netlist.insert_input("b".into());
  let and_inst = netlist.insert_gate(and_gate(), "and_0".into(), &[a.clone(), b.clone()]).unwrap();
  let or_inst = netlist.insert_gate(or_gate(),"or_0".into(),  &[a.clone(),b.clone()]).unwrap();
  let and_out = and_inst.get_output(0);
  drop(and_inst);
  assert!(netlist.replace_net_uses(and_out, &or_inst.clone().into()).is_ok());
  or_inst.get_output(0).expose_with_name("y".into());
  assert!(netlist.verify().is_ok());
  println!("{}", netlist.to_string());
}
#[test]
fn test_replace_single_multiple(){
  let netlist = Netlist::new("example".into());
  let a = netlist.insert_input("a".into());
  let b = netlist.insert_input("b".into());

  let and_inst = netlist
  .insert_gate(and_gate(), "and_0".into(), &[a.clone(), b.clone()])
  .unwrap();

  let dup = netlist
    .insert_gate(two_out_gate(), "dup0".into(), &[a.clone()])
    .unwrap();

  dup.get_output(1).expose_with_name("y".into());

  let and_out = and_inst.get_output(0);
  let dup_out1 = dup.get_output(1);
  drop(dup);
  netlist.replace_net_uses(dup_out1, &and_out.clone()).unwrap();
  assert!(netlist.verify().is_ok());
  println!("{}", netlist);
}
#[test]
fn test_replace_multiple_single(){
  let netlist = Netlist::new("example".into());
  let a = netlist.insert_input("a".into());
  let b = netlist.insert_input("b".into());

  let and_inst = netlist
  .insert_gate(and_gate(), "and_0".into(), &[a.clone(), b.clone()])
  .unwrap();

  let dup = netlist
    .insert_gate(two_out_gate(), "dup0".into(), &[a.clone()])
    .unwrap();

  and_inst.get_output(0).expose_with_name("y".into());

  let and_out = and_inst.get_output(0);
  let dup_out0 = dup.get_output(0);
  drop(and_inst);
  netlist.replace_net_uses(and_out, &dup_out0.clone()).unwrap();
  assert!(netlist.verify().is_ok());
  println!("{}", netlist);

}

#[test]
fn test_replace_multiple_multiple(){
  let netlist = Netlist::new("example".into());
  let a = netlist.insert_input("a".into());

  let dup1 = netlist
      .insert_gate(two_out_gate(), "dup1".into(), &[a.clone()])
      .unwrap();

  let dup2 = netlist
    .insert_gate(two_out_gate(), "dup2".into(), &[a.clone()])
    .unwrap();

  let dup1_out0 = dup1.get_output(0);
  dup1_out0.clone().expose_with_name("y".into());
  let dup2_out1 = dup2.get_output(1);
  
  drop(dup2);
  netlist.replace_net_uses(dup2_out1, &dup1_out0.clone()).unwrap();
  assert!(netlist.verify().is_ok());
  println!("{}", netlist);
  
}

