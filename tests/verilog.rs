use safety_net::{assert_verilog_eq, netlist::GateNetlist};

#[test]
fn min_module() {
    let netlist = GateNetlist::new("min_module".to_string());
    let a = netlist.insert_input("a".into());
    a.expose_with_name("y".into());
    assert!(netlist.verify().is_ok());
    assert_verilog_eq!(
        netlist.to_string(),
        "module min_module (
           a,
           y
         );
           input a;
           wire a;
           output y;
           wire y;
           assign y = a;
         endmodule\n"
    );
}
