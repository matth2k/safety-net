use safety_net::rewriter::NetMapper;
use safety_net::{Identifier, Instantiable, Logic, Net, Netlist, Parameter, assert_verilog_eq};

#[derive(Debug, Clone)]
enum Gate {
    And(Identifier, Vec<Net>, Net),
    Inv(Identifier, Net, Net),
}

impl Instantiable for Gate {
    fn get_name(&self) -> &Identifier {
        match self {
            Gate::And(id, _, _) | Gate::Inv(id, _, _) => id,
        }
    }

    fn get_input_ports(&self) -> impl IntoIterator<Item = &Net> {
        match self {
            Gate::And(_, inputs, _) => inputs,
            Gate::Inv(_, input, _) => std::slice::from_ref(input),
        }
    }

    fn get_output_ports(&self) -> impl IntoIterator<Item = &Net> {
        match self {
            Gate::And(_, _, output) => std::slice::from_ref(output),
            Gate::Inv(_, _, output) => std::slice::from_ref(output),
        }
    }

    fn has_parameter(&self, _id: &Identifier) -> bool {
        false
    }

    fn get_parameter(&self, _id: &Identifier) -> Option<Parameter> {
        None
    }

    fn set_parameter(&mut self, _id: &Identifier, _val: Parameter) -> Option<Parameter> {
        None
    }

    fn parameters(&self) -> impl Iterator<Item = (Identifier, Parameter)> {
        std::iter::empty()
    }

    fn from_constant(_val: Logic) -> Option<Self> {
        None
    }

    fn get_constant(&self) -> Option<Logic> {
        None
    }

    fn is_seq(&self) -> bool {
        false
    }
}

fn and_gate() -> Gate {
    Gate::And(
        "AND".into(),
        vec![Net::new_logic("A".into()), Net::new_logic("B".into())],
        Net::new_logic("Y".into()),
    )
}

fn inv_gate() -> Gate {
    Gate::Inv(
        "INV".into(),
        Net::new_logic("A".into()),
        Net::new_logic("Y".into()),
    )
}

#[test]
fn test_matches_macro() {
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

    let mut everything = Vec::new();
    for node in netlist.objects() {
        for output in node.outputs() {
            everything.push(output);
        }
    }
    let n = everything.len();

    let mapper = NetMapper::new(&netlist);
    assert!(mapper.is_ok());
    let mut mapper = mapper.unwrap();

    // We use i to differentiate between nets that have the same base identifer.
    for (i, net) in everything.into_iter().enumerate() {
        // Combine the net's base name (n) and i to to create unique instance names
        // across both repeated runs of this pass and nets with identical base names.
        let inst_name =
            net.as_net().get_identifier().clone() + n.to_string().into() + i.to_string().into();

        let net_inv = netlist.insert_gate_disconnected(inv_gate(), inst_name.clone());

        // Repeat the pattern for the second inverter
        let inst_name = inst_name + "inv".into();
        let net_inv_inv = netlist
            .insert_gate(inv_gate(), inst_name, &[net_inv.clone().into()])
            .unwrap();

        // Replace the uses of the original net
        let replacement = net_inv_inv.get_output(0);
        let disconnected = mapper.replace(net, replacement);

        // Now take our disconnected net and drive the inverter pair
        net_inv.get_input(0).connect(disconnected);
    }

    let res = mapper.apply();
    assert!(res.is_ok());
}
