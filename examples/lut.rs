use bitvec::vec::BitVec;
use safety_net::{
    attribute::Parameter,
    circuit::{Identifier, Instantiable, Net},
    format_id,
    logic::Logic,
    netlist::Netlist,
};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct Lut {
    lookup_table: BitVec,
    id: Identifier,
    inputs: Vec<Net>,
    output: Net,
}

impl Lut {
    fn new(k: usize, lookup_table: usize) -> Self {
        let mut bv: BitVec<usize, _> = BitVec::from_element(lookup_table);
        bv.truncate(1 << k);
        Lut {
            lookup_table: bv,
            id: format_id!("LUT{k}"),
            inputs: (0..k).map(|i| Net::new_logic(format_id!("I{i}"))).collect(),
            output: Net::new_logic("O".into()),
        }
    }

    fn invert(&mut self) {
        self.lookup_table = !self.lookup_table.clone();
    }
}

impl Instantiable for Lut {
    fn get_name(&self) -> &Identifier {
        &self.id
    }

    fn get_input_ports(&self) -> impl IntoIterator<Item = &Net> {
        &self.inputs
    }

    fn get_output_ports(&self) -> impl IntoIterator<Item = &Net> {
        std::slice::from_ref(&self.output)
    }

    fn has_parameter(&self, id: &Identifier) -> bool {
        *id == Identifier::new("INIT".to_string())
    }

    fn get_parameter(&self, id: &Identifier) -> Option<Parameter> {
        if self.has_parameter(id) {
            Some(Parameter::BitVec(self.lookup_table.clone()))
        } else {
            None
        }
    }

    fn set_parameter(&mut self, id: &Identifier, val: Parameter) -> Option<Parameter> {
        if !self.has_parameter(id) {
            return None;
        }

        let old = Some(Parameter::BitVec(self.lookup_table.clone()));

        if let Parameter::BitVec(bv) = val {
            self.lookup_table = bv;
        } else {
            panic!("Invalid parameter type for INIT");
        }

        old
    }

    fn parameters(&self) -> impl Iterator<Item = (Identifier, Parameter)> {
        std::iter::once((
            Identifier::new("INIT".to_string()),
            Parameter::BitVec(self.lookup_table.clone()),
        ))
    }

    fn from_constant(val: Logic) -> Option<Self> {
        match val {
            Logic::True => Some(Self {
                lookup_table: BitVec::from_element(1),
                id: "VDD".into(),
                inputs: vec![],
                output: "Y".into(),
            }),
            Logic::False => Some(Self {
                lookup_table: BitVec::from_element(0),
                id: "GND".into(),
                inputs: vec![],
                output: "Y".into(),
            }),
            _ => None,
        }
    }

    fn get_constant(&self) -> Option<Logic> {
        match self.id.to_string().as_str() {
            "VDD" => Some(Logic::True),
            "GND" => Some(Logic::False),
            _ => None,
        }
    }
}

fn main() {
    let netlist = Netlist::new("example".to_string());

    // Add the the two inputs
    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());

    // Instantiate an NAND gate
    let instance = netlist
        .insert_gate(Lut::new(2, 7), "inst_0".into(), &[a, b])
        .unwrap();

    // Let's make it an AND gate by inverting the lookup table
    instance.get_instance_type_mut().unwrap().invert();

    // Make this LUT an output
    instance.expose_with_name("y".into());

    // Print the netlist
    println!("{netlist}");

    #[cfg(feature = "serde")]
    {
        let res = netlist.reclaim().unwrap().serialize(std::io::stdout());
        if res.is_err() {
            eprintln!("Failed to serialize netlist: {:?}", res.err());
        }
    }
}
