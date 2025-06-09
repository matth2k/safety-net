/*!

  API for a netlist data structure.

*/

use crate::{
    circuit::{Instantiable, Net, Object},
    graph::Analysis,
};
use std::{
    cell::{Ref, RefCell, RefMut},
    collections::{HashMap, HashSet},
    rc::{Rc, Weak},
};

/// A trait for indexing into a collection of objects weakly.
trait WeakIndex<Idx: ?Sized> {
    /// The output data type which will be referred to weakly
    type Output: ?Sized;
    /// Indexes the collection weakly by the given index.
    fn index_weak(&self, index: &Idx) -> Rc<RefCell<Self::Output>>;
}

/// A primitive gate in a digital circuit, such as AND, OR, NOT, etc.
#[derive(Debug, Clone)]
pub struct GatePrimitive {
    /// The name of the primitive
    name: String,
    /// Input ports, order matters
    inputs: Vec<Net>,
    /// Output ports, order matters
    outputs: Vec<Net>,
}

impl std::fmt::Display for GatePrimitive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
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

    /// Creates a new gate primitive with four-state logic types with multiple outputs
    pub fn new_logical_multi(name: String, inputs: Vec<String>, outputs: Vec<String>) -> Self {
        let outputs = outputs.into_iter().map(Net::new_logic).collect::<Vec<_>>();
        let inputs = inputs.into_iter().map(Net::new_logic).collect::<Vec<_>>();
        Self {
            name,
            inputs,
            outputs,
        }
    }

    /// Returns the single output port of the gate
    pub fn get_single_output_port(&self) -> &Net {
        if self.outputs.len() > 1 {
            panic!("Attempted to grab output port of a multi-output gate");
        }
        self.outputs
            .first()
            .expect("GatePrimitive is missing an output port")
    }

    /// Set the type of cell by name
    pub fn set_gate_name(&mut self, new_name: String) {
        self.name = new_name;
    }

    /// Returns the name of the gate primitive
    pub fn get_gate_name(&self) -> &str {
        &self.name
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
struct OwnedObject<I, O>
where
    I: Instantiable,
    O: WeakIndex<usize, Output = Self>,
{
    /// The object that is owned by the netlist
    object: Object<I>,
    /// The weak reference to the owner netlist/module
    owner: Weak<O>,
    /// The list of operands for the object
    operands: Vec<Option<Operand>>,
    /// The index of the object within the netlist/module
    index: usize,
}

impl<I, O> OwnedObject<I, O>
where
    I: Instantiable,
    O: WeakIndex<usize, Output = Self>,
{
    /// Get the operand as a weak index
    fn get_operand(&self, index: usize) -> Option<Rc<RefCell<Self>>> {
        self.operands[index].as_ref().map(|operand| match operand {
            Operand::DirectIndex(idx) | Operand::CellIndex(idx, _) => self
                .owner
                .upgrade()
                .expect("Object is unlinked from netlist")
                .index_weak(idx),
        })
    }

    /// Iterator to operands
    fn operands(&self) -> impl Iterator<Item = Option<Rc<RefCell<Self>>>> {
        self.operands.iter().map(|operand| {
            operand.as_ref().map(|operand| match operand {
                Operand::DirectIndex(idx) | Operand::CellIndex(idx, _) => self
                    .owner
                    .upgrade()
                    .expect("Object is unlinked from netlist")
                    .index_weak(idx),
            })
        })
    }

    /// Iterator to operand nets
    fn operand_nets(&self) -> impl Iterator<Item = Option<Net>> {
        self.operands.iter().map(|operand| {
            operand.as_ref().map(|operand| match operand {
                Operand::DirectIndex(idx) => self
                    .owner
                    .upgrade()
                    .expect("Object is unlinked from netlist")
                    .index_weak(idx)
                    .borrow()
                    .as_net()
                    .clone(),
                Operand::CellIndex(idx, j) => self
                    .owner
                    .upgrade()
                    .expect("Object is unlinked from netlist")
                    .index_weak(idx)
                    .borrow()
                    .get_net(*j)
                    .clone(),
            })
        })
    }

    /// Get the underlying object
    fn get(&self) -> &Object<I> {
        &self.object
    }

    /// Get the underlying object mutably
    fn get_mut(&mut self) -> &mut Object<I> {
        &mut self.object
    }

    /// Get the index of `self` relative to the owning module
    fn get_index(&self) -> usize {
        self.index
    }

    /// Get the net that is driven by this object
    fn as_net(&self) -> &Net {
        match &self.object {
            Object::Input(net) => net,
            Object::Instance(nets, _, _) => {
                if nets.len() > 1 {
                    panic!("Attempt to grab the net of a multi-output instance");
                } else {
                    nets.first().expect("Instance is missing a net to drive")
                }
            }
        }
    }

    /// Get the net that is driven by this object
    fn as_net_mut(&mut self) -> &mut Net {
        match &mut self.object {
            Object::Input(net) => net,
            Object::Instance(nets, _, _) => {
                if nets.len() > 1 {
                    panic!("Attempt to grab the net of a multi-output instance");
                } else {
                    nets.first_mut()
                        .expect("Instance is missing a net to drive")
                }
            }
        }
    }

    /// Get the net that is driven by this object at position `idx`
    fn get_net(&self, idx: usize) -> &Net {
        match &self.object {
            Object::Input(net) => {
                if idx != 0 {
                    panic!("Nonzero index on an input object");
                }
                net
            }
            Object::Instance(nets, _, _) => &nets[idx],
        }
    }

    /// Get a mutable reference to the net that is driven by this object at position `idx`
    fn get_net_mut(&mut self, idx: usize) -> &mut Net {
        match &mut self.object {
            Object::Input(net) => {
                if idx != 0 {
                    panic!("Nonzero index on an input object");
                }
                net
            }
            Object::Instance(nets, _, _) => &mut nets[idx],
        }
    }

    /// Check if this object drives a specific net
    fn find_net(&self, net: &Net) -> Option<usize> {
        match &self.object {
            Object::Input(input_net) => {
                if input_net == net {
                    Some(0)
                } else {
                    None
                }
            }
            Object::Instance(nets, _, _) => nets.iter().position(|n| n == net),
        }
    }

    /// Attempt to find a mutable reference to a net within this object
    fn find_net_mut(&mut self, net: &Net) -> Option<&mut Net> {
        match &mut self.object {
            Object::Input(input_net) => {
                if input_net == net {
                    Some(input_net)
                } else {
                    None
                }
            }
            Object::Instance(nets, _, _) => nets.iter_mut().find(|n| *n == net),
        }
    }

    /// Get the operand as a weak index
    fn get_operand_net(&self, index: usize) -> Option<Net> {
        let operand = &self.operands[index];
        match operand {
            Some(op) => match op {
                Operand::DirectIndex(idx) => self
                    .owner
                    .upgrade()
                    .expect("Object is unlinked from netlist")
                    .index_weak(idx)
                    .borrow()
                    .as_net()
                    .clone()
                    .into(),
                Operand::CellIndex(idx, j) => self
                    .owner
                    .upgrade()
                    .expect("Object is unlinked from netlist")
                    .index_weak(idx)
                    .borrow()
                    .get_net(*j)
                    .clone()
                    .into(),
            },
            None => None,
        }
    }
}

/// This type exposes the interior mutability of elements in a netlist.
type NetRefT = Rc<RefCell<OwnedObject<GatePrimitive, Netlist>>>;

/// A helper struct to provide a more user-friendly interface
/// to the interior mutability of the netlist
#[derive(Debug, Clone)]
pub struct NetRef {
    netref: NetRefT,
}

impl NetRef {
    /// Creates a new [NetRef] from a [NetRefT]
    fn wrap(netref: NetRefT) -> Self {
        Self { netref }
    }

    /// Returns the underlying [NetRefT]
    fn unwrap(self) -> NetRefT {
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

    /// Returns a borrow to the output [Net] as position `idx`
    pub fn get_net(&self, idx: usize) -> Ref<Net> {
        Ref::map(self.netref.borrow(), |f| f.get_net(idx))
    }

    /// Returns a mutable borrow to the output [Net] as position `idx`
    pub fn get_net_mut(&self, idx: usize) -> RefMut<Net> {
        RefMut::map(self.netref.borrow_mut(), |f| f.get_net_mut(idx))
    }

    /// Returns the name of a circuit node
    pub fn get_name(&self) -> String {
        self.as_net().get_name().to_string()
    }

    /// Changes the name of the circuit node
    pub fn set_name(&self, name: String) {
        self.as_net_mut().set_name(name)
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

    /// Updates the name of the instance, if the circuit node is an instance.
    pub fn set_instance_name(&self, name: String) {
        match self.netref.borrow_mut().get_mut() {
            Object::Instance(_, inst_name, _) => *inst_name = name,
            _ => panic!("Attempted to set instance name on a non-instance object"),
        }
    }

    /// Exposes this circuit node as a top-level output in the netlist.
    pub fn expose_as_output(&self) -> Result<NetRef, String> {
        let netlist = self
            .netref
            .borrow()
            .owner
            .upgrade()
            .expect("NetRef is unlinked from netlist");
        netlist.expose_netref(self.clone())
    }

    /// Exposes this circuit node as a top-level output in the netlist with a specific port name.
    pub fn expose_with_name(&self, port: String) -> NetRef {
        let netlist = self
            .netref
            .borrow()
            .owner
            .upgrade()
            .expect("NetRef is unlinked from netlist");
        netlist.expose_netref_named(self.clone(), port)
    }

    /// Exposes the `net` driven by this circuit node as a top-level output.
    pub fn expose_net(&self, net: &Net) -> Result<(), String> {
        let netlist = self
            .netref
            .borrow()
            .owner
            .upgrade()
            .expect("NetRef is unlinked from netlist");
        let net_index = self.netref.borrow().find_net(net).ok_or(format!(
            "Net {} not found in circuit node",
            net.get_identifier().emit_name()
        ))?;
        netlist.expose_net(
            Operand::CellIndex(self.netref.borrow().get_index(), net_index),
            net.clone(),
        )
    }

    /// Returns the circuit node of the `index`th input
    pub fn get_operand(&self, index: usize) -> Option<NetRef> {
        self.netref.borrow().get_operand(index).map(NetRef::wrap)
    }

    /// Returns the net of the `index`th input
    pub fn get_operand_net(&self, index: usize) -> Option<Net> {
        self.netref.borrow().get_operand_net(index)
    }

    /// Returns a request to mutably borrow the operand net
    pub fn req_operand_net(&self, index: usize) -> Option<MutBorrowReq> {
        let net = self.get_operand_net(index)?;
        let operand = self.get_operand(index).unwrap();
        Some(MutBorrowReq::new(operand, net))
    }

    /// Returns the number of input ports for this circuit node.
    pub fn get_num_input_ports(&self) -> usize {
        if let Some(inst_type) = self.get_instance_type() {
            inst_type.get_input_ports().len()
        } else {
            0
        }
    }

    /// Returns `true` if this circuit node has all its input ports connected.
    pub fn is_fully_connected(&self) -> bool {
        assert_eq!(
            self.netref.borrow().operands.len(),
            self.get_num_input_ports()
        );
        let count_nones = self
            .netref
            .borrow()
            .operands
            .iter()
            .filter(|o| o.is_none())
            .count();
        count_nones == 0
    }

    /// Returns an iterator to the operand circuit nodes.
    pub fn operands(&self) -> impl Iterator<Item = Option<NetRef>> {
        let operands: Vec<Option<NetRef>> = self
            .netref
            .borrow()
            .operands()
            .map(|o| o.map(NetRef::wrap))
            .collect();
        operands.into_iter()
    }

    /// Returns an interator to the operands nets.
    pub fn operand_nets(&self) -> impl Iterator<Item = Option<Net>> {
        let vec: Vec<Option<Net>> = self.netref.borrow().operand_nets().collect();
        vec.into_iter()
    }

    /// Returns an iterator to the output nets of this circuit node.
    #[allow(clippy::unnecessary_to_owned)]
    pub fn nets(&self) -> impl Iterator<Item = Net> {
        self.netref.borrow().get().get_nets().to_vec().into_iter()
    }

    /// Returns an iterator to the output nets of this circuit node, along with the circuit node itself.
    pub fn nets_tagged(&self) -> impl Iterator<Item = TaggedNet> {
        self.nets().map(|net| (net, self.clone()))
    }

    /// Returns an iterator to mutate the output nets of this circuit node.
    pub fn nets_mut(&self) -> impl Iterator<Item = RefMut<Net>> {
        let nnets = self.netref.borrow().get().get_nets().len();
        (0..nnets).map(|i| self.get_net_mut(i))
    }

    /// Returns `true` if this circuit node drives the given net.
    pub fn drives_net(&self, net: &Net) -> bool {
        self.netref.borrow().find_net(net).is_some()
    }

    /// Attempts to find a mutable reference to `net` within this circuit node.
    pub fn find_net_mut(&self, net: &Net) -> Option<RefMut<Net>> {
        RefMut::filter_map(self.netref.borrow_mut(), |f| f.find_net_mut(net)).ok()
    }

    /// Returns `true` if this circuit node has multiple outputs/nets.
    pub fn is_multi_output(&self) -> bool {
        self.netref.borrow().get().get_nets().len() > 1
    }

    /// Deletes the uses of this circuit node from the netlist.
    pub fn delete_uses(self) -> Result<Object<GatePrimitive>, String> {
        let netlist = self
            .netref
            .borrow()
            .owner
            .upgrade()
            .expect("NetRef is unlinked from netlist");
        netlist.delete_net_uses(self)
    }

    /// Replaces the uses of this circuit node in the netlist with another circuit node.
    pub fn replace_uses_with(self, other: &Self) -> Result<Object<GatePrimitive>, String> {
        let netlist = self
            .netref
            .borrow()
            .owner
            .upgrade()
            .expect("NetRef is unlinked from netlist");
        netlist.replace_net_uses(self, other)
    }
}

impl From<NetRef> for TaggedNet {
    fn from(val: NetRef) -> Self {
        (val.clone().as_net().clone(), val)
    }
}

impl From<&NetRef> for TaggedNet {
    fn from(val: &NetRef) -> Self {
        (val.clone().as_net().clone(), val.clone())
    }
}

/// A helper-struct for returning operand net references
pub struct MutBorrowReq {
    from: NetRef,
    ind: Net,
}

impl MutBorrowReq {
    /// Creates a new mutable borrow request
    fn new(from: NetRef, ind: Net) -> Self {
        Self { from, ind }
    }

    /// Mutably borrows the requested net from the circuit node
    pub fn borrow_mut(&self) -> RefMut<Net> {
        self.from.find_net_mut(&self.ind).unwrap()
    }

    /// Returns `true` if the circuit node is an input
    pub fn is_an_input(&self) -> bool {
        self.from.is_an_input()
    }

    /// Attempts to borrow the net mutably if the condition `f` is satisfied.
    pub fn borrow_mut_if(&self, f: impl Fn(&NetRef) -> bool) -> Option<RefMut<Net>> {
        if f(&self.from) {
            Some(self.borrow_mut())
        } else {
            None
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

/// A type alias for a net and its driving reference
pub type TaggedNet = (Net, NetRef);

impl WeakIndex<usize> for Netlist {
    type Output = OwnedObject<GatePrimitive, Self>;

    fn index_weak(&self, index: &usize) -> Rc<RefCell<Self::Output>> {
        self.objects.borrow()[*index].clone()
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

    /// Attempts to reclaim the netlist, returning [Some] if successful.
    pub fn reclaim(self: Rc<Self>) -> Option<Self> {
        Rc::try_unwrap(self).ok()
    }

    /// Returns the index in [Operand] format of this [TaggedNet]
    fn get_operand_of_tag(t: &TaggedNet) -> Operand {
        let nr = &t.1;
        let no_outputs = nr.clone().unwrap().borrow().get().get_nets().len();
        if no_outputs == 1 {
            Operand::DirectIndex(nr.clone().unwrap().borrow().get_index())
        } else {
            let secondary = nr.clone().unwrap().borrow().find_net(&t.0).unwrap();
            Operand::CellIndex(nr.clone().unwrap().borrow().get_index(), secondary)
        }
    }

    /// Use interior mutability to add an object to the netlist. Returns a mutable reference to the created object.
    fn insert_object(
        self: &Rc<Self>,
        object: Object<GatePrimitive>,
        operands: &[TaggedNet],
    ) -> Result<NetRef, String> {
        let index = self.objects.borrow().len();
        let weak = Rc::downgrade(self);
        let operands = operands
            .iter()
            .map(|net| Some(Self::get_operand_of_tag(net)))
            .collect::<Vec<_>>();
        let owned_object = Rc::new(RefCell::new(OwnedObject {
            object,
            owner: weak,
            operands,
            index,
        }));
        self.objects.borrow_mut().push(owned_object.clone());
        Ok(NetRef::wrap(owned_object))
    }

    /// Inserts an input net to the netlist
    pub fn insert_input_net(self: &Rc<Self>, net: Net) -> NetRef {
        let obj = Object::Input(net);
        self.insert_object(obj, &[]).unwrap()
    }

    /// Inserts a four-state logic input port to the netlist
    pub fn insert_input_logic(self: &Rc<Self>, net: String) -> NetRef {
        let net = Net::new_logic(net);
        self.insert_input_net(net)
    }

    /// Inserts a four-state logic input port to the netlist
    pub fn insert_input_escaped_logic_bus(self: &Rc<Self>, net: String, bw: usize) -> Vec<NetRef> {
        Net::new_escaped_logic_bus(net, bw)
            .into_iter()
            .map(|n| self.insert_input_net(n))
            .collect()
    }

    /// Inserts a gate to the netlist
    pub fn insert_gate(
        self: &Rc<Self>,
        inst_type: GatePrimitive,
        inst_name: String,
        operands: &[TaggedNet],
    ) -> Result<NetRef, String> {
        let nets = inst_type
            .get_output_ports()
            .iter()
            .map(|pnet| pnet.with_name(format!("{}_{}", inst_name, pnet.get_name())))
            .collect::<Vec<_>>();
        if operands.len() != inst_type.get_input_ports().len() {
            return Err(format!(
                "Expected {} operands, got {}",
                inst_type.get_input_ports().len(),
                operands.len()
            ));
        }
        let obj = Object::Instance(nets, inst_name, inst_type);
        self.insert_object(obj, operands)
    }

    /// Set an added object as a top-level output.
    pub fn expose_netref_named(&self, net: NetRef, name: String) -> NetRef {
        let mut outputs = self.outputs.borrow_mut();
        outputs.insert(
            Operand::DirectIndex(net.clone().unwrap().borrow().get_index()),
            net.clone().unwrap().borrow().as_net().with_name(name),
        );
        net
    }

    /// Set an added object as a top-level output.
    pub fn expose_netref(&self, net: NetRef) -> Result<NetRef, String> {
        if net.is_an_input() {
            return Err("Cannot expose an input net as output without a new name".to_string());
        }
        let mut outputs = self.outputs.borrow_mut();
        outputs.insert(
            Operand::DirectIndex(net.clone().unwrap().borrow().get_index()),
            net.clone().unwrap().borrow().as_net().clone(),
        );
        Ok(net)
    }

    /// Get the circuit node with the given operand index.
    fn lookup_netref(&self, operand: Operand) -> NetRef {
        match operand {
            Operand::DirectIndex(idx) | Operand::CellIndex(idx, _) => {
                NetRef::wrap(self.objects.borrow()[idx].clone())
            }
        }
    }

    /// Set an added object as a top-level output.
    fn expose_net(&self, operand: Operand, net: Net) -> Result<(), String> {
        let netref = self.lookup_netref(operand.clone());
        if netref.is_an_input() {
            return Err("Cannot expose an input net as output without a new name".to_string());
        }
        let mut outputs = self.outputs.borrow_mut();
        outputs.insert(operand, net);
        Ok(())
    }

    /// Unlink a circuit node from the rest of the netlist. Return the object that was being stored.
    pub fn delete_net_uses(&self, netref: NetRef) -> Result<Object<GatePrimitive>, String> {
        let unwrapped = netref.clone().unwrap();
        if Rc::strong_count(&unwrapped) > 3 {
            return Err("Cannot delete a netref that is still in use elsewhere".to_string());
        }
        let old_index = unwrapped.borrow().get_index();
        let objects = self.objects.borrow();
        for oref in objects.iter() {
            let operands = &mut oref.borrow_mut().operands;
            for operand in operands.iter_mut() {
                if let Some(op) = operand {
                    match op {
                        Operand::DirectIndex(idx) | Operand::CellIndex(idx, _)
                            if *idx == old_index =>
                        {
                            *operand = None;
                        }
                        _ => (),
                    }
                }
            }
        }

        let outputs: Vec<Operand> = self
            .outputs
            .borrow()
            .keys()
            .filter(|operand| match operand {
                Operand::DirectIndex(idx) | Operand::CellIndex(idx, _) => *idx == old_index,
            })
            .cloned()
            .collect();

        for operand in outputs {
            self.outputs.borrow_mut().remove(&operand);
        }

        Ok(netref.unwrap().borrow().get().clone())
    }

    /// Replaces the uses of a circuit node with another circuit node. The [Object] stored at `of` is returned.
    pub fn replace_net_uses(
        &self,
        of: NetRef,
        with: &NetRef,
    ) -> Result<Object<GatePrimitive>, String> {
        let unwrapped = of.clone().unwrap();
        if Rc::strong_count(&unwrapped) > 3 {
            return Err("Cannot delete a netref that is still in use elsewhere".to_string());
        }

        let old_tag: TaggedNet = of.clone().into();
        let old_index = Self::get_operand_of_tag(&old_tag);
        let new_tag: TaggedNet = with.clone().into();
        let new_index = Self::get_operand_of_tag(&new_tag);
        let objects = self.objects.borrow();
        for oref in objects.iter() {
            let operands = &mut oref.borrow_mut().operands;
            for operand in operands.iter_mut() {
                if let Some(op) = operand {
                    if *op == old_index {
                        *operand = Some(new_index.clone());
                    }
                }
            }
        }

        if self.outputs.borrow().contains_key(&new_index) {
            self.outputs.borrow_mut().remove(&old_index);
        } else {
            self.outputs
                .borrow_mut()
                .insert(new_index, self.outputs.borrow()[&old_index].clone());
            self.outputs.borrow_mut().remove(&old_index);
        }

        Ok(of.unwrap().borrow().get().clone())
    }
}

impl Netlist {
    /// Returns the name of the netlist module
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Returns a list of input nets
    pub fn get_input_ports(&self) -> Vec<Net> {
        self.objects
            .borrow()
            .iter()
            .filter_map(|oref| {
                let owned = oref.borrow();
                if let Object::Input(net) = owned.get() {
                    Some(net.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Returns a list of ouput nets
    pub fn get_output_ports(&self) -> Vec<Net> {
        self.outputs.borrow().values().cloned().collect::<Vec<_>>()
    }

    /// Constructs an analysis of the netlist.
    pub fn get_analysis<A: Analysis>(&self) -> Result<A, String> {
        A::build(self)
    }
}

/// An iterator over the nets in a netlist
pub struct NetIterator<'a> {
    netlist: &'a Netlist,
    index: usize,
    subindex: usize,
}

impl<'a> NetIterator<'a> {
    /// Creates a new iterator for the netlist
    fn new(netlist: &'a Netlist) -> Self {
        Self {
            netlist,
            index: 0,
            subindex: 0,
        }
    }
}

impl Iterator for NetIterator<'_> {
    type Item = Net;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.netlist.objects.borrow().len() {
            let objects = self.netlist.objects.borrow();
            let object = objects[self.index].borrow();
            if self.subindex < object.get().get_nets().len() {
                let net = object.get().get_nets()[self.subindex].clone();
                self.subindex += 1;
                return Some(net);
            }
            self.subindex = 0;
            self.index += 1;
        }
        None
    }
}

/// An iterator over the nets in a netlist
pub struct ObjectIterator<'a> {
    netlist: &'a Netlist,
    index: usize,
}

impl<'a> ObjectIterator<'a> {
    /// Creates a new iterator for the netlist
    fn new(netlist: &'a Netlist) -> Self {
        Self { netlist, index: 0 }
    }
}

impl Iterator for ObjectIterator<'_> {
    type Item = Object<GatePrimitive>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.netlist.objects.borrow().len() {
            let objects = self.netlist.objects.borrow();
            let object = objects[self.index].borrow();
            self.index += 1;
            return Some(object.get().clone());
        }
        None
    }
}

/// The [Net] connects as an input to [Object]
pub type Connection = (Net, NetRef);

/// An iterator over the connections in a netlist
pub struct ConnectionIterator<'a> {
    netlist: &'a Netlist,
    index: usize,
    subindex: usize,
}

impl<'a> ConnectionIterator<'a> {
    /// Creates a new iterator for the netlist
    fn new(netlist: &'a Netlist) -> Self {
        Self {
            netlist,
            index: 0,
            subindex: 0,
        }
    }
}

impl Iterator for ConnectionIterator<'_> {
    type Item = Connection;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.netlist.objects.borrow().len() {
            let objects = self.netlist.objects.borrow();
            let object = objects[self.index].borrow();
            let noperands = object.operands.len();
            while self.subindex < noperands {
                if let Some(operand) = &object.operands[self.subindex] {
                    let net = match operand {
                        Operand::DirectIndex(idx) => objects[*idx].borrow().as_net().clone(),
                        Operand::CellIndex(idx, j) => objects[*idx].borrow().get_net(*j).clone(),
                    };
                    self.subindex += 1;
                    return Some((net, NetRef::wrap(objects[self.index].clone())));
                }
                self.subindex += 1;
            }
            self.subindex = 0;
            self.index += 1;
        }
        None
    }
}

impl<'a> IntoIterator for &'a Netlist {
    type Item = Net;
    type IntoIter = NetIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        NetIterator::new(self)
    }
}

impl Netlist {
    /// Returns an iterator over the  circuit nodes in the netlist.
    pub fn objects(&self) -> impl Iterator<Item = Object<GatePrimitive>> {
        ObjectIterator::new(self)
    }

    /// Returns an iterator over the wire connections in the netlist.
    pub fn connections(&self) -> impl Iterator<Item = Connection> {
        ConnectionIterator::new(self)
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
                    if let Some(operand) = owned.operands[idx].as_ref() {
                        let operand = match operand {
                            Operand::DirectIndex(idx) => objects[*idx].borrow().as_net().clone(),
                            Operand::CellIndex(idx, j) => {
                                objects[*idx].borrow().get_net(*j).clone()
                            }
                        };
                        writeln!(
                            f,
                            "{}.{}({}),",
                            indent,
                            port_name,
                            operand.get_identifier().emit_name()
                        )?;
                    }
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
                Operand::CellIndex(idx, j) => self.index_weak(idx).borrow().get_net(*j).clone(),
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
