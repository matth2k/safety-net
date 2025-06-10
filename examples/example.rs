use circuit::netlist::{Gate, Netlist};

fn and_gate() -> Gate {
    Gate::new_logical(
        "AND".to_string(),
        vec!["A".to_string(), "B".to_string()],
        "Y".to_string(),
    )
}

fn main() {
    let netlist = Netlist::new("example".to_string());

    // Add the the two inputs
    let a = netlist.insert_input_logic("a".to_string());
    let b = netlist.insert_input_logic("b".to_string());

    // Instantiate an AND gate
    let instance = netlist
        .insert_gate(and_gate(), "inst_0".to_string(), &[a.into(), b.into()])
        .unwrap();

    // Make this AND gate an output
    instance.expose_with_name("y".to_string());

    // Print the netlist
    println!("{}", netlist);
}
