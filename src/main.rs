use circuit::netlist::{GatePrimitive, Netlist, TaggedNet};

#[allow(dead_code)]
fn and_gate() -> GatePrimitive {
    GatePrimitive::new_logical(
        "AND".to_string(),
        vec!["A".to_string(), "B".to_string()],
        "Y".to_string(),
    )
}

fn full_adder() -> GatePrimitive {
    GatePrimitive::new_logical_multi(
        "FA".to_string(),
        vec!["CIN".to_string(), "A".to_string(), "B".to_string()],
        vec!["S".to_string(), "COUT".to_string()],
    )
}

#[allow(dead_code)]
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
            &[input1.clone().into(), input2.clone().into()],
        )
        .unwrap();

    // Make this AND gate an output
    instance.expose_as_output().unwrap();

    // This line won't change anything because it's clever
    if let Some(mut n) = instance
        .req_operand_net(0)
        .unwrap()
        .borrow_mut_if(|n| !n.is_an_input())
    {
        n.set_name("renaming_for_fun".to_string());
    }

    netlist.reclaim().unwrap()
}

fn harder_example() -> Netlist {
    let netlist = Netlist::new("harder_example".to_string());
    let bitwidth = 1000000;

    // Add the the inputs
    let a_vec = netlist.insert_input_escaped_logic_bus("a".to_string(), bitwidth);
    let b_vec = netlist.insert_input_escaped_logic_bus("b".to_string(), bitwidth);
    let cin = netlist.insert_input_logic("cin".to_string());

    // Instantiate the full adders
    let mut input_bus: [TaggedNet; 3] =
        [cin.into(), a_vec[0].clone().into(), b_vec[0].clone().into()];

    for i in 1..bitwidth {
        let instance = netlist
            .insert_gate(full_adder(), format!("fa_{}", i - 1), &input_bus)
            .unwrap();

        instance.expose_net(&instance.get_net(0)).unwrap();

        if i == bitwidth - 1 {
            // Last full adder, expose the carry out
            instance.nets_mut().enumerate().for_each(|(j, mut n)| {
                if j == 1 {
                    n.set_name("cout".to_string());
                }
            });
            instance.expose_net(&instance.get_net(1)).unwrap();
        } else {
            input_bus = [
                (instance.get_net(1).clone(), instance.clone()),
                a_vec[i].clone().into(),
                b_vec[i].clone().into(),
            ];
        }
    }

    netlist.reclaim().unwrap()
}

fn main() {
    let netlist = harder_example();
    let nets = netlist.filter_nets_threaded(|o| o.get_net_at(0).get_name().contains("fa_2_"), 8);
    for net in nets {
        println!("Filtered net: {}", net.get_name());
    }
}

#[test]
fn test_simple_example() {
    let netlist = simple_example();
    assert_eq!(netlist.get_name(), "simple_example");
    assert_eq!(netlist.get_input_ports().len(), 2);
    assert_eq!(netlist.get_output_ports().len(), 1);
    let objects: Vec<_> = netlist.object_iter().collect();
    assert_eq!(objects.len(), 3); // 2 inputs + 1 gate
}
