use circuit::{
    attribute::dont_touch_filter,
    netlist::{Gate, Netlist},
};

fn and_gate() -> Gate {
    Gate::new_logical("AND".into(), vec!["A".into(), "B".into()], "Y".into())
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
    instance.set_attribute("dont_touch".to_string());
    instance.expose_with_name("y".into());

    for nr in dont_touch_filter(&netlist) {
        println!("Don't touch: {nr}");
    }
}
