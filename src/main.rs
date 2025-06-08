use circuit::netlist::{GatePrimitive, Netlist};
fn main() {
    let and_gate = GatePrimitive::new_logical(
        "AND".to_string(),
        vec!["A".to_string(), "B".to_string()],
        "Y".to_string(),
    );

    let full_adder = GatePrimitive::new_logical_multi(
        "FA".to_string(),
        vec!["CIN".to_string(), "A".to_string(), "B".to_string()],
        vec!["COUT".to_string(), "S".to_string()],
    );

    let netlist = Netlist::new("top".to_string());

    let carry_in = netlist.add_input_logic("c0".to_string());
    let input1 = netlist.add_input_logic("a".to_string());
    let input2 = netlist.add_input_logic("b".to_string());

    let fa = netlist
        .add_gate(full_adder, "my_fa".to_string(), &[carry_in, input1, input2])
        .unwrap();

    // lets and the sum and cout together

    let anded = netlist
        .add_gate(and_gate, "my_and".to_string(), &[fa.clone(), fa])
        .unwrap();

    anded.expose_as_output().unwrap();

    println!("{}", netlist);
}
