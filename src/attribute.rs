/*!

  Attributes and parameters for nets and node (gates) in the netlist.

*/

use bitvec::vec::BitVec;
use std::collections::{HashMap, HashSet};

use crate::{
    circuit::{Identifier, Instantiable},
    netlist::{NetRef, Netlist},
};

/// A Verilog attribute assigned to a net or gate in the netlist: (* dont_touch *)
pub type AttributeKey = String;
/// A Verilog attribute can be assigned a string value: bitvec = (* dont_touch = true *)
pub type AttributeValue = Option<String>;

#[derive(Debug, Clone, PartialEq, Eq)]
/// An attribute can add information to instances and wires in string form, like 'dont_touch'
pub struct Attribute {
    k: AttributeKey,
    v: AttributeValue,
}

impl Attribute {
    /// Create a new attribute pair
    pub fn new(k: AttributeKey, v: AttributeValue) -> Self {
        Self { k, v }
    }

    /// Get the key of the attribute
    pub fn key(&self) -> &AttributeKey {
        &self.k
    }

    /// Get the value of the attribute
    pub fn value(&self) -> &AttributeValue {
        &self.v
    }

    /// Map a attribute key-value pairs to the Attribute struct
    pub fn from_pairs(
        iter: impl Iterator<Item = (AttributeKey, AttributeValue)>,
    ) -> impl Iterator<Item = Self> {
        iter.map(|(k, v)| Self::new(k, v))
    }
}

impl std::fmt::Display for Attribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(value) = &self.v {
            write!(f, "(* {} = {} *)", self.k, value)
        } else {
            write!(f, "(* {} *)", self.k)
        }
    }
}

/// A dedicated type to parameters for instantiables
pub enum Parameter {
    /// An integer parameter
    Integer(i32),
    /// A floating-point parameter
    Real(f32),
    /// A bit vector parameter, like for a truth table
    BitVec(BitVec),
}

impl std::fmt::Display for Parameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Parameter::Integer(i) => write!(f, "{}", i),
            Parameter::Real(_r) => todo!(),
            Parameter::BitVec(bv) => write!(
                f,
                "{}'b{}",
                bv.len(),
                bv.iter()
                    .rev()
                    .map(|b| if *b { '1' } else { '0' })
                    .collect::<String>()
            ),
        }
    }
}

/// Filter nodes/nets in the netlist by some attribute, like "dont_touch"
pub struct AttributeFilter<'a, I: Instantiable> {
    // A reference to the underlying netlist
    _netlist: &'a Netlist<I>,
    // The keys to filter by
    keys: Vec<AttributeKey>,
    // The set of identifiers that have this attribute
    set: HashSet<Identifier>,
    /// The mapping of netrefs that have this attribute
    map: HashMap<Identifier, NetRef<I>>,
}

impl<'a, I> AttributeFilter<'a, I>
where
    I: Instantiable,
{
    /// Create a new filter for the netlist
    fn new(netlist: &'a Netlist<I>, keys: Vec<AttributeKey>) -> Self {
        let mut set = HashSet::new();
        let mut map = HashMap::new();
        for nr in netlist.objects() {
            for attr in nr.attributes() {
                if keys.contains(attr.key()) {
                    if let Some(inst) = nr.get_instance_name() {
                        set.insert(inst.clone());
                        map.insert(inst.clone(), nr.clone());
                    }
                    for net in nr.nets() {
                        set.insert(net.get_identifier().clone());
                    }
                }
            }
        }
        Self {
            _netlist: netlist,
            keys,
            set,
            map,
        }
    }

    /// Check if an identifier has one of the attributes
    pub fn has(&self, id: &Identifier) -> bool {
        self.set.contains(id)
    }

    /// Return a slice to the keys that were used for filtering
    pub fn keys(&self) -> &[AttributeKey] {
        &self.keys
    }
}

impl<'a, I> IntoIterator for AttributeFilter<'a, I>
where
    I: Instantiable,
{
    type Item = NetRef<I>;

    type IntoIter = std::collections::hash_map::IntoValues<Identifier, NetRef<I>>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_values()
    }
}

/// Returns a filtering of nodes and nets that are marked as 'dont_touch'
pub fn dont_touch_filter<'a, I>(netlist: &'a Netlist<I>) -> AttributeFilter<'a, I>
where
    I: Instantiable,
{
    AttributeFilter::new(netlist, vec!["dont_touch".to_string()])
}
