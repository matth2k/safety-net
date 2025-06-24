/*!

  API for a netlist data structure.

*/

use crate::{
    attribute::Parameter,
    circuit::{Identifier, Instantiable, Net, Object},
    graph::{Analysis, FanOutTable},
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
pub struct Gate {
    /// The name of the primitive
    name: Identifier,
    /// Input ports, order matters
    inputs: Vec<Net>,
    /// Output ports, order matters
    outputs: Vec<Net>,
}

impl std::fmt::Display for Gate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Instantiable for Gate {
    fn get_name(&self) -> &Identifier {
        &self.name
    }

    fn get_input_ports(&self) -> &[Net] {
        &self.inputs
    }

    fn get_output_ports(&self) -> &[Net] {
        &self.outputs
    }

    fn has_parameter(&self, _id: &Identifier) -> bool {
        false
    }

    fn get_parameter(&self, _id: &Identifier) -> Option<Parameter> {
        None
    }

    fn parameters(&self) -> impl Iterator<Item = (Identifier, Parameter)> {
        std::iter::empty()
    }
}

impl Gate {
    /// Creates a new gate primitive with four-state logic types
    pub fn new_logical(name: Identifier, inputs: Vec<String>, output: String) -> Self {
        let outputs = vec![Net::new_logic(output)];
        let inputs = inputs.into_iter().map(Net::new_logic).collect::<Vec<_>>();
        Self {
            name,
            inputs,
            outputs,
        }
    }

    /// Creates a new gate primitive with four-state logic types with multiple outputs
    pub fn new_logical_multi(name: Identifier, inputs: Vec<String>, outputs: Vec<String>) -> Self {
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
            .expect("Gate is missing an output port")
    }

    /// Set the type of cell by name
    pub fn set_gate_name(&mut self, new_name: Identifier) {
        self.name = new_name;
    }

    /// Returns the name of the gate primitive
    pub fn get_gate_name(&self) -> &Identifier {
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

impl Operand {
    /// Remap the node index of the operand to `x`.
    fn remap(self, x: usize) -> Self {
        match self {
            Operand::DirectIndex(_idx) => Operand::DirectIndex(x),
            Operand::CellIndex(_idx, j) => Operand::CellIndex(x, j),
        }
    }

    /// Returns the circuit node index
    fn root(&self) -> usize {
        match self {
            Operand::DirectIndex(idx) => *idx,
            Operand::CellIndex(idx, _) => *idx,
        }
    }

    /// Returns the secondary index (the cell index)
    fn secondary(&self) -> usize {
        match self {
            Operand::DirectIndex(_) => 0,
            Operand::CellIndex(_, j) => *j,
        }
    }
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
    /// Get an iterator to mutate the operand indices
    fn inds_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        self.operands
            .iter_mut()
            .filter_map(|operand| operand.as_mut())
    }

    /// Get the driver to input `index`
    fn get_driver(&self, index: usize) -> Option<Rc<RefCell<Self>>> {
        self.operands[index].as_ref().map(|operand| {
            self.owner
                .upgrade()
                .expect("Object is unlinked from netlist")
                .index_weak(&operand.root())
        })
    }

    /// Iterator to driving objects
    fn drivers(&self) -> impl Iterator<Item = Option<Rc<RefCell<Self>>>> {
        self.operands.iter().map(|operand| {
            operand.as_ref().map(|operand| {
                self.owner
                    .upgrade()
                    .expect("Object is unlinked from netlist")
                    .index_weak(&operand.root())
            })
        })
    }

    /// Iterator to driving nets
    fn driver_nets(&self) -> impl Iterator<Item = Option<Net>> {
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

    /// Get driving net using the weak reference
    fn get_driver_net(&self, index: usize) -> Option<Net> {
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
type NetRefT<I> = Rc<RefCell<OwnedObject<I, Netlist<I>>>>;

/// Provides an idiomatic interface
/// to the interior mutability of the netlist
#[derive(Debug, Clone)]
pub struct NetRef<I>
where
    I: Instantiable,
{
    netref: NetRefT<I>,
}

impl<I> NetRef<I>
where
    I: Instantiable,
{
    /// Creates a new [NetRef] from a [NetRefT]
    fn wrap(netref: NetRefT<I>) -> Self {
        Self { netref }
    }

    /// Returns the underlying [NetRefT]
    fn unwrap(self) -> NetRefT<I> {
        self.netref
    }

    /// Returns a borrow to the [Net] at this circuit node.
    /// Panics if the circuit node has multiple outputs.
    pub fn as_net(&self) -> Ref<Net> {
        Ref::map(self.netref.borrow(), |f| f.as_net())
    }

    /// Returns a mutable borrow to the [Net] at this circuit node.
    /// Panics if the circuit node has multiple outputs.
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

    /// Returns a borrow to the output [Net] as position `idx`
    pub fn get_output(&self, idx: usize) -> DrivenNet<I> {
        DrivenNet::new(idx, self.clone())
    }

    /// Returns an abstraction around the input connection
    pub fn get_input(&self, idx: usize) -> InputPort<I> {
        if self.is_an_input() {
            panic!("Principal inputs do not have inputs");
        }
        InputPort::new(idx, self.clone())
    }

    /// Returns the name of the net at this circuit node.
    /// Panics if the circuit node has multiple outputs.
    pub fn get_identifier(&self) -> Identifier {
        self.as_net().get_identifier().clone()
    }

    /// Changes the identifier of the net at this circuit node.
    /// Panics if the circuit node has multiple outputs.
    pub fn set_identifier(&self, identifier: Identifier) {
        self.as_net_mut().set_identifier(identifier)
    }

    /// Returns `true` if this circuit node is a principal input
    pub fn is_an_input(&self) -> bool {
        matches!(self.netref.borrow().get(), Object::Input(_))
    }

    /// Returns a reference to the object at this node.
    pub fn get_obj(&self) -> Ref<Object<I>> {
        Ref::map(self.netref.borrow(), |f| f.get())
    }

    /// Returns the [Instantiable] type of the instance, if this circuit node is an instance
    pub fn get_instance_type(&self) -> Option<Ref<I>> {
        Ref::filter_map(self.netref.borrow(), |f| f.get().get_instance_type()).ok()
    }

    /// Returns the [Instantiable] type of the instance, if this circuit node is an instance
    pub fn get_instance_type_mut(&self) -> Option<RefMut<I>> {
        RefMut::filter_map(self.netref.borrow_mut(), |f| {
            f.get_mut().get_instance_type_mut()
        })
        .ok()
    }

    /// Returns a copy of the name of the instance, if the circuit node is a instance.
    pub fn get_instance_name(&self) -> Option<Identifier> {
        match self.netref.borrow().get() {
            Object::Instance(_, inst_name, _) => Some(inst_name.clone()),
            _ => None,
        }
    }

    /// Updates the name of the instance, if the circuit node is an instance.
    /// Panics if the circuit node is not an instance.
    pub fn set_instance_name(&self, name: Identifier) {
        match self.netref.borrow_mut().get_mut() {
            Object::Instance(_, inst_name, _) => *inst_name = name,
            _ => panic!("Attempted to set instance name on a non-instance object"),
        }
    }

    /// Exposes this circuit node as a top-level output in the netlist.
    /// Panics if cell is a multi-output circuit node. Errors if circuit node is a principal input.
    pub fn expose_as_output(self) -> Result<Self, String> {
        let netlist = self
            .netref
            .borrow()
            .owner
            .upgrade()
            .expect("NetRef is unlinked from netlist");
        netlist.expose_net(self.clone().into())?;
        Ok(self)
    }

    /// Exposes this circuit node as a top-level output in the netlist with a specific port name.
    pub fn expose_with_name(self, name: String) -> Self {
        let netlist = self
            .netref
            .borrow()
            .owner
            .upgrade()
            .expect("NetRef is unlinked from netlist");
        netlist.expose_net_with_name(self.clone().into(), name);
        self
    }

    /// Exposes the `net` driven by this circuit node as a top-level output.
    /// Errors if `net` is not driven by this circuit node.
    pub fn expose_net(&self, net: &Net) -> Result<(), String> {
        let netlist = self
            .netref
            .borrow()
            .owner
            .upgrade()
            .expect("NetRef is unlinked from netlist");
        let net_index = self.netref.borrow().find_net(net).ok_or(format!(
            "Net {} not found in circuit node",
            net.get_identifier()
        ))?;
        let dr = DrivenNet::new(net_index, self.clone());
        netlist.expose_net(dr)?;
        Ok(())
    }

    /// Returns the circuit node that drives the `index`th input
    pub fn get_driver(&self, index: usize) -> Option<Self> {
        self.netref.borrow().get_driver(index).map(NetRef::wrap)
    }

    /// Returns the net that drives the `index`th input
    pub fn get_driver_net(&self, index: usize) -> Option<Net> {
        self.netref.borrow().get_driver_net(index)
    }

    /// Returns a request to mutably borrow the operand net
    /// This requires another borrow in the form of [MutBorrowReq]
    pub fn req_driver_net(&self, index: usize) -> Option<MutBorrowReq<I>> {
        let net = self.get_driver_net(index)?;
        let operand = self.get_driver(index).unwrap();
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
        self.netref.borrow().operands.iter().any(|o| o.is_none())
    }

    /// Returns an iterator to the driving circuit nodes.
    pub fn drivers(&self) -> impl Iterator<Item = Option<Self>> {
        let drivers: Vec<Option<Self>> = self
            .netref
            .borrow()
            .drivers()
            .map(|o| o.map(NetRef::wrap))
            .collect();
        drivers.into_iter()
    }

    /// Returns an interator to the driving nets.
    pub fn driver_nets(&self) -> impl Iterator<Item = Option<Net>> {
        let vec: Vec<Option<Net>> = self.netref.borrow().driver_nets().collect();
        vec.into_iter()
    }

    /// Returns an iterator to the output nets of this circuit node.
    #[allow(clippy::unnecessary_to_owned)]
    pub fn nets(&self) -> impl Iterator<Item = Net> {
        self.netref.borrow().get().get_nets().to_vec().into_iter()
    }

    /// Returns an iterator to the output nets of this circuit node, along with port information.
    pub fn inputs(&self) -> impl Iterator<Item = InputPort<I>> {
        let len = self.netref.borrow().get().get_nets().len();
        (0..len).map(move |i| InputPort::new(i, self.clone()))
    }

    /// Returns an iterator to the output nets of this circuit node, along with port information.
    pub fn outputs(&self) -> impl Iterator<Item = DrivenNet<I>> {
        let len = self.netref.borrow().get().get_nets().len();
        (0..len).map(move |i| DrivenNet::new(i, self.clone()))
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

    /// Returns `true` if this circuit node drives a top-level output.
    pub fn drives_an_top_output(&self) -> bool {
        let netlist = self
            .netref
            .borrow()
            .owner
            .upgrade()
            .expect("NetRef is unlinked from netlist");
        netlist.drives_an_output(self.clone())
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
    pub fn delete_uses(self) -> Result<Object<I>, String> {
        let netlist = self
            .netref
            .borrow()
            .owner
            .upgrade()
            .expect("NetRef is unlinked from netlist");
        netlist.delete_net_uses(self)
    }

    /// Replaces the uses of this circuit node in the netlist with another circuit node.
    /// Panics if either `self` or `other` is a multi-output circuit node.
    pub fn replace_uses_with(self, other: &Self) -> Result<Object<I>, String> {
        let netlist = self
            .netref
            .borrow()
            .owner
            .upgrade()
            .expect("NetRef is unlinked from netlist");
        netlist.replace_net_uses(self, other)
    }
}

impl<I> std::fmt::Display for NetRef<I>
where
    I: Instantiable,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.netref.borrow().object.fmt(f)
    }
}

impl<I> From<NetRef<I>> for DrivenNet<I>
where
    I: Instantiable,
{
    fn from(val: NetRef<I>) -> Self {
        if val.is_multi_output() {
            panic!("Cannot convert a multi-output netref to an output port");
        }
        DrivenNet::new(0, val)
    }
}

impl<I> From<&NetRef<I>> for DrivenNet<I>
where
    I: Instantiable,
{
    fn from(val: &NetRef<I>) -> Self {
        if val.is_multi_output() {
            panic!("Cannot convert a multi-output netref to an output port");
        }
        DrivenNet::new(0, val.clone())
    }
}

/// Facilitates mutable borrows to driver nets
pub struct MutBorrowReq<I: Instantiable> {
    from: NetRef<I>,
    ind: Net,
}

impl<I> MutBorrowReq<I>
where
    I: Instantiable,
{
    /// Creates a new mutable borrow request
    fn new(from: NetRef<I>, ind: Net) -> Self {
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
    pub fn borrow_mut_if(&self, f: impl Fn(&NetRef<I>) -> bool) -> Option<RefMut<Net>> {
        if f(&self.from) {
            Some(self.borrow_mut())
        } else {
            None
        }
    }
}

/// A netlist data structure
#[derive(Debug)]
pub struct Netlist<I>
where
    I: Instantiable,
{
    /// The name of the netlist
    name: String,
    /// The list of objects in the netlist, such as inputs, modules, and primitives
    objects: RefCell<Vec<NetRefT<I>>>,
    /// The list of operands that point to objects which are outputs
    outputs: RefCell<HashMap<Operand, Net>>,
}

/// Represent the input port of a primitive
#[derive(Debug, Clone)]
pub struct InputPort<I: Instantiable> {
    pos: usize,
    netref: NetRef<I>,
}

impl<I> InputPort<I>
where
    I: Instantiable,
{
    fn new(pos: usize, netref: NetRef<I>) -> Self {
        if pos >= netref.clone().unwrap().borrow().operands.len() {
            panic!(
                "Position {} out of bounds for netref with {} input nets",
                pos,
                netref.unwrap().borrow().get().get_nets().len()
            );
        }
        Self { pos, netref }
    }

    /// Returns the net that is driving this input port
    pub fn get_driver(&self) -> Option<DrivenNet<I>> {
        if self.netref.is_an_input() {
            panic!("Input port is not driven by a primitive");
        }
        if let Some(prev_operand) = self.netref.clone().unwrap().borrow().operands[self.pos].clone()
        {
            let netlist = self
                .netref
                .clone()
                .unwrap()
                .borrow()
                .owner
                .upgrade()
                .expect("Input port is unlinked from netlist");
            let driver_nr = netlist.index_weak(&prev_operand.root());
            let nr = NetRef::wrap(driver_nr);
            let pos = prev_operand.secondary();
            Some(DrivenNet::new(pos, nr))
        } else {
            None
        }
    }

    /// Disconnects an input port and returns the previous [DrivenNet] if it was connected.
    pub fn disconnect(&self) -> Option<DrivenNet<I>> {
        let val = self.get_driver();
        self.netref.clone().unwrap().borrow_mut().operands[self.pos] = None;
        val
    }

    /// Get the input port associated with this connection
    pub fn get_port(&self) -> Net {
        if self.netref.is_an_input() {
            panic!("Net is not driven by a primitive");
        }
        self.netref
            .get_instance_type()
            .unwrap()
            .get_input_port(self.pos)
            .clone()
    }

    /// Connects this input port to a driven net.
    pub fn connect(self, output: DrivenNet<I>) {
        output.connect(self);
    }

    /// Return the underlying circuit node
    pub fn unwrap(self) -> NetRef<I> {
        self.netref
    }
}

impl<I> std::fmt::Display for InputPort<I>
where
    I: Instantiable,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.get_port().fmt(f)
    }
}

/// Represent a net that is being driven by a [Instantiable]
#[derive(Debug, Clone)]
pub struct DrivenNet<I: Instantiable> {
    pos: usize,
    netref: NetRef<I>,
}

impl<I> DrivenNet<I>
where
    I: Instantiable,
{
    fn new(pos: usize, netref: NetRef<I>) -> Self {
        if pos >= netref.clone().unwrap().borrow().get().get_nets().len() {
            panic!(
                "Position {} out of bounds for netref with {} outputted nets",
                pos,
                netref.unwrap().borrow().get().get_nets().len()
            );
        }
        Self { pos, netref }
    }

    /// Returns the index that can address this net in the netlist.
    fn get_operand(&self) -> Operand {
        if self.netref.is_multi_output() {
            Operand::CellIndex(self.netref.clone().unwrap().borrow().get_index(), self.pos)
        } else {
            Operand::DirectIndex(self.netref.clone().unwrap().borrow().get_index())
        }
    }

    /// Borrow the net being driven
    pub fn get_net(&self) -> Ref<Net> {
        self.netref.get_net(self.pos)
    }

    /// Get a mutable reference to the net being driven
    pub fn get_net_mut(&self) -> RefMut<Net> {
        self.netref.get_net_mut(self.pos)
    }

    /// Returns `true` if this net is a principal input
    pub fn is_an_input(&self) -> bool {
        self.netref.is_an_input()
    }

    /// Get the output port associated with this connection
    pub fn get_port(&self) -> Net {
        if self.netref.is_an_input() {
            panic!("Net is not driven by a primitive");
        }
        self.netref
            .get_instance_type()
            .unwrap()
            .get_output_port(self.pos)
            .clone()
    }

    /// Connects the net driven by this output port to the given input port.
    pub fn connect(&self, input: InputPort<I>) {
        let operand = self.get_operand();
        let index = input.netref.unwrap().borrow().get_index();
        let netlist = self
            .netref
            .clone()
            .unwrap()
            .borrow()
            .owner
            .upgrade()
            .expect("Output port is unlinked from netlist");
        let obj = netlist.index_weak(&index);
        obj.borrow_mut().operands[input.pos] = Some(operand.clone());
    }

    /// Returns `true` if this net is a top-level output in the netlist.
    pub fn is_top_level_output(&self) -> bool {
        let netlist = self
            .netref
            .clone()
            .unwrap()
            .borrow()
            .owner
            .upgrade()
            .expect("DrivenNet is unlinked from netlist");
        let outputs = netlist.outputs.borrow();
        outputs.contains_key(&self.get_operand())
    }

    /// Return the underlying circuit node
    pub fn unwrap(self) -> NetRef<I> {
        self.netref
    }
}

impl<I> std::fmt::Display for DrivenNet<I>
where
    I: Instantiable,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.get_net().fmt(f)
    }
}

impl<I> WeakIndex<usize> for Netlist<I>
where
    I: Instantiable,
{
    type Output = OwnedObject<I, Self>;

    fn index_weak(&self, index: &usize) -> Rc<RefCell<Self::Output>> {
        self.objects.borrow()[*index].clone()
    }
}

impl<I> Netlist<I>
where
    I: Instantiable,
{
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

    /// Use interior mutability to add an object to the netlist. Returns a mutable reference to the created object.
    fn insert_object(
        self: &Rc<Self>,
        object: Object<I>,
        operands: &[DrivenNet<I>],
    ) -> Result<NetRef<I>, String> {
        let index = self.objects.borrow().len();
        let weak = Rc::downgrade(self);
        let operands = operands
            .iter()
            .map(|net| Some(net.get_operand()))
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
    pub fn insert_input(self: &Rc<Self>, net: Net) -> NetRef<I> {
        let obj = Object::Input(net);
        self.insert_object(obj, &[]).unwrap()
    }

    /// Inserts a four-state logic input port to the netlist
    pub fn insert_input_escaped_logic_bus(
        self: &Rc<Self>,
        net: String,
        bw: usize,
    ) -> Vec<NetRef<I>> {
        Net::new_escaped_logic_bus(net, bw)
            .into_iter()
            .map(|n| self.insert_input(n))
            .collect()
    }

    /// Inserts a gate to the netlist
    pub fn insert_gate(
        self: &Rc<Self>,
        inst_type: I,
        inst_name: Identifier,
        operands: &[DrivenNet<I>],
    ) -> Result<NetRef<I>, String> {
        let nets = inst_type
            .get_output_ports()
            .iter()
            .map(|pnet| pnet.with_name(format!("{}_{}", inst_name, pnet.get_identifier())))
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

    /// Use interior mutability to add an object to the netlist. Returns a mutable reference to the created object.
    pub fn insert_gate_disconnected(
        self: &Rc<Self>,
        inst_type: I,
        inst_name: Identifier,
    ) -> Result<NetRef<I>, String> {
        let nets = inst_type
            .get_output_ports()
            .iter()
            .map(|pnet| pnet.with_name(format!("{}_{}", inst_name, pnet.get_identifier())))
            .collect::<Vec<_>>();
        let object = Object::Instance(nets, inst_name, inst_type);
        let index = self.objects.borrow().len();
        let weak = Rc::downgrade(self);
        let operands = vec![None; object.get_instance_type().unwrap().get_input_ports().len()];
        let owned_object = Rc::new(RefCell::new(OwnedObject {
            object,
            owner: weak,
            operands,
            index,
        }));
        self.objects.borrow_mut().push(owned_object.clone());
        Ok(NetRef::wrap(owned_object))
    }

    /// Set an added object as a top-level output.
    /// Panics if `net`` is a multi-output node.
    pub fn expose_net_with_name(&self, net: DrivenNet<I>, name: String) -> DrivenNet<I> {
        let mut outputs = self.outputs.borrow_mut();
        outputs.insert(net.get_operand(), net.get_net().with_name(name));
        net
    }

    /// Set an added object as a top-level output.
    pub fn expose_net(&self, net: DrivenNet<I>) -> Result<DrivenNet<I>, String> {
        if net.is_an_input() {
            return Err(
                "Cannot expose an input net as output without a new name to bind to".to_string(),
            );
        }
        let mut outputs = self.outputs.borrow_mut();
        outputs.insert(net.get_operand(), net.get_net().clone());
        Ok(net)
    }

    /// Unlink a circuit node from the rest of the netlist. Return the object that was being stored.
    pub fn delete_net_uses(&self, netref: NetRef<I>) -> Result<Object<I>, String> {
        let unwrapped = netref.clone().unwrap();
        if Rc::strong_count(&unwrapped) > 3 {
            return Err("Cannot delete. References still exist on this node".to_string());
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
    /// Panics if `of` and  `with` are not single-output nodes.
    pub fn replace_net_uses(&self, of: NetRef<I>, with: &NetRef<I>) -> Result<Object<I>, String> {
        let unwrapped = of.clone().unwrap();
        if Rc::strong_count(&unwrapped) > 3 {
            return Err("Cannot replace. References still exist on this node".to_string());
        }

        let old_tag: DrivenNet<I> = of.clone().into();
        let old_index = old_tag.get_operand();
        let new_tag: DrivenNet<I> = with.clone().into();
        let new_index = new_tag.get_operand();
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
        } else if let Some(v) = self.outputs.borrow().get(&old_index) {
            self.outputs.borrow_mut().insert(new_index, v.clone());
            self.outputs.borrow_mut().remove(&old_index);
        }

        Ok(of.unwrap().borrow().get().clone())
    }
}

impl<I> Netlist<I>
where
    I: Instantiable,
{
    /// Returns the name of the netlist module
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Iterates over the input ports of the netlist.
    pub fn get_input_ports(&self) -> impl Iterator<Item = Net> {
        self.objects().filter_map(|oref| {
            if oref.is_an_input() {
                Some(oref.as_net().clone())
            } else {
                None
            }
        })
    }

    /// Returns a list of output nets
    pub fn get_output_ports(&self) -> Vec<Net> {
        self.outputs.borrow().values().cloned().collect::<Vec<_>>()
    }

    /// Constructs an analysis of the netlist.
    pub fn get_analysis<'a, A: Analysis<'a, I>>(&'a self) -> Result<A, String> {
        A::build(self)
    }

    /// Finds the first circuit node that drives the `net`. This operation is O(n).
    /// This should be unique provided the netlist is well-formed.
    pub fn find_net(&self, net: &Net) -> Option<DrivenNet<I>> {
        for obj in self.objects() {
            for o in obj.outputs() {
                if *o.get_net() == *net {
                    return Some(o);
                }
            }
        }
        None
    }

    /// Returns a `NetRef` to the first circuit node
    pub fn first(&self) -> Option<NetRef<I>> {
        self.objects
            .borrow()
            .first()
            .map(|nr| NetRef::wrap(nr.clone()))
    }

    /// Returns a `NetRef` to the last circuit node
    pub fn last(&self) -> Option<NetRef<I>> {
        self.objects
            .borrow()
            .last()
            .map(|nr| NetRef::wrap(nr.clone()))
    }

    /// Returns `true` if an output of `netref` which is driving a module output.
    pub fn drives_an_output(&self, netref: NetRef<I>) -> bool {
        let my_index = netref.unwrap().borrow().get_index();
        for key in self.outputs.borrow().keys() {
            if key.root() == my_index {
                return true;
            }
        }
        false
    }

    /// Cleans unused nodes from the netlist, returning `Ok(true)` if the netlist changed.
    pub fn clean_once(&self) -> Result<bool, String> {
        let mut dead_objs = HashSet::new();
        {
            let fan_out = self.get_analysis::<FanOutTable<I>>().unwrap();
            for obj in self.objects() {
                let mut is_dead = true;
                for net in obj.nets() {
                    // This should account for outputs
                    if fan_out.has_uses(&net) {
                        is_dead = false;
                        break;
                    }
                }
                if is_dead && !obj.is_an_input() {
                    dead_objs.insert(obj.unwrap().borrow().index);
                }
            }
        }

        if dead_objs.is_empty() {
            return Ok(false);
        }

        let old_objects = self.objects.take();
        let mut remap: HashMap<usize, usize> = HashMap::new();
        for (old_index, obj) in old_objects.into_iter().enumerate() {
            if dead_objs.contains(&old_index) {
                if Rc::strong_count(&obj) > 2 {
                    return Err(format!(
                        "Cannot delete object {} as a NetRef still exists, or it is an output. SC = {}",
                        obj.borrow().get(),
                        Rc::strong_count(&obj)
                    ));
                }
                continue;
            }
            let new_index = self.objects.borrow().len();
            remap.insert(old_index, new_index);
            obj.borrow_mut().index = new_index;
            self.objects.borrow_mut().push(obj);
        }

        for obj in self.objects.borrow().iter() {
            for operand in obj.borrow_mut().inds_mut() {
                let root = operand.root();
                let root = *remap.get(&root).unwrap_or(&root);
                *operand = operand.clone().remap(root);
            }
        }

        let pairs: Vec<_> = self.outputs.take().into_iter().collect();
        for (operand, net) in pairs {
            let root = operand.root();
            let root = *remap.get(&root).unwrap_or(&root);
            let new_operand = operand.clone().remap(root);
            self.outputs.borrow_mut().insert(new_operand, net);
        }

        Ok(true)
    }

    /// Greedly removes unused nodes from the netlist, until it stops changing.
    pub fn clean(&self) -> Result<(), String> {
        let mut changed = true;
        while changed {
            changed = self.clean_once()?;
        }
        Ok(())
    }

    /// Returns `true` if all the nets are uniquely named
    fn nets_unique(&self) -> bool {
        let mut nets = HashSet::new();
        for net in self.into_iter() {
            if !nets.insert(net.take_identifier()) {
                return false;
            }
        }
        true
    }

    /// Returns `true` if all the nets are uniquely named
    fn insts_unique(&self) -> bool {
        let mut insts = HashSet::new();
        for inst in self.objects() {
            if let Some(name) = inst.get_instance_name() {
                if !insts.insert(name) {
                    return false;
                }
            }
        }
        true
    }

    /// Verifies that a netlist is well-formed.
    pub fn verify(&self) -> Result<(), String> {
        if !self.nets_unique() {
            return Err("Netlist contains non-unique nets".to_string());
        }

        if !self.insts_unique() {
            return Err("Netlist contains non-unique instances".to_string());
        }
        Ok(())
    }
}

/// Represent a driven net alongside its connection to an input port
#[derive(Debug, Clone)]
pub struct Connection<I: Instantiable> {
    driver: DrivenNet<I>,
    input: InputPort<I>,
}

impl<I> Connection<I>
where
    I: Instantiable,
{
    fn new(driver: DrivenNet<I>, input: InputPort<I>) -> Self {
        Self { driver, input }
    }

    /// Return the driver of the connection
    pub fn src(&self) -> DrivenNet<I> {
        self.driver.clone()
    }

    /// Return the net along the connection
    pub fn net(&self) -> Net {
        self.driver.get_net().clone()
    }

    /// Returns the input port of the connection
    pub fn target(&self) -> InputPort<I> {
        self.input.clone()
    }
}

impl<I> std::fmt::Display for Connection<I>
where
    I: Instantiable,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.net().fmt(f)
    }
}

/// A collection of iterators for the netlist
pub mod iter {

    use super::{
        Connection, DrivenNet, InputPort, Instantiable, Net, NetRef, Netlist, Operand, WeakIndex,
    };
    use std::collections::HashSet;
    /// An iterator over the nets in a netlist
    pub struct NetIterator<'a, I: Instantiable> {
        netlist: &'a Netlist<I>,
        index: usize,
        subindex: usize,
    }

    impl<'a, I> NetIterator<'a, I>
    where
        I: Instantiable,
    {
        /// Creates a new iterator for the netlist
        pub fn new(netlist: &'a Netlist<I>) -> Self {
            Self {
                netlist,
                index: 0,
                subindex: 0,
            }
        }
    }

    impl<I> Iterator for NetIterator<'_, I>
    where
        I: Instantiable,
    {
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

    /// An iterator over the objects in a netlist
    pub struct ObjectIterator<'a, I: Instantiable> {
        netlist: &'a Netlist<I>,
        index: usize,
    }

    impl<'a, I> ObjectIterator<'a, I>
    where
        I: Instantiable,
    {
        /// Creates a new  object iterator for the netlist
        pub fn new(netlist: &'a Netlist<I>) -> Self {
            Self { netlist, index: 0 }
        }
    }

    impl<I> Iterator for ObjectIterator<'_, I>
    where
        I: Instantiable,
    {
        type Item = NetRef<I>;

        fn next(&mut self) -> Option<Self::Item> {
            if self.index < self.netlist.objects.borrow().len() {
                let objects = self.netlist.objects.borrow();
                let object = &objects[self.index];
                self.index += 1;
                return Some(NetRef::wrap(object.clone()));
            }
            None
        }
    }

    /// An iterator over the connections in a netlist
    pub struct ConnectionIterator<'a, I: Instantiable> {
        netlist: &'a Netlist<I>,
        index: usize,
        subindex: usize,
    }

    impl<'a, I> ConnectionIterator<'a, I>
    where
        I: Instantiable,
    {
        /// Create a new connection iterator for the netlist
        pub fn new(netlist: &'a Netlist<I>) -> Self {
            Self {
                netlist,
                index: 0,
                subindex: 0,
            }
        }
    }

    impl<I> Iterator for ConnectionIterator<'_, I>
    where
        I: Instantiable,
    {
        type Item = super::Connection<I>;

        fn next(&mut self) -> Option<Self::Item> {
            while self.index < self.netlist.objects.borrow().len() {
                let objects = self.netlist.objects.borrow();
                let object = objects[self.index].borrow();
                let noperands = object.operands.len();
                while self.subindex < noperands {
                    if let Some(operand) = &object.operands[self.subindex] {
                        let driver = match operand {
                            Operand::DirectIndex(idx) => {
                                DrivenNet::new(0, NetRef::wrap(objects[*idx].clone()))
                            }
                            Operand::CellIndex(idx, j) => {
                                DrivenNet::new(*j, NetRef::wrap(objects[*idx].clone()))
                            }
                        };
                        let input = InputPort::new(
                            self.subindex,
                            NetRef::wrap(objects[self.index].clone()),
                        );
                        self.subindex += 1;
                        return Some(Connection::new(driver, input));
                    }
                    self.subindex += 1;
                }
                self.subindex = 0;
                self.index += 1;
            }
            None
        }
    }

    /// A depth-first iterator over the circuit nodes in a netlist
    pub struct DFSIterator<'a, I: Instantiable> {
        netlist: &'a Netlist<I>,
        stack: Vec<NetRef<I>>,
        visited: HashSet<usize>,
    }

    impl<'a, I> DFSIterator<'a, I>
    where
        I: Instantiable,
    {
        /// Create a new DFS iterator for the netlist starting at `from`.
        pub fn new(netlist: &'a Netlist<I>, from: NetRef<I>) -> Self {
            Self {
                netlist,
                stack: vec![from],
                visited: HashSet::new(),
            }
        }
    }

    impl<I> Iterator for DFSIterator<'_, I>
    where
        I: Instantiable,
    {
        type Item = NetRef<I>;

        fn next(&mut self) -> Option<Self::Item> {
            if let Some(item) = self.stack.pop() {
                let uw = item.clone().unwrap();
                let index = uw.borrow().get_index();
                if !self.visited.insert(index) {
                    return self.next();
                }
                let operands = &uw.borrow().operands;
                for operand in operands.iter().flatten() {
                    self.stack
                        .push(NetRef::wrap(self.netlist.index_weak(&operand.root())));
                }
                return Some(item);
            }

            None
        }
    }
}

impl<'a, I> IntoIterator for &'a Netlist<I>
where
    I: Instantiable,
{
    type Item = Net;
    type IntoIter = iter::NetIterator<'a, I>;

    fn into_iter(self) -> Self::IntoIter {
        iter::NetIterator::new(self)
    }
}

/// Filter invariants of [Instantiable] in a netlist. Use it like you would `matches!`.
/// Example: ```filter_nodes!(netlist, Gate::AND(_));```
#[macro_export]
macro_rules! filter_nodes {
    ($netlist:ident, $pattern:pat $(if $guard:expr)? $(,)?) => {
        $netlist.matches(|f| match f {
            $pattern $(if $guard)? => true,
            _ => false
        })
    };
}

impl<I> Netlist<I>
where
    I: Instantiable,
{
    /// Returns an iterator over the circuit nodes in the netlist.
    pub fn objects(&self) -> impl Iterator<Item = NetRef<I>> {
        iter::ObjectIterator::new(self)
    }

    /// Returns an iterator over the circuit nodes that match the instance type.
    pub fn matches<F>(&self, filter: F) -> impl Iterator<Item = NetRef<I>>
    where
        F: Fn(&I) -> bool,
    {
        self.objects().filter(move |f| {
            if let Some(inst_type) = f.get_instance_type() {
                filter(&inst_type)
            } else {
                false
            }
        })
    }

    /// Returns an iterator to principal inputs in the netlist as references.
    pub fn inputs(&self) -> impl Iterator<Item = DrivenNet<I>> {
        self.objects()
            .filter(|n| n.is_an_input())
            .map(|n| DrivenNet::new(0, n))
    }

    /// Returns an iterator to circuit nodes that drive an output in the netlist.
    pub fn outputs(&self) -> Vec<(DrivenNet<I>, Net)> {
        self.outputs
            .borrow()
            .iter()
            .map(|(k, n)| {
                (
                    DrivenNet::new(k.secondary(), NetRef::wrap(self.index_weak(&k.root()))),
                    n.clone(),
                )
            })
            .collect()
    }

    /// Returns an iterator over the wire connections in the netlist.
    pub fn connections(&self) -> impl Iterator<Item = Connection<I>> {
        iter::ConnectionIterator::new(self)
    }

    /// Returns a depth-first search iterator over the nodes in the netlist.
    pub fn dfs(&self, from: NetRef<I>) -> impl Iterator<Item = NetRef<I>> {
        iter::DFSIterator::new(self, from)
    }
}

impl<I> std::fmt::Display for Netlist<I>
where
    I: Instantiable,
{
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
                    let port_name = inst_type.get_output_port(idx).get_identifier().emit_name();
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

#[test]
fn test_delete_netlist() {
    let netlist = Netlist::new("simple_example".to_string());

    // Add the the two inputs
    let input1 = netlist.insert_input("input1".into());
    let input2 = netlist.insert_input("input2".into());

    // Instantiate an AND gate
    let instance = netlist
        .insert_gate(
            Gate::new_logical(
                "AND".into(),
                vec!["A".to_string(), "B".to_string()],
                "Y".to_string(),
            ),
            "my_and".into(),
            &[input1.clone().into(), input2.clone().into()],
        )
        .unwrap();

    // Make this AND gate an output
    let instance = instance.expose_as_output().unwrap();
    instance.delete_uses().unwrap();
    let res = netlist.clean();
    assert!(res.is_ok());
}

/// A type alias for a netlist of gates
pub type GateNetlist = Netlist<Gate>;
/// A type alias to Gate circuit nodes
pub type GateRef = NetRef<Gate>;
