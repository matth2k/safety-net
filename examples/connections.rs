use circuit::netlist::{DrivenNet, Gate, Netlist};

#[allow(dead_code)]
fn and_gate() -> Gate {
    Gate::new_logical(
        "AND".to_string(),
        vec!["A".to_string(), "B".to_string()],
        "Y".to_string(),
    )
}

fn full_adder() -> Gate {
    Gate::new_logical_multi(
        "FA".to_string(),
        vec!["CIN".to_string(), "A".to_string(), "B".to_string()],
        vec!["S".to_string(), "COUT".to_string()],
    )
}

fn harder_example() -> Netlist<Gate> {
    let netlist = Netlist::new("harder_example".to_string());
    let bitwidth = 8;

    // Add the the inputs
    let a_vec = netlist.insert_input_escaped_logic_bus("a".to_string(), bitwidth);
    let b_vec = netlist.insert_input_escaped_logic_bus("b".to_string(), bitwidth);
    let mut carry: DrivenNet<Gate> = netlist.insert_input("cin".into()).into();

    for i in 0..bitwidth {
        // Instantiate a full adder for each bit
        let fa = netlist
            .insert_gate_disconnected(full_adder(), format!("fa_{}", i))
            .unwrap();

        // Connect A_i and B_i
        fa.get_input(1).connect(a_vec[i].clone().into());
        fa.get_input(2).connect(b_vec[i].clone().into());

        // Connect with the prev carry
        carry.connect(fa.get_input(0));

        // Expose the sum
        fa.expose_net(&fa.get_net(0)).unwrap();

        carry = fa.get_output(1);
    }

    netlist.reclaim().unwrap()
}

fn main() {
    let netlist = harder_example();
    print!("{}", netlist);
    // let fo = netlist.get_analysis::<FanOutTable<_>>().unwrap();
    // for net in netlist.into_iter() {
    //     println!("Net: {}", net);
    //     for user in fo.get_users(&net) {
    //         println!("  User: {}", user.get_instance_name().unwrap());
    //     }
    // }
    // for inst in netlist.objects() {
    //     println!("{}", inst);
    // }
}

#[test]
fn test_simple_example() {
    let netlist = simple_example();
    assert_eq!(netlist.get_name(), "simple_example");
    assert_eq!(netlist.get_input_ports().len(), 2);
    assert_eq!(netlist.get_output_ports().len(), 1);
    let objects: Vec<_> = netlist.objects().collect();
    assert_eq!(objects.len(), 3); // 2 inputs + 1 gate
}
