use safety_net::Gate;
use safety_net::GateNetlist;
use safety_net::Netlist;
use safety_net::assert_verilog_eq;
use safety_net::format_id;
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
    {
        let _new_cell = netlist
            .insert_gate(and_gate(), "inst_1".into(), &inputs)
            .unwrap();
        assert!(netlist.verify().is_ok());
        assert_eq!(netlist.objects().count(), 4);
        assert!(netlist.clean().is_err());
    }
    assert!(netlist.clean().unwrap());
    assert_eq!(netlist.objects().count(), 3);
    assert!(!netlist.clean().unwrap());
}

#[test]
fn test_multiple_output_aliases() {
    let netlist = GateNetlist::new("passthru_example".to_string());

    // Add the input
    let b = netlist.insert_input("b".into());

    // Instantiate an AND gate
    let instance = netlist
        .insert_gate(
            Gate::new_logical("AND".into(), vec!["A".into(), "B".into()], "Y".into()),
            "inst_0".into(),
            &[b.clone(), b.clone()],
        )
        .unwrap();

    // Create multiple output aliases for the same net
    instance.clone().expose_with_name("y".into());
    instance.clone().expose_with_name("z".into());

    // Verify that we have two outputs
    let outputs = netlist.get_output_ports();
    assert_eq!(
        outputs.len(),
        2,
        "Expected 2 output aliases but got {}",
        outputs.len()
    );

    // Verify that the outputs have the correct names
    let output_names: Vec<&str> = outputs
        .iter()
        .map(|net| net.get_identifier().get_name())
        .collect();
    assert!(
        output_names.contains(&"y"),
        "Expected output 'y' but got {:?}",
        output_names
    );
    assert!(
        output_names.contains(&"z"),
        "Expected output 'z' but got {:?}",
        output_names
    );
}

#[test]
fn test_remove_output() {
    let netlist = GateNetlist::new("remove_output_test".to_string());

    // Add the input
    let b = netlist.insert_input("b".into());

    // Instantiate an AND gate
    let instance = netlist
        .insert_gate(
            Gate::new_logical("AND".into(), vec!["A".into(), "B".into()], "Y".into()),
            "inst_0".into(),
            &[b.clone(), b.clone()],
        )
        .unwrap();

    // Create multiple output aliases
    instance.clone().expose_with_name("y".into());
    instance.clone().expose_with_name("z".into());
    instance.clone().expose_with_name("w".into());

    // Verify we have three outputs
    let outputs = netlist.get_output_ports();
    assert_eq!(outputs.len(), 3);

    // Remove one output alias using NetRef method
    let removed = instance.remove_output(&"z".into());
    assert!(removed, "Should have successfully removed 'z'");

    // Verify we now have two outputs
    let outputs = netlist.get_output_ports();
    assert_eq!(
        outputs.len(),
        2,
        "Expected 2 outputs after removal but got {}",
        outputs.len()
    );

    // Verify the correct outputs remain
    let output_names: Vec<&str> = outputs
        .iter()
        .map(|net| net.get_identifier().get_name())
        .collect();
    assert!(output_names.contains(&"y"));
    assert!(!output_names.contains(&"z"));
    assert!(output_names.contains(&"w"));

    // Try to remove a non-existent output
    let removed = instance.remove_output(&"nonexistent".into());
    assert!(
        !removed,
        "Should return false when removing non-existent output"
    );

    // Verify output count unchanged
    let outputs = netlist.get_output_ports();
    assert_eq!(outputs.len(), 2);
}

#[test]
fn test_remove_all_outputs() {
    let netlist = GateNetlist::new("remove_all_outputs_test".to_string());

    // Add the input
    let b = netlist.insert_input("b".into());

    // Instantiate an AND gate
    let instance = netlist
        .insert_gate(
            Gate::new_logical("AND".into(), vec!["A".into(), "B".into()], "Y".into()),
            "inst_0".into(),
            &[b.clone(), b.clone()],
        )
        .unwrap();

    // Create multiple output aliases
    instance.clone().expose_with_name("y".into());
    instance.clone().expose_with_name("z".into());
    instance.clone().expose_with_name("w".into());

    // Verify we have three outputs
    let outputs = netlist.get_output_ports();
    assert_eq!(outputs.len(), 3);

    // Remove all outputs using NetRef method
    let count = instance.remove_all_outputs();
    assert_eq!(
        count, 3,
        "Should have removed 3 outputs but removed {}",
        count
    );

    // Verify we have no outputs
    let outputs = netlist.get_output_ports();
    assert_eq!(
        outputs.len(),
        0,
        "Expected 0 outputs but got {}",
        outputs.len()
    );

    // Verify calling remove_all_outputs again returns 0
    let count = instance.remove_all_outputs();
    assert_eq!(count, 0, "Should return 0 when removing from empty outputs");
}

#[test]
fn test_driven_net_remove_output() {
    let netlist = GateNetlist::new("driven_net_remove_test".to_string());

    // Add inputs
    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());

    // Instantiate an AND gate
    let instance = netlist
        .insert_gate(
            Gate::new_logical("AND".into(), vec!["A".into(), "B".into()], "Y".into()),
            "inst_0".into(),
            &[a, b],
        )
        .unwrap();

    // Get the driven net output
    let driven_net = instance.get_output(0);

    // Create multiple output aliases via DrivenNet
    driven_net.clone().expose_with_name("y".into());
    driven_net.clone().expose_with_name("z".into());
    driven_net.clone().expose_with_name("w".into());

    // Verify we have three outputs
    let outputs = netlist.get_output_ports();
    assert_eq!(outputs.len(), 3);

    // Remove one output using DrivenNet method
    let removed = driven_net.remove_output(&"y".into());
    assert!(removed, "Should have successfully removed 'y'");

    // Verify we now have two outputs
    let outputs = netlist.get_output_ports();
    assert_eq!(outputs.len(), 2);

    // Remove all remaining outputs using DrivenNet method
    let count = driven_net.remove_all_outputs();
    assert_eq!(count, 2, "Should have removed 2 remaining outputs");

    // Verify we have no outputs
    let outputs = netlist.get_output_ports();
    assert_eq!(outputs.len(), 0);
}

#[test]
fn test_netlist_remove_output_by_operand() {
    let netlist = GateNetlist::new("netlist_remove_test".to_string());

    // Add inputs
    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());

    // Instantiate an AND gate
    let instance = netlist
        .insert_gate(
            Gate::new_logical("AND".into(), vec!["A".into(), "B".into()], "Y".into()),
            "inst_0".into(),
            &[a, b],
        )
        .unwrap();

    // Create multiple output aliases
    instance.clone().expose_with_name("y".into());
    instance.clone().expose_with_name("z".into());

    // Verify we have two outputs
    let outputs = netlist.get_output_ports();
    assert_eq!(outputs.len(), 2);

    // Remove one output directly from netlist
    let removed = netlist.remove_output(&instance.clone().into(), &"y".into());
    assert!(removed, "Should have successfully removed 'y'");

    // Verify we have one output left
    let outputs = netlist.get_output_ports();
    assert_eq!(outputs.len(), 1);

    // Remove all outputs using netlist method
    let count = netlist.remove_outputs(&instance.into());
    assert_eq!(count, 1, "Should have removed 1 remaining output");

    // Verify we have no outputs
    let outputs = netlist.get_output_ports();
    assert_eq!(outputs.len(), 0);
}

#[test]
fn test_netlist_clear_outputs() {
    let netlist = GateNetlist::new("clear_outputs_test".to_string());

    // Add inputs
    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());

    // Instantiate gates
    let instance1 = netlist
        .insert_gate(
            Gate::new_logical("AND".into(), vec!["A".into(), "B".into()], "Y".into()),
            "inst_0".into(),
            &[a.clone(), b.clone()],
        )
        .unwrap();

    let instance2 = netlist
        .insert_gate(
            Gate::new_logical("OR".into(), vec!["A".into(), "B".into()], "Y".into()),
            "inst_1".into(),
            &[a.clone(), b.clone()],
        )
        .unwrap();

    // Create multiple output aliases on both gates
    instance1.clone().expose_with_name("y1".into());
    instance1.clone().expose_with_name("y2".into());
    instance2.clone().expose_with_name("z1".into());
    instance2.clone().expose_with_name("z2".into());

    // Verify we have four outputs
    let outputs = netlist.get_output_ports();
    assert_eq!(outputs.len(), 4);

    // Clear all outputs
    netlist.clear_outputs();

    // Verify we have no outputs
    let outputs = netlist.get_output_ports();
    assert_eq!(outputs.len(), 0, "Expected 0 outputs after clear");
}

#[test]
fn test_replace_self() {
    let netlist = get_simple_example();
    let gate = netlist.last().unwrap();
    let output = gate.get_output(0);
    let result = netlist.replace_net_uses(output.clone(), &output);
    assert!(result.is_ok());
    let result = result.unwrap().get_instance_type().cloned();
    assert!(result.is_some());
    let gtype = result.unwrap();
    assert_eq!(gtype.get_gate_name(), &"AND".into());
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

#[test]
fn test_rename() {
    let netlist = get_simple_example();
    assert!(netlist.rename_nets(|i| format_id!("__{i}__")).is_ok());
    let gate = netlist.last().unwrap();
    assert_eq!(gate.get_instance_name().unwrap(), "__1__".into());
}
