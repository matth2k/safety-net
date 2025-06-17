use circuit::{
    circuit::{Identifier, Instantiable, Net},
    filter_nodes,
    netlist::Netlist,
};

#[derive(Debug, Clone)]
enum Gate {
    And(Identifier, Vec<Net>, Net),
}

impl std::fmt::Display for Gate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Instantiable for Gate {
    fn get_name(&self) -> &Identifier {
        match self {
            Gate::And(id, _, _) => &id,
        }
    }

    fn get_input_ports(&self) -> &[Net] {
        match self {
            Gate::And(_, inputs, _) => inputs,
        }
    }

    fn get_output_ports(&self) -> &[Net] {
        match self {
            Gate::And(_, _, output) => std::slice::from_ref(output),
        }
    }
}

fn and_gate() -> Gate {
    Gate::And(
        "AND".into(),
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
    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());

    // Instantiate an AND gate
    let instance = netlist
        .insert_gate(and_gate(), "inst_0".into(), &[a.into(), b.into()])
        .unwrap();

    // Make this AND gate an output
    instance.expose_with_name("y".to_string());

    // Print the netlist
    println!("{}", netlist);
    for node in filter_nodes!(netlist, Gate::And(_, _, _)) {
        println!("Found AND gate: {}", node);
    }
}
