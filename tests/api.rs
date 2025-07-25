use safety_net::assert_verilog_eq;
use safety_net::circuit::Net;
use safety_net::format_id;
use safety_net::netlist::DrivenNet;
use safety_net::netlist::Gate;
use safety_net::netlist::GateNetlist;
use safety_net::netlist::Netlist;
use std::rc::Rc;

fn and_gate() -> Gate {
    Gate::new_logical("AND".into(), vec!["A".into(), "B".into()], "Y".into())
}

fn full_adder() -> Gate {
    Gate::new_logical_multi(
        "FA".into(),
        vec!["CIN".into(), "A".into(), "B".into()],
        vec!["S".into(), "COUT".into()],
    )
}

fn ripple_adder() -> GateNetlist {
    let netlist = Netlist::new("ripple_adder".to_string());
    let bitwidth = 4;

    // Add the the inputs
    let a = netlist.insert_input_escaped_logic_bus("a".to_string(), bitwidth);
    let b = netlist.insert_input_escaped_logic_bus("b".to_string(), bitwidth);
    let mut carry: DrivenNet<Gate> = netlist.insert_input("cin".into());

    for (i, (a, b)) in a.into_iter().zip(b.into_iter()).enumerate() {
        // Instantiate a full adder for each bit
        let fa = netlist
            .insert_gate(full_adder(), format_id!("fa_{i}"), &[carry, a, b])
            .unwrap();

        // Expose the sum
        fa.expose_net(&fa.get_net(0)).unwrap();

        carry = fa.find_output(&"COUT".into()).unwrap();

        if i == bitwidth - 1 {
            // Last full adder, expose the carry out
            fa.get_output(1).expose_with_name("cout".into()).unwrap();
        }
    }

    netlist.reclaim().unwrap()
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

#[test]
#[should_panic(expected = "Attempt to grab the net of a multi-output instance")]
fn test_bad_access_1() {
    let netlist = ripple_adder();
    let last_fa = netlist.last().unwrap();
    // Can't use 'as' methods on multi-output
    last_fa.as_net();
}

#[test]
#[should_panic(expected = "Attempt to grab the net of a multi-output instance")]
fn test_bad_access_2() {
    let netlist = ripple_adder();
    let last_fa = netlist.last().unwrap();
    // Can't use 'as' methods on multi-output
    last_fa.as_net_mut();
}

#[test]
#[should_panic(expected = "Attempted to grab output port of a multi-output gate")]
fn test_bad_access_3() {
    let netlist = ripple_adder();
    let last_fa = netlist.last().unwrap();
    // Can't use 'as' methods on multi-output
    last_fa
        .get_instance_type()
        .unwrap()
        .get_single_output_port();
}

#[test]
#[should_panic(expected = "Input port is unlinked from netlist")]
fn test_unlinked_1() {
    let netlist = ripple_adder();
    let last_fa = netlist.last().unwrap();
    last_fa.get_input(0).get_driver();
}

#[test]
#[should_panic(expected = "NetRef is unlinked from netlist")]
fn test_unlinked_2() {
    let netlist = ripple_adder();
    let last_fa = netlist.last().unwrap();
    last_fa.expose_with_name("no".into());
}

#[test]
fn test_get_net_from_obj() {
    let netlist = get_simple_example();
    let gate = netlist.last().unwrap();
    let obj = gate.get_obj();
    let net = gate.get_net(0).clone();
    let also_net = obj.get_net(0).clone();
    let still_net = obj.get_single_net().clone();
    assert_eq!(net, also_net);
    assert_eq!(net, still_net);
}

#[test]
#[should_panic(expected = "already mutably borrowed: BorrowError")]
fn test_change_gate_incorrect() {
    let netlist = get_simple_example();
    let gate = netlist.last().unwrap();
    let mut type_gate = gate.get_instance_type_mut().unwrap();
    type_gate.set_gate_name("OR".into());
    // This borrow needs to end
    eprintln!("{netlist}");
}

#[test]
fn test_change_gate_correct() {
    let netlist = get_simple_example();
    let gate = netlist.last().unwrap();
    {
        let mut type_gate = gate.get_instance_type_mut().unwrap();
        type_gate.set_gate_name("OR".into());
    }
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
           OR inst_0 (
             .A(a),
             .B(b),
             .Y(inst_0_Y)
           );
           assign y = inst_0_Y;
         endmodule\n"
    );
}
