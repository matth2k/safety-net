use bitvec::vec::BitVec;
use safety_net::{
    attribute::Parameter,
    circuit::{Identifier, Instantiable, Net},
    format_id,
    logic::{Logic, dont_care},
    netlist::{Gate, Netlist},
};
//use std::str::FromStr;

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

#[derive(Debug, Clone, PartialEq, Eq)]
enum FlopVariant {
    #[allow(clippy::upper_case_acronyms)]
    FDRE(Identifier),
    #[allow(clippy::upper_case_acronyms)]
    FDSE(Identifier),
    #[allow(clippy::upper_case_acronyms)]
    FDPE(Identifier),
    #[allow(clippy::upper_case_acronyms)]
    FDCE(Identifier),
}

impl FlopVariant {
    fn new(variant: &str) -> Self {
        match variant {
            "FDRE" => FlopVariant::FDRE("FDRE".into()),
            "FDSE" => FlopVariant::FDSE("FDSE".into()),
            "FDPE" => FlopVariant::FDPE("FDPE".into()),
            "FDCE" => FlopVariant::FDCE("FDCE".into()),
            _ => panic!("Unknown flip-flop variant: {}", variant),
        }
    }

    fn from_id(id: &Identifier) -> Self {
        FlopVariant::new(&id.to_string())
    }

    fn get_id(&self) -> &Identifier {
        match self {
            FlopVariant::FDRE(id) => id,
            FlopVariant::FDSE(id) => id,
            FlopVariant::FDPE(id) => id,
            FlopVariant::FDCE(id) => id,
        }
    }

    fn get_reset(self) -> Identifier {
        match self {
            FlopVariant::FDRE(_) => "R".into(),
            FlopVariant::FDSE(_) => "S".into(),
            FlopVariant::FDPE(_) => "PRE".into(),
            FlopVariant::FDCE(_) => "CLR".into(),
        }
    }
}

#[derive(Debug, Clone)]
/// A flip-flop in a digital circuit
struct FlipFlop {
    variant: FlopVariant,
    init_value: Logic,
    q: Net,
    c: Net,
    ce: Net,
    reset: Net,
    d: Net,
}

impl FlipFlop {
    fn new(variant: FlopVariant, init_value: Logic) -> Self {
        let q = Net::new_logic("Q".into());
        let c = Net::new_logic("C".into());
        let ce = Net::new_logic("CE".into());
        let reset = Net::new_logic(variant.clone().get_reset());
        let d = Net::new_logic("D".into());
        FlipFlop {
            variant,
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
        self.variant.get_id()
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

#[derive(Debug, Clone)]
enum Cell {
    Lut(Lut),
    FlipFlop(FlipFlop),
    Gate(Gate),
}

impl Instantiable for Cell {
    fn get_name(&self) -> &Identifier {
        match self {
            Cell::Lut(lut) => lut.get_name(),
            Cell::FlipFlop(ff) => ff.get_name(),
            Cell::Gate(gate) => gate.get_name(),
        }
    }

    fn get_input_ports(&self) -> impl IntoIterator<Item = &Net> {
        match self {
            Cell::Lut(lut) => lut.get_input_ports().into_iter().collect::<Vec<_>>(),
            Cell::FlipFlop(ff) => ff.get_input_ports().into_iter().collect::<Vec<_>>(),
            Cell::Gate(gate) => gate.get_input_ports().into_iter().collect::<Vec<_>>(),
        }
    }

    fn get_output_ports(&self) -> impl IntoIterator<Item = &Net> {
        match self {
            Cell::Lut(lut) => lut.get_output_ports().into_iter().collect::<Vec<_>>(),
            Cell::FlipFlop(ff) => ff.get_output_ports().into_iter().collect::<Vec<_>>(),
            Cell::Gate(gate) => gate.get_output_ports().into_iter().collect::<Vec<_>>(),
        }
    }

    fn has_parameter(&self, id: &Identifier) -> bool {
        match self {
            Cell::Lut(lut) => lut.has_parameter(id),
            Cell::FlipFlop(ff) => ff.has_parameter(id),
            Cell::Gate(gate) => gate.has_parameter(id),
        }
    }

    fn get_parameter(&self, id: &Identifier) -> Option<Parameter> {
        match self {
            Cell::Lut(lut) => lut.get_parameter(id),
            Cell::FlipFlop(ff) => ff.get_parameter(id),
            Cell::Gate(gate) => gate.get_parameter(id),
        }
    }

    fn set_parameter(&mut self, id: &Identifier, val: Parameter) -> Option<Parameter> {
        match self {
            Cell::Lut(lut) => lut.set_parameter(id, val),
            Cell::FlipFlop(ff) => ff.set_parameter(id, val),
            Cell::Gate(gate) => gate.set_parameter(id, val),
        }
    }

    fn parameters(&self) -> impl Iterator<Item = (Identifier, Parameter)> {
        match self {
            Cell::Lut(lut) => lut.parameters().collect::<Vec<_>>().into_iter(),
            Cell::FlipFlop(ff) => ff.parameters().collect::<Vec<_>>().into_iter(),
            Cell::Gate(gate) => gate.parameters().collect::<Vec<_>>().into_iter(),
        }
    }

    fn from_constant(val: Logic) -> Option<Self> {
        if (val == Logic::True) || (val == Logic::False) {
            Lut::from_constant(val).map(Cell::Lut)
        } else {
            None
        }
    }

    fn get_constant(&self) -> Option<Logic> {
        match self {
            Cell::Lut(lut) => lut.get_constant(),
            Cell::FlipFlop(ff) => ff.get_constant(),
            Cell::Gate(gate) => gate.get_constant(),
        }
    }

    fn is_seq(&self) -> bool {
        match self {
            Cell::Lut(lut) => lut.is_seq(),
            Cell::FlipFlop(ff) => ff.is_seq(),
            Cell::Gate(gate) => gate.is_seq(),
        }
    }
}

#[test]
fn test_flopvariant() {
    let fv_1 = FlopVariant::new("FDRE");
    let fv_2 = FlopVariant::from_id(&"FDRE".into());
    assert_eq!(fv_1.get_id(), &"FDRE".into());
    assert_eq!(fv_2.get_reset(), "R".into());
}

#[test]
fn cell_test_get_name() {
    let lut = Lut::new(4, 0xAAAA);
    let ff = FlipFlop::new(FlopVariant::new("FDCE"), Logic::False);
    let gate = Gate::new_logical("AND".into(), vec!["A".into(), "B".into()], "Y".into());
    let cell_lut = Cell::Lut(lut.clone());
    let cell_ff = Cell::FlipFlop(ff.clone());
    let cell_gate = Cell::Gate(gate.clone());

    // get_name tests
    assert_eq!(lut.get_name(), cell_lut.get_name());
    assert_eq!(ff.get_name(), cell_ff.get_name());
    assert_eq!(gate.get_name(), cell_gate.get_name());
}

#[test]
fn cell_test_get_inputs_outputs() {
    let lut = Lut::new(4, 0xAAAA);
    let ff = FlipFlop::new(FlopVariant::new("FDSE"), Logic::False);
    let gate = Gate::new_logical("AND".into(), vec!["A".into(), "B".into()], "Y".into());
    let cell_lut = Cell::Lut(lut.clone());
    let cell_ff = Cell::FlipFlop(ff.clone());
    let cell_gate = Cell::Gate(gate.clone());

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
}

#[test]
fn cell_test_parameters() {
    let lut = Lut::new(4, 0xAAAA);
    let ff = FlipFlop::new(FlopVariant::new("FDSE"), Logic::False);
    let mut cell_lut = Cell::Lut(lut.clone());
    let mut cell_ff = Cell::FlipFlop(ff.clone());

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
    let old_ff_param = cell_ff.set_parameter(&"INIT".into(), Parameter::from_bool(true));
    assert_eq!(old_ff_param, Some(Parameter::from_bool(false)));
    let ff_param = cell_ff.get_parameter(&"INIT".into());
    assert_eq!(ff_param, Some(Parameter::from_bool(true)));

    // parameters tests
    let lut_params: Vec<_> = cell_lut.parameters().collect();
    assert_eq!(lut_params[0].0, Identifier::new("INIT".to_string()));
    let ff_params: Vec<_> = cell_ff.parameters().collect();
    assert_eq!(ff_params[0].0, Identifier::new("INIT".to_string()));
}

#[test]
fn cell_test_constants() {
    // from_constant and get_constant tests
    let vdd = Cell::from_constant(Logic::True).unwrap();
    assert_eq!(vdd.get_constant(), Some(Logic::True));
    let gnd = Cell::from_constant(Logic::False).unwrap();
    assert_eq!(gnd.get_constant(), Some(Logic::False));
}

#[test]
fn cell_test_is_seq() {
    let lut = Lut::new(4, 0xAAAA);
    let ff = FlipFlop::new(FlopVariant::new("FDSE"), Logic::False);
    let gate = Gate::new_logical("AND".into(), vec!["A".into(), "B".into()], "Y".into());
    let cell_lut = Cell::Lut(lut.clone());
    let cell_ff = Cell::FlipFlop(ff.clone());
    let cell_gate = Cell::Gate(gate.clone());

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
    let preset = netlist.insert_input("pre".into());
    let d = netlist.insert_input("d".into());
    let flipflop = FlipFlop::new(FlopVariant::new("FDPE"), dont_care());

    let instance = netlist
        .insert_gate(
            Cell::FlipFlop(flipflop),
            "ff1".into(),
            &[clk, ce, preset, d],
        )
        .unwrap();

    instance.expose_with_name("q".into());
    assert!(netlist.verify().is_ok());
}

#[test]
fn flipflop_test() {
    let mut ff = FlipFlop::new(FlopVariant::new("FDRE"), Logic::False);
    assert_eq!(ff.get_name(), &"FDRE".into());
    let input_ports: Vec<_> = ff.get_input_ports().into_iter().collect();
    assert_eq!(input_ports[0], &Net::new_logic("C".into()));
    assert_eq!(input_ports[1], &Net::new_logic("CE".into()));
    assert_eq!(input_ports[2], &Net::new_logic("R".into()));
    assert_eq!(input_ports[3], &Net::new_logic("D".into()));
    let output_ports: Vec<_> = ff.get_output_ports().into_iter().collect();
    assert_eq!(output_ports[0], &Net::new_logic("Q".into()));
    let params: Vec<_> = ff.parameters().collect();
    assert_eq!(params[0].0, Identifier::new("INIT".to_string()));
    assert_eq!(
        ff.set_parameter(&"INIT".into(), Parameter::from_bool(true)),
        Some(Parameter::from_bool(false))
    );
    assert_eq!(
        ff.get_parameter(&"INIT".into()),
        Some(Parameter::from_bool(true))
    );
    assert!(ff.is_seq());
}
