use safety_net::format_id;
#[cfg(feature = "graph")]
use safety_net::graph::MultiDiGraph;
use safety_net::{DrivenNet, Gate, Netlist};

#[allow(dead_code)]
fn and() -> Gate {
    Gate::new_logical("AND".into(), vec!["A".into(), "B".into()], "Y".into())
}

#[allow(dead_code)]
fn nor() -> Gate {
    Gate::new_logical("NOR".into(), vec!["A".into(), "B".into()], "Y".into())
}

#[allow(dead_code)]
fn inv() -> Gate {
    Gate::new_logical("INV".into(), vec!["A".into()], "Y".into())
}

#[allow(dead_code)]
fn circuit() -> Netlist<Gate> {
    let netlist = Netlist::new("circuit".to_string());

    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());
    let and = netlist.insert_gate(and(), "and_0".into(), &[a, b]).unwrap();
    let c = netlist.insert_input("c".into());
    let nor = netlist
        .insert_gate(nor(), "nor_0".into(), &[and.into(), c])
        .unwrap();

    nor.expose_as_output().unwrap();

    netlist.reclaim().unwrap()
}

fn main() {
    #[cfg(feature = "graph")]
    {
        let netlist = circuit();
        println!("{}", netlist.dot_string().unwrap());
    }
}
