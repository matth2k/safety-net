use std::rc::Rc;

use circuit::netlist::{DrivenNet, Gate, Netlist};

#[allow(dead_code)]
fn and_gate() -> Gate {
    Gate::new_logical(
        "AND".into(),
        vec!["A".to_string(), "B".to_string()],
        "Y".to_string(),
    )
}

fn full_adder() -> Gate {
    Gate::new_logical_multi(
        "FA".into(),
        vec!["CIN".to_string(), "A".to_string(), "B".to_string()],
        vec!["S".to_string(), "COUT".to_string()],
    )
}

#[allow(dead_code)]
fn simple_example() -> Netlist<Gate> {
    let netlist = Netlist::new("simple_example".to_string());

    // Add the the two inputs
    let input1 = netlist.insert_input("input1".into());
    let input2 = netlist.insert_input("input2".into());

    // Instantiate an AND gate
    let instance = netlist
        .insert_gate(
            and_gate(),
            "my_and".into(),
            &[input1.clone().into(), input2.clone().into()],
        )
        .unwrap();

    // Make this AND gate an output
    let instance = instance.expose_as_output().unwrap();

    // This line won't change anything because it's clever
    if let Some(mut n) = instance
        .req_driver_net(0)
        .unwrap()
        .borrow_mut_if(|n| !n.is_an_input())
    {
        n.set_identifier("renaming_for_fun".into());
    }

    netlist.reclaim().unwrap()
}

fn harder_example() -> Rc<Netlist<Gate>> {
    let netlist = Netlist::new("harder_example".to_string());
    let bitwidth = 9;

    // Add the the inputs
    let a_vec = netlist.insert_input_escaped_logic_bus("a".to_string(), bitwidth);
    let b_vec = netlist.insert_input_escaped_logic_bus("b".to_string(), bitwidth);
    let cin = netlist.insert_input("cin".into());

    // Instantiate the full adders
    let mut input_bus: [DrivenNet<Gate>; 3] =
        [cin.into(), a_vec[0].clone().into(), b_vec[0].clone().into()];

    for i in 0..bitwidth {
        let instance = netlist
            .insert_gate(full_adder(), format!("fa_{}", i).into(), &input_bus)
            .unwrap();

        if i == bitwidth - 1 {
            // Last full adder, expose the carry out
            instance.nets_mut().enumerate().for_each(|(j, mut n)| {
                if j == 1 {
                    n.set_identifier("cout".into());
                }
            });
            instance.expose_net(&instance.get_net(0)).unwrap();
            instance.expose_net(&instance.get_net(1)).unwrap();
            // instance.delete_uses().unwrap();
        } else {
            instance.expose_net(&instance.get_net(0)).unwrap();
            input_bus = [
                instance.get_output(1),
                a_vec[i + 1].clone().into(),
                b_vec[i + 1].clone().into(),
            ];
        }
    }

    netlist.clean().unwrap();
    // netlist.reclaim().unwrap()
    netlist
}

fn main() {
    let netlist = harder_example();
    print!("{}", netlist);

    let logic_levels = netlist
        .get_analysis::<circuit::graph::SimpleCombDepth<_>>()
        .unwrap();
    println!("Logic levels: {}", logic_levels.get_max_depth());
    for n in netlist.objects() {
        for n in n.nets() {
            println!(
                "{}: {}",
                n.get_identifier(),
                logic_levels.get_comb_depth(&n).unwrap()
            );
        }
    }
    // let fo = netlist
    //     .get_analysis::<circuit::graph::FanOutTable<_>>()
    //     .unwrap();
    // for net in netlist.into_iter() {
    //     println!("Net: {}", net);
    //     for user in fo.get_users(&net) {
    //         println!("  User: {}", user.get_instance_name().unwrap());
    //     }
    // }
    // for inst in netlist.objects() {
    //     println!("{}", inst);
    // }
    // let pg = netlist
    //     .get_analysis::<circuit::graph::MultiDiGraph<_>>()
    //     .unwrap();
    // let graph = pg.get_graph();
    // println!("{}", petgraph::dot::Dot::with_config(&graph, &[]));
}

#[test]
fn test_simple_example() {
    let netlist = simple_example();
    assert_eq!(netlist.get_name(), "simple_example");
    assert_eq!(netlist.get_input_ports().count(), 2);
    assert_eq!(netlist.get_output_ports().len(), 1);
    let objects: Vec<_> = netlist.objects().collect();
    assert_eq!(objects.len(), 3); // 2 inputs + 1 gate
}
