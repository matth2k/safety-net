//! Demonstrates how to wrap several instantiable types into a 'Cell' enum
//! This could make certain traversals and manipulations easier

#[cfg(feature = "derive")]
mod flipflop {
    use bitvec::vec::BitVec;
    use safety_net::{CombDepthResult, Gate, Netlist, SimpleCombDepth, dont_care, format_id};
    use safety_net::{Identifier, Instantiable, Logic, Net, Parameter};

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum FlopVariant {
        #[allow(clippy::upper_case_acronyms)]
        FDRE,
        #[allow(clippy::upper_case_acronyms)]
        FDSE,
        #[allow(clippy::upper_case_acronyms)]
        FDPE,
        #[allow(clippy::upper_case_acronyms)]
        FDCE,
    }

    impl FlopVariant {
        fn new(variant: &str) -> Self {
            match variant {
                "FDRE" => FlopVariant::FDRE,
                "FDSE" => FlopVariant::FDSE,
                "FDPE" => FlopVariant::FDPE,
                "FDCE" => FlopVariant::FDCE,
                _ => panic!("Unknown flip-flop variant: {}", variant),
            }
        }

        fn from_id(id: &Identifier) -> Self {
            FlopVariant::new(&id.to_string())
        }

        fn get_id(&self) -> Identifier {
            match self {
                FlopVariant::FDRE => "FDRE".into(),
                FlopVariant::FDSE => "FDSE".into(),
                FlopVariant::FDPE => "FDPE".into(),
                FlopVariant::FDCE => "FDCE".into(),
            }
        }

        fn get_reset(self) -> Identifier {
            match self {
                FlopVariant::FDRE => "R".into(),
                FlopVariant::FDSE => "S".into(),
                FlopVariant::FDPE => "PRE".into(),
                FlopVariant::FDCE => "CLR".into(),
            }
        }
    }
    #[derive(Debug, Clone)]
    /// A flip-flop in a digital circuit
    struct FlipFlop {
        init_value: Logic,
        identifier: Identifier,
        q: Net,
        c: Net,
        ce: Net,
        reset: Net,
        d: Net,
    }

    impl FlipFlop {
        fn new(variant: FlopVariant, init_value: Logic) -> Self {
            let identifier = variant.get_id();
            let q = Net::new_logic("Q".into());
            let c = Net::new_logic("C".into());
            let ce = Net::new_logic("CE".into());
            let reset = Net::new_logic(variant.get_reset());
            let d = Net::new_logic("D".into());
            FlipFlop {
                init_value,
                identifier,
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
            &self.identifier
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

    #[derive(Debug, Clone, inst_derive::Instantiable)]
    enum Cell {
        Lut(Lut),
        FlipFlop(FlipFlop),
        #[instantiable(constant)]
        Gate(Gate),
    }

    #[test]
    fn test_flopvariant() {
        let fv_1 = FlopVariant::new("FDRE");
        let fv_2 = FlopVariant::from_id(&"FDRE".into());
        assert_eq!(fv_1.get_id(), "FDRE".into());
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
        let old_lut_param =
            cell_lut.set_parameter(&"INIT".into(), Parameter::BitVec(new_bv.clone()));
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

    fn and() -> Gate {
        Gate::new_logical("AND2".into(), vec!["A".into(), "B".into()], "Y".into())
    }

    fn or3() -> Gate {
        Gate::new_logical(
            "OR3".into(),
            vec!["A".into(), "B".into(), "C".into()],
            "Y".into(),
        )
    }
    fn or2() -> Gate {
        Gate::new_logical("OR2".into(), vec!["A".into(), "B".into()], "Y".into())
    }

    fn inv() -> Gate {
        Gate::new_logical("INV".into(), vec!["A".into()], "Y".into())
    }

    #[test]
    fn test_seq_comb_depth_pipeline() {
        let netlist = Netlist::<Cell>::new("seq_pipeline".to_string());

        // === inputs ===
        let a = netlist.insert_input("a".into());
        let b = netlist.insert_input("b".into());
        let c = netlist.insert_input("c".into());

        let clk = netlist.insert_input("clk".into());
        let ce = netlist.insert_input("ce".into());
        let rst = netlist.insert_input("rst".into());

        // === BEFORE reg1 (depth 1,2,3) ===
        let n1 = netlist
            .insert_gate(Cell::Gate(inv()), "inv1".into(), std::slice::from_ref(&a))
            .unwrap()
            .get_output(0);

        let n2 = netlist
            .insert_gate(Cell::Gate(and()), "and1".into(), &[n1.clone(), b.clone()])
            .unwrap()
            .get_output(0);

        let n3 = netlist
            .insert_gate(Cell::Gate(inv()), "inv2".into(), std::slice::from_ref(&n2))
            .unwrap()
            .get_output(0);

        // === reg1 ===
        let reg1 = netlist
            .insert_gate(
                Cell::FlipFlop(FlipFlop::new(FlopVariant::FDRE, Logic::False)),
                "reg1".into(),
                &[clk.clone(), ce.clone(), rst.clone(), n3.clone()],
            )
            .unwrap();

        let q1 = reg1.get_output(0); // depth resets to 0

        // === BETWEEN reg1 and reg2 (depth 1..4) ===
        let n4 = netlist
            .insert_gate(Cell::Gate(inv()), "inv3".into(), std::slice::from_ref(&q1))
            .unwrap()
            .get_output(0);

        let n5 = netlist
            .insert_gate(Cell::Gate(and()), "and2".into(), &[n4.clone(), c.clone()])
            .unwrap()
            .get_output(0);

        let n6 = netlist
            .insert_gate(
                Cell::Gate(or3()),
                "or1".into(),
                &[n5.clone(), q1.clone(), a.clone()],
            )
            .unwrap()
            .get_output(0);

        let n7 = netlist
            .insert_gate(Cell::Gate(inv()), "inv4".into(), std::slice::from_ref(&n6))
            .unwrap()
            .get_output(0);

        // === reg2 ===
        let reg2 = netlist
            .insert_gate(
                Cell::FlipFlop(FlipFlop::new(FlopVariant::FDRE, Logic::False)),
                "reg2".into(),
                &[clk.clone(), ce.clone(), rst.clone(), n7.clone()],
            )
            .unwrap();

        let q2 = reg2.get_output(0); // reset again

        // === AFTER reg2 (depth 1,2) ===
        let n8 = netlist
            .insert_gate(Cell::Gate(inv()), "inv5".into(), std::slice::from_ref(&q2))
            .unwrap()
            .get_output(0);

        let n9 = netlist
            .insert_gate(Cell::Gate(and()), "and3".into(), &[n8.clone(), b.clone()])
            .unwrap()
            .get_output(0);

        netlist.last().unwrap().expose_with_name("y".into());

        // === run analysis ===
        let depth_info = netlist.get_analysis::<SimpleCombDepth<_>>().unwrap();

        // BEFORE reg1
        assert_eq!(
            depth_info.get_comb_depth(&n1.unwrap()),
            Some(CombDepthResult::Depth(1))
        );
        assert_eq!(
            depth_info.get_comb_depth(&n2.unwrap()),
            Some(CombDepthResult::Depth(2))
        );
        assert_eq!(
            depth_info.get_comb_depth(&n3.unwrap()),
            Some(CombDepthResult::Depth(3))
        );

        // reg outputs reset
        assert_eq!(
            depth_info.get_comb_depth(&q1.unwrap()),
            Some(CombDepthResult::Depth(0))
        );
        assert_eq!(
            depth_info.get_comb_depth(&q2.unwrap()),
            Some(CombDepthResult::Depth(0))
        );

        // between regs
        assert_eq!(
            depth_info.get_comb_depth(&n4.unwrap()),
            Some(CombDepthResult::Depth(1))
        );
        assert_eq!(
            depth_info.get_comb_depth(&n5.unwrap()),
            Some(CombDepthResult::Depth(2))
        );
        assert_eq!(
            depth_info.get_comb_depth(&n6.unwrap()),
            Some(CombDepthResult::Depth(3))
        );
        assert_eq!(
            depth_info.get_comb_depth(&n7.unwrap()),
            Some(CombDepthResult::Depth(4))
        );

        // after reg2
        assert_eq!(
            depth_info.get_comb_depth(&n8.unwrap()),
            Some(CombDepthResult::Depth(1))
        );
        assert_eq!(
            depth_info.get_comb_depth(&n9.unwrap()),
            Some(CombDepthResult::Depth(2))
        );

        // max across all combinational regions
        assert_eq!(depth_info.get_max_depth(), Some(4));
    }

    #[test]
    fn test_clk_div2_simple() {
        let netlist = Netlist::<Cell>::new("clk_div2".to_string());

        // === inputs ===
        let clk = netlist.insert_input("clk".into());
        let ce = netlist.insert_input("ce".into());
        let rst = netlist.insert_input("rst".into());

        let ff = netlist.insert_gate_disconnected(
            Cell::FlipFlop(FlipFlop::new(FlopVariant::FDRE, Logic::False)),
            "ff".into(),
        );
        let ff_clone = ff.clone();

        let inv = netlist.insert_gate_disconnected(Cell::Gate(inv()), "inv".into());

        // === ports ===
        let q = ff.get_output(0);
        let inv_in = inv.find_input(&"A".into()).unwrap();
        let inv_out = inv.get_output(0);

        // === wire sequential feedback ===
        inv_in.connect(q);
        ff.find_input(&"D".into()).unwrap().connect(inv_out);

        // === wire remaining FF inputs ===
        ff.find_input(&"C".into()).unwrap().connect(clk);
        ff.find_input(&"CE".into()).unwrap().connect(ce);
        ff.find_input(&"R".into()).unwrap().connect(rst);

        // === expose output ===
        ff_clone.expose_with_name("clk_div2".into());

        // === sanity check ===
        assert!(netlist.verify().is_ok());

        // === run comb depth analysis ===
        let depth_info = netlist.get_analysis::<SimpleCombDepth<_>>().unwrap();

        // inverter is combinational depth 1
        assert_eq!(
            depth_info.get_comb_depth(&inv),
            Some(CombDepthResult::Depth(1))
        );

        // FF output breaks combinational depth
        assert_eq!(
            depth_info.get_comb_depth(&ff),
            Some(CombDepthResult::Depth(0))
        );

        // NOT a combinational loop
        assert_ne!(
            depth_info.get_comb_depth(&inv),
            Some(CombDepthResult::CombCycle)
        );
    }
    #[test]
    fn test_complex_seq_circuit() {
        let netlist = Netlist::<Cell>::new("complex_circuit".to_string());

        // === inputs ===
        let clk = netlist.insert_input("clk".into());
        let ce = netlist.insert_input("ce".into());
        let rst = netlist.insert_input("rst".into());
        let main_in = netlist.insert_input("in".into());

        // === flip-flops ===
        let ff1 = netlist.insert_gate_disconnected(
            Cell::FlipFlop(FlipFlop::new(FlopVariant::FDRE, Logic::False)),
            "ff1".into(),
        );
        let ff2 = netlist.insert_gate_disconnected(
            Cell::FlipFlop(FlipFlop::new(FlopVariant::FDRE, Logic::False)),
            "ff2".into(),
        );

        // === combinational gates ===
        let inv1 = netlist.insert_gate_disconnected(Cell::Gate(inv()), "inv1".into());
        let inv2 = netlist.insert_gate_disconnected(Cell::Gate(inv()), "inv2".into());
        let and2 = netlist.insert_gate_disconnected(Cell::Gate(and()), "and2".into());
        let or2 = netlist.insert_gate_disconnected(Cell::Gate(or2()), "or2".into());

        // === FF1.Q → INV1 ===
        let ff1_q = ff1.get_output(0);
        ff1_q.connect(inv1.get_input(0));
        let inv1_out = inv1.get_output(0);

        // === FF2.Q → INV2 ===
        let ff2_q = ff2.get_output(0);
        ff2_q.connect(inv2.get_input(0));
        let inv2_out = inv2.get_output(0);

        // === AND inputs ===
        inv1_out.connect(and2.get_input(0)); // from INV1
        inv2_out.connect(and2.get_input(1)); // from INV2
        let and_out = and2.get_output(0);
        and_out.clone().expose_with_name("y".into());

        // === OR inputs ===
        and_out.connect(or2.get_input(0));
        main_in.connect(or2.get_input(1));
        let or_out = or2.get_output(0);

        // === sequential feedback ===
        or_out.connect(ff1.find_input(&"D".into()).unwrap());
        inv1_out.connect(ff2.find_input(&"D".into()).unwrap());

        // === clocks / control ===
        ff1.find_input(&"C".into()).unwrap().connect(clk.clone());
        ff1.find_input(&"CE".into()).unwrap().connect(ce.clone());
        ff1.find_input(&"R".into()).unwrap().connect(rst.clone());

        ff2.find_input(&"C".into()).unwrap().connect(clk);
        ff2.find_input(&"CE".into()).unwrap().connect(ce);
        ff2.find_input(&"R".into()).unwrap().connect(rst);

        assert!(netlist.verify().is_ok());

        let depth_info = netlist.get_analysis::<SimpleCombDepth<_>>().unwrap();

        assert_eq!(
            depth_info.get_comb_depth(&inv1),
            Some(CombDepthResult::Depth(1))
        );

        assert_eq!(
            depth_info.get_comb_depth(&inv2),
            Some(CombDepthResult::Depth(1))
        );

        assert_eq!(
            depth_info.get_comb_depth(&ff1),
            Some(CombDepthResult::Depth(0))
        );

        assert_eq!(
            depth_info.get_comb_depth(&ff2),
            Some(CombDepthResult::Depth(0))
        );

        assert_eq!(
            depth_info.get_comb_depth(&and2),
            Some(CombDepthResult::Depth(2))
        );

        assert_eq!(
            depth_info.get_comb_depth(&or2),
            Some(CombDepthResult::Depth(3))
        );

        assert_ne!(
            depth_info.get_comb_depth(&inv1),
            Some(CombDepthResult::CombCycle)
        );

        assert_eq!(depth_info.get_max_depth(), Some(3));
    }

    #[test]
    fn test_clk_div2_disconnected() {
        let netlist = Netlist::<Cell>::new("clk_div2".to_string());

        // === inputs ===
        let clk = netlist.insert_input("clk".into());
        let ce = netlist.insert_input("ce".into());
        let rst = netlist.insert_input("rst".into());

        let ff = netlist.insert_gate_disconnected(
            Cell::FlipFlop(FlipFlop::new(FlopVariant::FDRE, Logic::False)),
            "ff".into(),
        );
        let ff_clone = ff.clone();

        let inv = netlist.insert_gate_disconnected(Cell::Gate(inv()), "inv".into());

        // === ports ===
        let q = ff.get_output(0);
        let inv_in = inv.find_input(&"A".into()).unwrap();
        let inv_out = inv.get_output(0);

        // === wire sequential feedback ===
        inv_in.clone().connect(q);
        ff.find_input(&"D".into()).unwrap().connect(inv_out);

        // === wire remaining FF inputs ===
        ff.find_input(&"C".into()).unwrap().connect(clk);
        ff.find_input(&"CE".into()).unwrap().connect(ce);
        ff.find_input(&"R".into()).unwrap().connect(rst);

        // === expose output ===
        ff_clone.expose_with_name("clk_div2".into());

        inv_in.disconnect();

        // === run comb depth analysis ===
        let depth_info = netlist.get_analysis::<SimpleCombDepth<_>>().unwrap();

        // inverter is combinational depth 1
        assert_eq!(
            depth_info.get_comb_depth(&inv),
            Some(CombDepthResult::Undefined)
        );

        // FF output breaks combinational depth
        assert_eq!(
            depth_info.get_comb_depth(&ff),
            Some(CombDepthResult::Depth(0))
        );

        // NOT a combinational loop
        assert_ne!(
            depth_info.get_comb_depth(&inv),
            Some(CombDepthResult::CombCycle)
        );
    }
}
