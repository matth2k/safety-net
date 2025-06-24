use bitvec::vec::BitVec;
use circuit::{
    attribute::Parameter,
    circuit::{Identifier, Instantiable, Net},
    netlist::Netlist,
};

#[derive(Debug, Clone)]
struct LUT {
    lookup_table: BitVec,
    id: Identifier,
    inputs: Vec<Net>,
    output: Net,
}

impl LUT {
    fn new(k: usize, lookup_table: usize) -> Self {
        let mut bv: BitVec<usize, _> = BitVec::from_element(lookup_table);
        bv.truncate(1 << k);
        LUT {
            lookup_table: bv,
            id: Identifier::new(format!("LUT{}", k)),
            inputs: (0..k).map(|i| Net::new_logic(format!("I{}", i))).collect(),
            output: Net::new_logic("O".to_string()),
        }
    }

    fn invert(&mut self) {
        self.lookup_table = !self.lookup_table.clone();
    }
}

impl Instantiable for LUT {
    fn get_name(&self) -> &Identifier {
        &self.id
    }

    fn get_input_ports(&self) -> &[Net] {
        &self.inputs
    }

    fn get_output_ports(&self) -> &[Net] {
        std::slice::from_ref(&self.output)
    }

    fn has_parameter(&self, id: &Identifier) -> bool {
        return *id == Identifier::new("INIT".to_string());
    }

    fn get_parameter(&self, id: &Identifier) -> Option<Parameter> {
        if self.has_parameter(id) {
            Some(Parameter::BitVec(self.lookup_table.clone()))
        } else {
            None
        }
    }

    fn parameters(&self) -> impl Iterator<Item = (Identifier, Parameter)> {
        std::iter::once((
            Identifier::new("INIT".to_string()),
            Parameter::BitVec(self.lookup_table.clone()),
        ))
    }
}

fn main() {
    let netlist = Netlist::new("example".to_string());

    // Add the the two inputs
    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());

    // Instantiate an NAND gate
    let instance = netlist
        .insert_gate(LUT::new(2, 7), "inst_0".into(), &[a.into(), b.into()])
        .unwrap();

    // Let's make it an AND gate by inverting the lookup table
    instance.get_instance_type_mut().unwrap().invert();

    // Make this LUT an output
    instance.expose_with_name("y".to_string());

    // Print the netlist
    println!("{}", netlist);
}
