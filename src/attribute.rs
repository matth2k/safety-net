/*!

  Attributes and parameters for nets and node (gates) in the netlist.

*/

use bitvec::vec::BitVec;

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
