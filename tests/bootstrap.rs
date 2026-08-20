use safety_net::{
    Gate, Identifier, Instantiable, ModInst, ModOrCell, Net, Netlist, Parameter, assert_verilog_eq,
};
use std::collections::HashMap;
use std::rc::Rc;

type Inst = ModOrCell<Gate>;

fn passthru_nl<I: Instantiable>(id: Identifier) -> Rc<Netlist<I>> {
    let nl = Netlist::new(id);

    let x = nl.insert_input(Net::new_logic("x".into()));

    x.expose_with_name("y".into());

    nl
}

#[test]
fn test_nesting() {
    let outer: Rc<Netlist<Inst>> = passthru_nl("outer".into());
    let inner: Rc<Netlist<Inst>> = passthru_nl("inner".into());

    let inst = ModOrCell::ModInst(ModInst::new(&inner));

    assert!(!inst.is_seq());

    let nr = outer.insert_gate_disconnected(inst, "inst".into());

    let oin = outer.first().unwrap().get_output(0);
    let oin = outer.replace_net_uses(oin, &nr.get_output(0)).unwrap();
    nr.get_input(0).connect(oin);

    let verilog = inner.to_string() + "\n" + &outer.to_string();

    assert_verilog_eq!(
        verilog,
        "module inner (
           x,
           y
         );
           input wire x;
           output wire y;
         
         
           assign y = x;
         
         endmodule
         
         module outer (
           x,
           y
         );
           input wire x;
           output wire y;
           wire inst_y;
         
           inner inst (
             .x(x),
             .y(inst_y)
           );
         
           assign y = inst_y;
         
         endmodule"
            .to_string()
    );
}

#[test]
#[should_panic(expected = "Cannot set parameter")]
fn test_modinst_set_param() {
    let inner: Rc<Netlist<Inst>> = passthru_nl("inner".into());

    let mut inst = ModOrCell::ModInst(ModInst::new(&inner));

    assert!(!inst.has_parameter(&"ex".into()));
    assert!(inst.get_parameter(&"ex".into()).is_none());

    inst.set_parameter(&"ex".into(), Parameter::from_bool(true));
}

#[test]
fn test_modinst_get_const() {
    let inst = ModOrCell::<Gate>::from_constant(false.into()).unwrap();

    assert!(matches!(inst, ModOrCell::Cell(_)));

    assert_eq!(inst.get_constant(), Some(false.into()));
    assert!(!inst.is_seq());
}

#[test]
fn test_clone_into() {
    let outer: Rc<Netlist<Inst>> = passthru_nl("outer".into());
    let inner: Rc<Netlist<Inst>> = passthru_nl("inner".into());

    let input = inner.first().unwrap();
    let _clone = outer.clone_into(&input, Some("myclone".into()), &mut HashMap::new());

    assert_verilog_eq!(
        outer.to_string(),
        "module outer (
           myclone_x,
           x,
           y
         );
           input wire myclone_x;
           input wire x;
           output wire y;


           assign y = x;

         endmodule"
            .to_string()
    );
}

#[test]
fn test_clone_into_inst_into() {
    let outer: Rc<Netlist<Inst>> = passthru_nl("outer".into());
    let inner: Rc<Netlist<Gate>> = passthru_nl("inner".into());

    let input = inner.first().unwrap();
    let _clone = outer.clone_into(&input, Some("myclone".into()), &mut HashMap::new());

    assert_verilog_eq!(
        outer.to_string(),
        "module outer (
           myclone_x,
           x,
           y
         );
           input wire myclone_x;
           input wire x;
           output wire y;


           assign y = x;

         endmodule"
            .to_string()
    );
}
