use safety_net::netlist::Netlist;
use safety_net::netlist::Gate;
use safety_net::format_id;
use safety_net::attribute::Parameter;
use safety_net::circuit::{Identifier, Instantiable, Net};
use safety_net::logic::Logic;
use bitvec::vec::BitVec;

fn and_gate() -> Gate {
    Gate::new_logical("AND".into(), vec!["A".into(), "B".into()], "Y".into())
}

fn or_gate() -> Gate {
    Gate::new_logical("OR".into(), vec!["A".into(), "B".into()], "Y".into())
}

fn not_gate() -> Gate {
    Gate::new_logical("NOT".into(), vec!["A".into()], "Y".into())
}
#[test]
fn test_stage_1(){
    let netlist = Netlist::new("example".to_string());
    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());
    let c = netlist.insert_input("c".into());
    let instance = netlist.insert_gate(and_gate(), "inst_0".into(), &[a,b]).unwrap();
    let and_out = instance.get_output(0);
    let not_instance = netlist.insert_gate(not_gate(), 
            "inst_1".into(), &[and_out]).unwrap();
    let not_out = not_instance.get_output(0);
    let final_instance = 
        netlist.insert_gate(or_gate(), 
        "inst_3".into(), &[c, not_out]).unwrap();
    final_instance.expose_with_name("y".into());
    not_instance.expose_with_name("n".into());

}

#[test]
fn test_stage_2() {
    let netlist = Netlist::new("stage2".into());

    // input
    let a = netlist.insert_input("a".into());

    // not gate
    let n_instance = netlist.insert_gate(not_gate(), "inst_0".into(), &[a]).unwrap();
    let n_out = n_instance.get_output(0);

    // constant 1
    let vdd = netlist.insert_constant(Logic::True, "const1".into()).unwrap();

    // and gate: (NOT a) & 1
    let and_instance = netlist.insert_gate(and_gate(), "inst_1".into(), &[n_out, vdd]).unwrap();
    and_instance.set_attribute("dont_touch".into());
    // expose output
    and_instance.expose_with_name("y".into());
   

    // optional: check / print
    assert!(netlist.verify().is_ok());
    println!("{}", netlist.to_string());
}

#[test]
fn test_stage_3() {
    let netlist = Netlist::new("stage3".into());
    let bus = netlist.insert_input_escaped_logic_bus("input_bus".to_string(), 4);
    let bit0 = bus[0].clone();
    let n_instance = netlist.insert_gate(not_gate(), "inst_0".into(), &[bit0]).unwrap();
    let n_out = n_instance.get_output(0);
    n_instance.expose_with_name("y".into());
    println!("{}", netlist.to_string());
}

// Implementing a LUT from the code params.rs

#[derive(Debug, Clone)]
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

// to do 
#[test]
fn test_stage_4(){
    let netlist: std::rc::Rc<Netlist<Lut>> = Netlist::new("example".to_string());

    // Add the the two inputs
    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());
    let instance = netlist
        .insert_gate(Lut::new(2, 7), "inst_0".into(), &[a, b])
        .unwrap();


    // Let's make it an AND gate by inverting the lookup table
    instance.get_instance_type_mut().unwrap().invert();

    // Make this LUT an output
    instance.expose_with_name("y".into());
        println!("{netlist}");

}


// key functions for mutating netlists:
// replace_net_uses(old_net, new_net)
// delete_net_uses(net)
// clean()
// Name management

#[test]
fn replace_input_with_constant() {
    let netlist = Netlist::new("demo".into());

    // input a, input b
    let a = netlist.insert_input("a".into());
    let b = netlist.insert_input("b".into());

    // AND gate
    let and_inst = netlist.insert_gate(and_gate(), "inst_0".into(), &[a.clone(), b.clone()]).unwrap();

    // constant 1
    let vdd = netlist.insert_constant(Logic::True, "const1".into()).unwrap();

    // Old API: panics here because `a` is not a DrivenNet
    //netlist.replace_net_uses( a.unwrap(), &vdd.unwrap()).unwrap();

    // New API: works, because we check dynamically
    netlist.replace_net_uses_driven(a, &vdd).unwrap();

    and_inst.expose_with_name("y".into());
    assert!(netlist.verify().is_ok());
    println!("{}", netlist.to_string());
}
