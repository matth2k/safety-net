use circuit::circuit::Net;
use circuit::netlist::{GatePrimitive, NetRef, Netlist};
fn main() {
    let and_gate = GatePrimitive::new_logical(
        "AND".to_string(),
        vec!["A".to_string(), "B".to_string()],
        "Y".to_string(),
    );

    let netlist = Netlist::new("top".to_string());
    let input_net = Net::new_logic("input1".to_string());
    let input_net2 = Net::new_logic("input2".to_string());

    let input_net: NetRef = netlist.add_input(input_net);
    let input_net2: NetRef = netlist.add_input(input_net2);

    let output_net: NetRef =
        netlist.add_gate(and_gate, "and1".to_string(), &[input_net, input_net2]);

    let output_net: NetRef = netlist.add_as_output(output_net, Net::new_logic("out1".to_string()));

    output_net
        .borrow_mut()
        .as_net_mut()
        .set_name("woah".to_string());

    output_net
        .borrow()
        .get_operand(1)
        .borrow_mut()
        .as_net_mut()
        .set_name("hijacked".to_string());

    println!("{}", netlist);
}
