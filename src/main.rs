use circuit::circuit::Net;
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

    let output_net = netlist.add_as_output(output, Net::new_logic("out1".to_string()));

    output_net.as_net_mut().set_name("woah".to_string());

    // output_net
    //     .borrow()
    //     .get_operand(1)
    //     .borrow_mut()
    //     .as_net_mut()
    //     .set_name("hijacked".to_string());

    output_net
        .get_instance_type_mut()
        .unwrap()
        .change_gate_name("AND_X1".to_string());

    println!("{}", netlist);
}
