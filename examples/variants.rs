use circuit::{
    circuit::{Instantiable, Net},
    filter_nodes,
    netlist::Netlist,
};

#[derive(Debug, Clone)]
enum Gate {
    And(Vec<Net>, Net),
}

impl std::fmt::Display for Gate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Instantiable for Gate {
    fn get_name(&self) -> &str {
        match self {
            Gate::And(_, _) => "AND",
        }
    }

    fn get_input_ports(&self) -> &[Net] {
        match self {
            Gate::And(inputs, _) => inputs,
        }
    }

    fn get_output_ports(&self) -> &[Net] {
        match self {
            Gate::And(_, output) => std::slice::from_ref(output),
        }
    }
}

fn and_gate() -> Gate {
    Gate::And(
        vec![
            Net::new_logic("A".to_string()),
            Net::new_logic("B".to_string()),
        ],
        Net::new_logic("Y".to_string()),
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
    for node in filter_nodes!(netlist, Gate::And(_, _)) {
        println!("Found AND gate: {}", node);
    }
}
