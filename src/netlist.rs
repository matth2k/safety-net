/*!

  TODO: netlist docs

*/

use crate::circuit::{Instantiable, Net, Object};
use std::{
    cell::{Ref, RefCell, RefMut},
    collections::{HashMap, HashSet},
    rc::{Rc, Weak},
};

/// A trait for indexing into a collection of objects weakly.
pub trait WeakIndex<Idx: ?Sized> {
    /// The output data type which will be referred to weakly
    type Output: ?Sized;
    /// Indexes the collection weakly by the given index.
    fn index_weak(&self, index: &Idx) -> Rc<RefCell<Self::Output>>;
}

/// A primitive gate in a digital circuit, such as AND, OR, NOT, etc.
#[derive(Debug)]
pub struct GatePrimitive {
    /// The name of the primitive
    name: String,
    /// Input ports, order matters
    inputs: Vec<Net>,
    /// Output ports, order matters
    outputs: Vec<Net>,
}

impl Instantiable for GatePrimitive {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_input_ports(&self) -> &[Net] {
        &self.inputs
    }

    fn get_output_ports(&self) -> &[Net] {
        &self.outputs
    }
}

impl GatePrimitive {
    /// Creates a new gate primitive with four-state logic types
    pub fn new_logical(name: String, inputs: Vec<String>, output: String) -> Self {
        let outputs = vec![Net::new_logic(output)];
        let inputs = inputs.into_iter().map(Net::new_logic).collect::<Vec<_>>();
        Self {
            name,
            inputs,
            outputs,
        }
    }

    /// Returns the single output port of the gate
    pub fn get_single_output_port(&self) -> &Net {
        self.outputs.first().expect("GatePrimitive has no outputs")
    }

    /// Updates the type of cell by name
    pub fn change_gate_name(&mut self, new_name: String) {
        self.name = new_name;
    }
}

/// An operand to an [Instantiable]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Operand {
    /// An index into the list of objects
    DirectIndex(usize),
    /// An index into the list of objects, with an extra index on the cell/primitive
    CellIndex(usize, usize),
}

/// An object that has a reference to its owning netlist/module
#[derive(Debug)]
pub struct OwnedObject<I, O>
where
    I: Instantiable,
    O: WeakIndex<usize, Output = Self>,
{
    /// The object that is owned by the netlist
    object: Object<I>,
    /// The weak reference to the owner netlist/module
    owner: Weak<O>,
    /// The list of operands for the object
    operands: Vec<Operand>,
    /// The index of the object within the netlist/module
    index: usize,
    /// Whether each output port is a top-level output
    is_output: Vec<bool>,
}

impl<I, O> OwnedObject<I, O>
where
    I: Instantiable,
    O: WeakIndex<usize, Output = Self>,
{
    /// Get the operand as a weak index
    pub fn get_operand(&self, index: usize) -> Rc<RefCell<Self>> {
        let operand = &self.operands[index];
        match operand {
            Operand::DirectIndex(idx) => self.owner.upgrade().unwrap().index_weak(idx),
            _ => todo!("get_operand(): Handle other operand types"),
        }
    }

    /// Implement iterator to operands
    pub fn operands_iter(&self) -> impl Iterator<Item = Rc<RefCell<Self>>> {
        self.operands.iter().map(|operand| match operand {
            Operand::DirectIndex(idx) => self.owner.upgrade().unwrap().index_weak(idx),
            _ => todo!("operands_iter(): Handle other operand types"),
        })
    }

    /// Get the underlying object
    pub fn get(&self) -> &Object<I> {
        &self.object
    }

    /// Get the underlying object mutably
    pub fn get_mut(&mut self) -> &mut Object<I> {
        &mut self.object
    }

    /// Get the index of `self` relative to the owning module
    pub fn get_index(&self) -> usize {
        self.index
    }

    /// Returns true if a port on this object is a top-level output
    pub fn has_top_level_output(&self) -> bool {
        self.is_output.iter().any(|&is_out| is_out)
    }

    /// Get the net that is driven by this object
    pub fn as_net(&self) -> &Net {
        match &self.object {
            Object::Input(net) => net,
            Object::Instance(nets, _, _) => {
                if nets.len() > 1 {
                    panic!("Instance has more than one output net");
                } else {
                    nets.first().expect("Instance has no output net")
                }
            }
        }
    }

    /// Get the net that is driven by this object
    pub fn as_net_mut(&mut self) -> &mut Net {
        match &mut self.object {
            Object::Input(net) => net,
            Object::Instance(nets, _, _) => {
                if nets.len() > 1 {
                    panic!("Instance has more than one output net");
                } else {
                    nets.first_mut().expect("Instance has no output net")
                }
            }
        }
    }

    /// Get the operand as a weak index
    pub fn get_operand_as_net(&self, index: usize) -> Net {
        let operand = &self.operands[index];
        match operand {
            Operand::DirectIndex(idx) => self
                .owner
                .upgrade()
                .unwrap()
                .index_weak(idx)
                .borrow()
                .as_net()
                .clone(),
            _ => todo!("get_operand(): Handle other operand types"),
        }
    }
}

/// This type exposes the interior mutability of elements in a netlist.
type NetRefT = Rc<RefCell<OwnedObject<GatePrimitive, Netlist>>>;

/// A helper struct around [NetRefT] to provide
/// a more user-friendly interface to the interior mutability of the netlist
#[derive(Clone)]
pub struct NetRef {
    netref: NetRefT,
}

impl NetRef {
    /// Creates a new [NetRef] from a [NetRefT]
    pub fn new(netref: NetRefT) -> Self {
        Self { netref }
    }

    /// Creates a new [NetRef] from a [NetRefT]
    pub fn wrap(netref: NetRefT) -> Self {
        Self { netref }
    }

    /// Returns the underlying [NetRefT]
    pub fn unwrap(self) -> NetRefT {
        self.netref
    }

    /// Returns a borrow to the [Net] at this circuit node
    pub fn as_net(&self) -> Ref<Net> {
        Ref::map(self.netref.borrow(), |f| f.as_net())
    }

    /// Returns a mutable borrow to the [Net] at this circuit node
    pub fn as_net_mut(&self) -> RefMut<Net> {
        RefMut::map(self.netref.borrow_mut(), |f| f.as_net_mut())
    }

    /// Returns `true` if this circuit node is a principal input
    pub fn is_an_input(&self) -> bool {
        matches!(self.netref.borrow().get(), Object::Input(_))
    }

    /// Returns the [GatePrimitive] type of the instance, if this circuit node is an instance
    pub fn get_instance_type(&self) -> Option<Ref<GatePrimitive>> {
        Ref::filter_map(self.netref.borrow(), |f| {
            match f.get().get_instance_type() {
                Some(inst_type) => Some(inst_type),
                None => None,
            }
        })
        .ok()
    }

    /// Returns the [GatePrimitive] type of the instance, if this circuit node is an instance
    pub fn get_instance_type_mut(&self) -> Option<RefMut<GatePrimitive>> {
        RefMut::filter_map(self.netref.borrow_mut(), |f| {
            match f.get_mut().get_instance_type_mut() {
                Some(inst_type) => Some(inst_type),
                None => None,
            }
        })
        .ok()
    }

    /// Returns a copy of the name of the instance, if the circuit node is a instance.
    pub fn get_instance_name(&self) -> Option<String> {
        match self.netref.borrow().get() {
            Object::Instance(_, inst_name, _) => Some(inst_name.clone()),
            _ => None,
        }
    }
}

/// A netlist data structure
#[derive(Debug)]
pub struct Netlist {
    /// The name of the netlist
    name: String,
    /// The list of objects in the netlist, such as inputs, modules, and primitives
    objects: RefCell<Vec<NetRefT>>,
    /// The list of operands that point to objects which are outputs
    outputs: RefCell<HashMap<Operand, Net>>,
}

impl WeakIndex<usize> for Netlist {
    type Output = OwnedObject<GatePrimitive, Self>;

    fn index_weak(&self, index: &usize) -> Rc<RefCell<Self::Output>> {
        self.objects.borrow()[*index].clone()
    }
}

impl Instantiable for Netlist {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_input_ports(&self) -> &[Net] {
        todo!()
    }

    fn get_output_ports(&self) -> &[Net] {
        todo!()
    }
}

impl Netlist {
    /// Creates a new netlist with the given name
    pub fn new(name: String) -> Rc<Self> {
        Rc::new(Self {
            name,
            objects: RefCell::new(Vec::new()),
            outputs: RefCell::new(HashMap::new()),
        })
    }

    /// Use interior mutability to add an object to the netlist. Returns a mutable reference to the created object.
    fn insert_object(
        self: &Rc<Self>,
        object: Object<GatePrimitive>,
        operands: &[NetRef],
    ) -> NetRef {
        let index = self.objects.borrow().len();
        let weak = Rc::downgrade(self);
        let operands = operands
            .iter()
            .map(|net| Operand::DirectIndex(net.clone().unwrap().borrow().get_index()))
            .collect::<Vec<_>>();
        let noperands = operands.len();
        let owned_object = Rc::new(RefCell::new(OwnedObject {
            object,
            owner: weak,
            operands,
            index,
            is_output: vec![false; noperands],
        }));
        self.objects.borrow_mut().push(owned_object.clone());
        NetRef::wrap(owned_object)
    }

    /// Adds an input net to the netlist
    pub fn insert_input(self: &Rc<Self>, net: Net) -> NetRef {
        let obj = Object::Input(net);
        self.insert_object(obj, &[])
    }

    /// Add a four-state logic input port to the netlist
    pub fn add_input_logic(self: &Rc<Self>, net: String) -> NetRef {
        let net = Net::new_logic(net);
        self.insert_input(net)
    }

    /// Adds a gate to the netlist
    pub fn add_gate(
        self: &Rc<Self>,
        inst_type: GatePrimitive,
        inst_name: String,
        operands: &[NetRef],
    ) -> NetRef {
        let nets = inst_type
            .get_output_ports()
            .iter()
            .map(|pnet| pnet.with_name(format!("{}_{}", inst_name, pnet.get_name())))
            .collect::<Vec<_>>();
        let obj = Object::Instance(nets, inst_name, inst_type);
        self.insert_object(obj, operands)
    }

    /// Set an added object as a top-level output.
    pub fn add_as_output(self: &Rc<Self>, net: NetRef, port: Net) -> NetRef {
        let mut outputs = self.outputs.borrow_mut();
        outputs.insert(
            Operand::DirectIndex(net.clone().unwrap().borrow().get_index()),
            port,
        );
        net
    }
}

impl std::fmt::Display for Netlist {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Borrow everything first
        let objects = self.objects.borrow();
        let outputs = self.outputs.borrow();

        writeln!(f, "module {} (", self.name)?;

        // Print inputs and outputs
        let level = 2;
        let indent = " ".repeat(level);
        for oref in objects.iter() {
            let owned = oref.borrow();
            let obj = owned.get();
            if let Object::Input(net) = obj {
                writeln!(f, "{}{},", indent, net.get_identifier().emit_name())?;
            }
        }
        for (i, (_, net)) in outputs.iter().enumerate() {
            if i == outputs.len() - 1 {
                writeln!(f, "{}{}", indent, net.get_identifier().emit_name())?;
            } else {
                writeln!(f, "{}{},", indent, net.get_identifier().emit_name())?;
            }
        }
        writeln!(f, ");")?;

        // Make wire decls
        let mut already_decl = HashSet::new();
        for oref in objects.iter() {
            let owned = oref.borrow();
            let obj = owned.get();
            if let Object::Input(net) = obj {
                writeln!(f, "{}input {};", indent, net.get_identifier().emit_name())?;
                writeln!(f, "{}wire {};", indent, net.get_identifier().emit_name())?;
                already_decl.insert(net.clone());
            }
        }
        for (_, net) in outputs.iter() {
            if !already_decl.contains(net) {
                writeln!(f, "{}output {};", indent, net.get_identifier().emit_name())?;
                writeln!(f, "{}wire {};", indent, net.get_identifier().emit_name())?;
                already_decl.insert(net.clone());
            }
        }
        for oref in objects.iter() {
            let owned = oref.borrow();
            let obj = owned.get();
            if let Object::Instance(nets, _, _) = obj {
                for net in nets.iter() {
                    if !already_decl.contains(net) {
                        writeln!(f, "{}wire {};", indent, net.get_identifier().emit_name())?;
                        already_decl.insert(net.clone());
                    }
                }
            }
        }

        for oref in objects.iter() {
            let owned = oref.borrow();
            let obj = owned.get();
            if let Object::Instance(nets, inst_name, inst_type) = obj {
                writeln!(f, "{}{} {} (", indent, inst_type.get_name(), inst_name)?;
                let level = 4;
                let indent = " ".repeat(level);
                for (idx, port) in inst_type.get_input_ports().iter().enumerate() {
                    let port_name = port.get_identifier().emit_name();
                    writeln!(
                        f,
                        "{}.{}({}),",
                        indent,
                        port_name,
                        owned.get_operand_as_net(idx).get_identifier().emit_name()
                    )?;
                }

                for (idx, net) in nets.iter().enumerate() {
                    let port_name = inst_type
                        .get_output_port_at(idx)
                        .get_identifier()
                        .emit_name();
                    if idx == nets.len() - 1 {
                        writeln!(
                            f,
                            "{}.{}({})",
                            indent,
                            port_name,
                            net.get_identifier().emit_name()
                        )?;
                    } else {
                        writeln!(
                            f,
                            "{}.{}({}),",
                            indent,
                            port_name,
                            net.get_identifier().emit_name()
                        )?;
                    }
                }

                let level = 2;
                let indent = " ".repeat(level);
                writeln!(f, "{});", indent)?;
            }
        }

        for (driver, net) in outputs.iter() {
            let driver_net = match driver {
                Operand::DirectIndex(idx) => self.index_weak(idx).borrow().as_net().clone(),
                _ => todo!("add_as_output(): Handle other operand types"),
            };
            if *net != driver_net {
                writeln!(
                    f,
                    "{}assign {} = {};",
                    indent,
                    net.get_identifier().emit_name(),
                    driver_net.get_identifier().emit_name()
                )?;
            }
        }

        writeln!(f, "endmodule")
    }
}
