/*!

  Attributes and parameters for nets and node (gates) in the netlist.

*/

use bitvec::vec::BitVec;
use std::collections::{HashMap, HashSet};

use crate::{
    circuit::Instantiable,
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

#[derive(Debug, Clone)]
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
            Parameter::Integer(i) => write!(f, "{i}"),
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
    /// The mapping of netrefs that have this attribute
    map: HashMap<AttributeKey, HashSet<NetRef<I>>>,
    /// Contains a dedup collection of all filtered nodes
    full_set: HashSet<NetRef<I>>,
}

impl<'a, I> AttributeFilter<'a, I>
where
    I: Instantiable,
{
    /// Create a new filter for the netlist
    fn new(netlist: &'a Netlist<I>, keys: Vec<AttributeKey>) -> Self {
        let mut map = HashMap::new();
        let mut full_set = HashSet::new();
        for nr in netlist.objects() {
            for attr in nr.attributes() {
                if keys.contains(attr.key()) {
                    map.entry(attr.key().clone())
                        .or_insert_with(HashSet::new)
                        .insert(nr.clone());
                    full_set.insert(nr.clone());
                }
            }
        }
        Self {
            _netlist: netlist,
            keys,
            map,
            full_set,
        }
    }

    /// Check if an node matches any of the filter keys
    pub fn has(&self, n: &NetRef<I>) -> bool {
        self.map.values().any(|s| s.contains(n))
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

    type IntoIter = std::collections::hash_set::IntoIter<NetRef<I>>;

    fn into_iter(self) -> Self::IntoIter {
        self.full_set.into_iter()
    }
}

/// Returns a filtering of nodes and nets that are marked as 'dont_touch'
pub fn dont_touch_filter<'a, I>(netlist: &'a Netlist<I>) -> AttributeFilter<'a, I>
where
    I: Instantiable,
{
    AttributeFilter::new(netlist, vec!["dont_touch".to_string()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitvec::prelude::*;

    #[test]
    fn attribute_iter() {
        let attributes: [(AttributeKey, AttributeValue); 2] = [
            ("dont_touch".to_string(), Some("true".to_string())),
            ("synthesizable".to_string(), None),
        ];
        let real_attrs: Vec<Attribute> = Attribute::from_pairs(attributes.into_iter()).collect();
        assert_eq!(real_attrs.len(), 2);
        assert_eq!(
            real_attrs.first().unwrap().to_string(),
            "(* dont_touch = true *)"
        );
        assert_eq!(real_attrs.first().unwrap().key(), "dont_touch");
        assert_eq!(
            real_attrs.last().unwrap().to_string(),
            "(* synthesizable *)"
        );
        assert!(real_attrs.last().unwrap().value().is_none());
    }

    #[test]
    fn test_parameter_fmt() {
        let p1 = Parameter::Integer(42);
        // Lsb first
        let p2 = Parameter::BitVec(bitvec![0, 0, 0, 0, 0, 0, 0, 1]);
        assert_eq!(p1.to_string(), "42");
        assert_eq!(p2.to_string(), "8'b10000000");
    }
}
