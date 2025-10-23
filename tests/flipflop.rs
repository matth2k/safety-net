use bitvec::vec::BitVec;
#[cfg(feature = "derive")]
use safety_net::derive::Instantiable;
use safety_net::{
    attribute::Parameter,
    circuit::{Identifier, Instantiable, Net},
    format_id,
    logic::Logic,
    netlist::Gate,
    netlist::GateNetlist,
    netlist::Netlist,
};
use std::str::FromStr;

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

    #[allow(dead_code)]
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

    fn is_seq(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone)]
/// A flip-flop in a digital circuit
struct FlipFlop {
    id: Identifier,
    init_value: Logic,
    q: Net,
    c: Net,
    ce: Net,
    reset: Net,
    d: Net,
}

impl FlipFlop {
    fn new(id: Identifier, init_value: Logic) -> Self {
        if (id.get_name() != "FDRE")
            && (id.get_name() != "FDSE")
            && (id.get_name() != "FDPE")
            && (id.get_name() != "FDCE")
        {
            let name: &str = id.get_name();
            panic!("Unsupported flip-flop type: {name}");
        }
        let q = Net::new_logic("Q".into());
        let c = Net::new_logic("C".into());
        let ce = Net::new_logic("CE".into());
        let reset = Net::new_logic(match id.get_name() {
            "FDRE" => "R".into(),
            "FDSE" => "S".into(),
            "FDPE" => "PRE".into(),
            "FDCE" => "CLR".into(),
            &_ => unreachable!(),
        });
        let d = Net::new_logic("D".into());
        FlipFlop {
            id,
            init_value,
            q,
            c,
            ce,
            reset,
            d,
        }
    }
}

impl Instantiable for FlipFlop {
    fn get_name(&self) -> &Identifier {
        &self.id
    }

    fn get_input_ports(&self) -> impl IntoIterator<Item = &Net> {
        vec![&self.c, &self.ce, &self.reset, &self.d]
    }

    fn get_output_ports(&self) -> impl IntoIterator<Item = &Net> {
        std::slice::from_ref(&self.q)
    }

    fn has_parameter(&self, id: &Identifier) -> bool {
        *id == Identifier::new("INIT".to_string())
    }

    fn get_parameter(&self, id: &Identifier) -> Option<Parameter> {
        if self.has_parameter(id) {
            Some(Parameter::Logic(self.init_value))
        } else {
            None
        }
    }

    fn set_parameter(&mut self, id: &Identifier, val: Parameter) -> Option<Parameter> {
        if !self.has_parameter(id) {
            return None;
        }

        let old = Some(Parameter::Logic(self.init_value));

        if let Parameter::Logic(l) = val {
            self.init_value = l;
        } else {
            panic!("Invalid type for INIT parameter: {val}");
        }

        old
    }

    fn parameters(&self) -> impl Iterator<Item = (Identifier, Parameter)> {
        std::iter::once((
            Identifier::new("INIT".to_string()),
            Parameter::Logic(self.init_value),
        ))
    }

    fn from_constant(_val: Logic) -> Option<Self> {
        None
    }

    fn get_constant(&self) -> Option<Logic> {
        None
    }

    fn is_seq(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Instantiable)]
enum Cell {
    #[instantiable(constant)]
    Lut(Lut),
    FlipFlop(FlipFlop),
    Gate(Gate),
}

#[test]
fn cell_test() {
    let lut = Lut::new(4, 0xAAAA);
    let ff = FlipFlop::new("FDRE".into(), Logic::from_str("1'b0").unwrap());
    let gate = Gate::new_logical("AND".into(), vec!["A".into(), "B".into()], "Y".into());
    let mut cell_lut = Cell::Lut(lut.clone());
    let mut cell_ff = Cell::FlipFlop(ff.clone());
    let cell_gate = Cell::Gate(gate.clone());

    // get_name tests
    assert_eq!(lut.get_name(), cell_lut.get_name());
    assert_eq!(ff.get_name(), cell_ff.get_name());
    assert_eq!(gate.get_name(), cell_gate.get_name());

    // get_input_ports and get_output_ports tests
    let cell_lut_inputs: Vec<_> = cell_lut.get_input_ports().into_iter().collect();
    let lut_inputs: Vec<_> = lut.get_input_ports().into_iter().collect();
    assert_eq!(cell_lut_inputs, lut_inputs);
    let cell_lut_outputs: Vec<_> = cell_lut.get_output_ports().into_iter().collect();
    let lut_outputs: Vec<_> = lut.get_output_ports().into_iter().collect();
    assert_eq!(cell_lut_outputs, lut_outputs);
    let cell_ff_inputs: Vec<_> = cell_ff.get_input_ports().into_iter().collect();
    let ff_inputs: Vec<_> = ff.get_input_ports().into_iter().collect();
    assert_eq!(cell_ff_inputs, ff_inputs);
    let cell_ff_outputs: Vec<_> = cell_ff.get_output_ports().into_iter().collect();
    let ff_outputs: Vec<_> = ff.get_output_ports().into_iter().collect();
    assert_eq!(cell_ff_outputs, ff_outputs);
    let cell_gate_inputs: Vec<_> = cell_gate.get_input_ports().into_iter().collect();
    let gate_inputs: Vec<_> = gate.get_input_ports().into_iter().collect();
    assert_eq!(cell_gate_inputs, gate_inputs);
    let cell_gate_outputs: Vec<_> = cell_gate.get_output_ports().into_iter().collect();
    let gate_outputs: Vec<_> = gate.get_output_ports().into_iter().collect();
    assert_eq!(cell_gate_outputs, gate_outputs);

    // get_parameter and set_parameter tests
    let new_bv: BitVec<usize, _> = BitVec::from_element(0x5555);
    let old_lut_param = cell_lut.set_parameter(&"INIT".into(), Parameter::BitVec(new_bv.clone()));
    if let Some(Parameter::BitVec(bv)) = old_lut_param {
        for i in 0..15 {
            assert_eq!(bv[i], (i % 2 == 1));
        }
    } else {
        panic!("Expected BitVec parameter");
    }
    let lut_param = cell_lut.get_parameter(&"INIT".into());
    if let Some(Parameter::BitVec(bv)) = lut_param {
        for i in 0..15 {
            assert_eq!(bv[i], (i % 2 == 0));
        }
    } else {
        panic!("Expected BitVec parameter");
    }
    let old_ff_param = cell_ff.set_parameter(
        &"INIT".into(),
        Parameter::Logic(Logic::from_str("1'b1").unwrap()),
    );
    assert_eq!(
        old_ff_param,
        Some(Parameter::Logic(Logic::from_str("1'b0").unwrap()))
    );
    let ff_param = cell_ff.get_parameter(&"INIT".into());
    assert_eq!(
        ff_param,
        Some(Parameter::Logic(Logic::from_str("1'b1").unwrap()))
    );

    // parameters tests
    let lut_params: Vec<_> = cell_lut.parameters().collect();
    assert_eq!(lut_params[0].0, Identifier::new("INIT".to_string()));
    let ff_params: Vec<_> = cell_ff.parameters().collect();
    assert_eq!(ff_params[0].0, Identifier::new("INIT".to_string()));

    // from_constant and get_constant tests
    let vdd = Cell::from_constant(Logic::True).unwrap();
    assert_eq!(vdd.get_constant(), Some(Logic::True));
    let gnd = Cell::from_constant(Logic::False).unwrap();
    assert_eq!(gnd.get_constant(), Some(Logic::False));
    assert!(cell_ff.get_constant().is_none());
    assert!(cell_gate.get_constant().is_none());

    // is_seq tests
    assert!(!cell_lut.is_seq());
    assert!(cell_ff.is_seq());
    assert!(!cell_gate.is_seq());
}

#[test]
fn insert_cell_test() {
    let netlist = Netlist::new("test_netlist".to_string());

    let clk = netlist.insert_input("clk".into());
    let ce = netlist.insert_input("ce".into());
    let rst = netlist.insert_input("rst".into());
    let d = netlist.insert_input("d".into());

    let flipflop = FlipFlop::new("FDRE".into(), Logic::from_str("1'bx").unwrap());

    let instance = netlist
        .insert_gate(Cell::FlipFlop(flipflop), "ff1".into(), &[clk, ce, rst, d])
        .unwrap();

    instance.expose_with_name("q".into());
    assert!(netlist.verify().is_ok());
}

fn nand_gate() -> Gate {
    Gate::new_logical("NAND".into(), vec!["A".into(), "B".into()], "Y".into())
}

fn not_gate() -> Gate {
    Gate::new_logical("NOT".into(), vec!["A".into()], "Y".into())
}

#[test]
fn d_flip_flop() {
    let netlist = GateNetlist::new("d_flip_flop".to_string());

    let d = netlist.insert_input("d".into());
    let clk = netlist.insert_input("clk".into());

    let inv_1 = netlist
        .insert_gate(not_gate(), "not1".into(), std::slice::from_ref(&d))
        .unwrap();
    let nand_1 = netlist
        .insert_gate(nand_gate(), "nand_1".into(), &[d, clk.clone()])
        .expect("Failed to insert gate");
    let nand_2 = netlist
        .insert_gate(nand_gate(), "nand_2".into(), &[inv_1.get_output(0), clk])
        .expect("Failed to insert gate");
    let nand_3 = netlist.insert_gate_disconnected(nand_gate(), "nand3".into());
    let nand_4 = netlist.insert_gate_disconnected(nand_gate(), "nand4".into());

    nand_3.get_input(0).connect(nand_1.get_output(0));
    nand_3.get_input(1).connect(nand_4.get_output(0));
    nand_4.get_input(0).connect(nand_3.get_output(0));
    nand_4.get_input(1).connect(nand_2.get_output(0));
    nand_3.expose_with_name("q".into());
    assert!(netlist.verify().is_ok());
}
