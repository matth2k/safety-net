#[cfg(feature = "graph")]
use safety_net::graph::MultiDiGraph;
use safety_net::{DrivenNet, Gate, Identifier, Netlist, format_id};

#[allow(dead_code)]
fn full_adder() -> Gate {
    Gate::new_logical_multi(
        "FA".into(),
        vec!["CIN".into(), "A".into(), "B".into()],
        vec!["S".into(), "COUT".into()],
    )
}

#[allow(dead_code)]
fn ripple_adder() -> Netlist<Gate> {
    let netlist = Netlist::new("ripple_adder".to_string());
    let bitwidth = 4;

    // Add the the inputs
    let a_vec = netlist.insert_input_logic_bus("a".to_string(), bitwidth);
    let b_vec = netlist.insert_input_logic_bus("b".to_string(), bitwidth);
    let s_vec = Identifier::new_bus("s".to_string(), bitwidth);
    let mut carry: DrivenNet<Gate> = netlist.insert_input("cin".into());

    for i in 0..bitwidth {
        // Instantiate a full adder for each bit
        let fa = netlist.insert_gate_disconnected(full_adder(), format_id!("fa_{i}"));

        // Connect A_i and B_i
        fa.get_input(1).connect(a_vec[i].clone());
        fa.get_input(2).connect(b_vec[i].clone());

        // Connect with the prev carry
        carry.connect(fa.get_input(0));

        // Expose the sum
        fa.get_output(0).expose_with_name(s_vec[i].clone());

        carry = fa.get_output(1);

        if i == bitwidth - 1 {
            // Last full adder, expose the carry out
            fa.get_output(1).expose_with_name("cout".into());
        }
    }

    netlist.reclaim().unwrap()
}

fn main() {
    #[cfg(feature = "graph")]
    {
        let netlist = ripple_adder();
        eprintln!("{netlist}");
        let analysis = netlist.get_analysis::<MultiDiGraph<_>>().unwrap();
        let graph = analysis.get_graph();
        let dot = petgraph::dot::Dot::with_config(graph, &[]);
        println!("{dot}");
    }
}
