use circuit::circuit::Net;
use circuit::netlist::{GatePrimitive, Netlist};
fn main() {
    let and_gate = GatePrimitive::new_logical(
        "AND".to_string(),
        vec!["A".to_string(), "B".to_string()],
        "Y".to_string(),
    );
    let netlist = Netlist::new("top".to_string());
    let input_net = Net::new_logic("input1".to_string());
    let input_net2 = Net::new_logic("input2".to_string());

    let input_net = netlist.add_input(input_net);
    let input_net2 = netlist.add_input(input_net2);

    let output_net = netlist.add_gate(and_gate, "and1".to_string(), &[input_net, input_net2]);

    netlist.add_as_output(output_net, Net::new_logic("out1".to_string()));

    println!("{}", netlist);
}
