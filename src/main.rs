use circuit::netlist::{GatePrimitive, Netlist};
fn main() {
    let and_gate = GatePrimitive::new_logical(
        "AND".to_string(),
        vec!["A".to_string(), "B".to_string()],
        "Y".to_string(),
    );

    let netlist = Netlist::new("top".to_string());

    let input1 = netlist.add_input_logic("input1".to_string());
    let input2 = netlist.add_input_logic("input2".to_string());

    let output = netlist.add_gate(and_gate, "my_and_gate".to_string(), &[input1, input2]);

    output.expose_with_name("my_output".to_string());

    println!("{}", netlist);
    // output_net
    //     .borrow()
    //     .get_operand(1)
    //     .borrow_mut()
    //     .as_net_mut()
    //     .set_name("hijacked".to_string());

    output
        .get_instance_type_mut()
        .unwrap()
        .change_gate_name("AND_X1".to_string());

    println!("{}", netlist);
}
