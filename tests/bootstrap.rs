use safety_net::{Gate, Identifier, Instantiable, Net, Netlist, assert_verilog_eq};
use std::collections::HashMap;
use std::rc::Rc;

fn passthru_nl<I: Instantiable>(id: Identifier) -> Rc<Netlist<I>> {
    let nl = Netlist::new(id);

    let x = nl.insert_input(Net::new_logic("x".into()));

    x.expose_with_name("y".into());

    nl
}

#[test]
fn test_clone_into() {
    let outer: Rc<Netlist<Gate>> = passthru_nl("outer".into());
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
