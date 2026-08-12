use safety_net::{
    Gate, Identifier, Instantiable, ModInst, ModOrCell, Net, Netlist, assert_verilog_eq,
};
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
