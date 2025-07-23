use circuit::{
    attribute::Parameter,
    circuit::{Identifier, Instantiable, Net},
    filter_nodes,
    netlist::Netlist,
};

#[derive(Debug, Clone)]
enum Gate {
    And(Identifier, Vec<Net>, Net),
}

impl Instantiable for Gate {
    fn get_name(&self) -> &Identifier {
        match self {
            Gate::And(id, _, _) => id,
        }
    }

    fn get_input_ports(&self) -> impl IntoIterator<Item = &Net> {
        match self {
            Gate::And(_, inputs, _) => inputs,
        }
    }

    fn get_output_ports(&self) -> impl IntoIterator<Item = &Net> {
        match self {
            Gate::And(_, _, output) => std::slice::from_ref(output),
        }
    }

    fn has_parameter(&self, _id: &Identifier) -> bool {
        false
    }

    fn get_parameter(&self, _id: &Identifier) -> Option<Parameter> {
        None
    }

    fn parameters(&self) -> impl Iterator<Item = (Identifier, Parameter)> {
        std::iter::empty()
    }
}

fn and_gate() -> Gate {
    Gate::And(
        "AND".into(),
        vec![Net::new_logic("A".into()), Net::new_logic("B".into())],
        Net::new_logic("Y".into()),
    )
}

fn main() {
    let netlist = Netlist::new("example".to_string());

    // Add the the two inputs
    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());

    // Instantiate an AND gate
    let instance = netlist
        .insert_gate(and_gate(), "inst_0".into(), &[a, b])
        .unwrap();

    // Make this AND gate an output
    instance.expose_with_name("y".into());

    // Print the netlist
    println!("{netlist}");
    for node in filter_nodes!(netlist, Gate::And(_, _, _)) {
        println!("Found AND gate: {node}");
    }
}
