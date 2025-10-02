use safety_net::netlist::{Gate, GateNetlist, Netlist};

fn full_adder() -> Gate {
    Gate::new_logical_multi(
        "FA".into(),
        vec!["CIN".into(), "A".into(), "B".into()],
        vec!["S".into(), "COUT".into()],
    )
}

#[test]
fn replace_multi_output_port_usage() {
    let netlist: std::rc::Rc<GateNetlist> = Netlist::new("multi_replace".to_string());

    // Inputs
    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());
    let cin = netlist.insert_input("cin".into());

    // Instantiate a multi-output gate (FA with S and COUT)
    let fa = netlist
        .insert_gate(full_adder(), "inst_0".into(), &[cin, a, b])
        .unwrap();

    // Expose SUM (output 0) as a top-level output initially
    fa.expose_net(&fa.get_net(0)).unwrap();
    print!("{}", netlist);

    // Sanity: only SUM is a top-level output now
    assert!(fa.get_output(0).is_top_level_output());
    assert!(!fa.get_output(1).is_top_level_output());

    // Replace uses of SUM with CARRY using the multi-output safe API
    // Drop the FA netref to avoid extra outstanding references
    let sum = fa.get_output(0);
    let carry = fa.get_output(1);
    drop(fa);
    netlist.replace_output_uses(sum, carry).unwrap();

    // Re-grab the instance to check the outputs (should be unchanged)
    let fa = netlist.last().unwrap();
    // SUM should still be the top-level output; CARRY should not
    assert!(fa.get_output(0).is_top_level_output());
    assert!(!fa.get_output(1).is_top_level_output());

    assert!(netlist.verify().is_ok());
}
