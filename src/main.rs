use circuit::netlist::{GatePrimitive, Netlist};

fn and_gate() -> GatePrimitive {
    GatePrimitive::new_logical(
        "AND".to_string(),
        vec!["A".to_string(), "B".to_string()],
        "Y".to_string(),
    )
}

fn simple_example() -> Netlist {
    let netlist = Netlist::new("simple_example".to_string());

    // Add the the two inputs
    let input1 = netlist.insert_input_logic("input1".to_string());
    let input2 = netlist.insert_input_logic("input2".to_string());

    // Instantiate an AND gate

    let instance = netlist
        .insert_gate(
            and_gate(),
            "my_and".to_string(),
            &[input1.into(), input2.into()],
        )
        .unwrap();

    // Make this AND gate an output
    instance.expose_as_output().unwrap();

    netlist.reclaim().unwrap()
}

fn main() {
    let netlist = simple_example();
    print!("{}", netlist);
}
