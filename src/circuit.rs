/*!

  Types for the constructs found within a digital circuit.

*/

use crate::{attribute::Parameter, logic::Logic};

/// Signals in a circuit can be binary, tri-state, or four-state.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DataType {
    /// A logical 0 or 1
    TwoState,
    /// A logical 0, 1, or high-Z
    ThreeState,
    /// A logical 0, 1, high-Z, or unknown (X)
    FourState,
}

impl DataType {
    /// Returns the data type for bools (1'b0 and 1'b1)
    pub fn boolean() -> Self {
        DataType::TwoState
    }

    /// Returns the data type for tri-state signals (1'b0, 1'b1, and 1'bz)
    pub fn tristate() -> Self {
        DataType::ThreeState
    }

    /// Returns the data type for four-state signals (1'b0, 1'b1, 1'bz, and 1'bx)
    pub fn fourstate() -> Self {
        DataType::FourState
    }

    /// Returns the data type for four-state signals (1'b0, 1'b1, 1'bz, and 1'bx)
    pub fn logic() -> Self {
        DataType::FourState
    }
}

/// An identifier of a node in a circuit
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Identifier {
    /// The name of the identifier
    name: String,
    /// Is the identifier escaped
    escaped: bool,
    /// The bit index of the identifier, if it is part of a bus.
    idx: Option<usize>,
}

impl Identifier {
    /// Creates a new identifier with the given name
    pub fn new(name: String) -> Self {
        if name.is_empty() {
            panic!("Identifier name cannot be empty");
        }

        if let Some(root) = name.strip_prefix('\\') {
            return Identifier {
                name: root.to_string(),
                escaped: true,
                idx: None,
            };
        }

        // Check if first char is a digit
        let esc_chars = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
        if esc_chars.contains(&name.chars().next().unwrap()) {
            return Identifier {
                name,
                escaped: true,
                idx: None,
            };
        }

        // Certainly not an exhaustive list.
        // TODO(matth2k): Implement a true isEscaped()
        let esc_chars = [
            ' ', '\\', '(', ')', ',', '+', '-', '$', '\'', '~', ';', '.', ',', '?', '!',
        ];
        if name.chars().any(|c| esc_chars.contains(&c)) {
            return Identifier {
                name,
                escaped: true,
                idx: None,
            };
        }

        if name.contains('[') && name.ends_with(']') {
            let name_ind = name.find('[').unwrap();
            let rname = &name[..name_ind];
            let index_start = name_ind + 1;
            let slice = name[index_start..name.len() - 1].parse::<usize>();
            if let Ok(s) = slice {
                let id = Identifier::new(rname.to_string());
                if !id.is_sliced() {
                    return Identifier { idx: Some(s), ..id };
                }
            }
            return Identifier {
                name,
                escaped: true,
                idx: None,
            };
        }

        Identifier {
            name,
            escaped: false,
            idx: None,
        }
    }

    /// Add an index to the identifier
    ///
    /// # Panics
    ///
    /// if self has an index already
    pub fn with_index(self, index: usize) -> Self {
        if self.idx.is_some() {
            panic!("Cannot add an index to an identifier that already has one");
        }
        Identifier {
            idx: Some(index),
            ..self
        }
    }

    /// Returns a bus with length `bw`
    ///
    /// # Panics
    ///
    /// if `name` already includes slicing brackets
    pub fn new_bus(name: String, bw: usize) -> Vec<Self> {
        let mut vec = Vec::new();
        let id = Identifier::new(name.clone());
        if id.is_sliced() {
            panic!("Cannot create a bus from an identifier that is sliced by string");
        }
        for i in 0..bw {
            vec.push(Identifier {
                idx: Some(i),
                ..id.clone()
            });
        }
        vec
    }

    /// Returns the stem of the identifier
    pub fn get_stem(&self) -> Identifier {
        Identifier {
            name: self.name.clone(),
            escaped: self.escaped,
            idx: None,
        }
    }

    /// Returns the bit index, if the identifier is a bit-slice
    pub fn get_bit_index(&self) -> Option<usize> {
        self.idx
    }

    /// Returns `true` if the identifier is a slice of a wire bus
    pub fn is_sliced(&self) -> bool {
        self.idx.is_some()
    }

    /// The identifier is escaped, as defined by Verilog
    pub fn is_escaped(&self) -> bool {
        self.escaped
    }

    /// Emit the name as suitable for an HDL like Verilog. This takes into account bit-slicing and escaped identifiers
    pub fn emit_name(&self) -> String {
        let stem = match self.escaped {
            false => self.name.clone(),
            true => format!("\\{} ", self.name),
        };
        match self.idx {
            Some(i) => format!("{stem}[{i}]"),
            None => stem,
        }
    }
}

impl std::ops::Add for &Identifier {
    type Output = Identifier;

    fn add(self, rhs: Self) -> Identifier {
        let lname = self.name.as_str();
        let rname = rhs.name.as_str();
        let escaped = self.escaped || rhs.escaped;

        let new_name = match (self.idx, rhs.idx) {
            (Some(l), Some(r)) => {
                format!("{}_{}_{}_{}", lname, l, rname, r)
            }
            (Some(l), None) => format!("{}_{}_{}", lname, l, rname),
            (None, Some(r)) => format!("{}_{}_{}", lname, rname, r),
            _ => format!("{}_{}", lname, rname),
        };

        Identifier {
            name: new_name,
            escaped,
            idx: None,
        }
    }
}

impl std::ops::Add for Identifier {
    type Output = Identifier;

    fn add(self, rhs: Self) -> Identifier {
        &self + &rhs
    }
}

impl From<&str> for Identifier {
    fn from(name: &str) -> Self {
        Identifier::new(name.to_string())
    }
}

impl From<String> for Identifier {
    fn from(name: String) -> Self {
        Identifier::new(name)
    }
}

impl std::fmt::Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.escaped {
            write!(f, "\\")?;
        }
        write!(f, "{}", self.name)?;
        if self.escaped {
            write!(f, " ")?;
        }
        if let Some(idx) = self.idx {
            write!(f, "[{idx}]")?;
        }
        Ok(())
    }
}

/// A net in a circuit, which is identified with a name and data type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Net {
    identifier: Identifier,
    data_type: DataType,
}

impl Net {
    /// Creates a new net with the given identifier and data type
    pub fn new(identifier: Identifier, data_type: DataType) -> Self {
        Self {
            identifier,
            data_type,
        }
    }

    /// Create a new net for SystemVerilog-like four-state logic
    pub fn new_logic(name: Identifier) -> Self {
        Self::new(name, DataType::logic())
    }

    /// Create a wire bus
    pub fn new_logic_bus(name: String, bw: usize) -> Vec<Self> {
        let ids = Identifier::new_bus(name, bw);
        ids.into_iter()
            .map(|id| Self::new(id, DataType::logic()))
            .collect()
    }

    /// Sets the identifier of the net
    pub fn set_identifier(&mut self, identifier: Identifier) {
        self.identifier = identifier;
    }

    /// Returns the full identifier to the net
    pub fn get_identifier(&self) -> &Identifier {
        &self.identifier
    }

    /// Returns the full identifier to the net
    pub fn take_identifier(self) -> Identifier {
        self.identifier
    }

    /// Returns the data type of the net
    pub fn get_type(&self) -> &DataType {
        &self.data_type
    }

    /// Returns a net of the same type but with a different [Identifier].
    pub fn with_name(&self, name: Identifier) -> Self {
        Self::new(name, self.data_type)
    }
}

/// Functions like the [format!] macro, but returns an [Identifier]
#[macro_export]
macro_rules! format_id {
    ($($arg:tt)*) => {
        $crate::Identifier::new(format!($($arg)*))
    }
}

impl std::fmt::Display for Net {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.identifier.fmt(f)
    }
}

impl From<&str> for Net {
    fn from(name: &str) -> Self {
        Net::new_logic(name.into())
    }
}

/// A trait for primitives in a digital circuit, such as gates or other components.
pub trait Instantiable: Clone {
    /// Returns the name of the primitive
    fn get_name(&self) -> &Identifier;

    /// Returns the input ports of the primitive
    fn get_input_ports(&self) -> impl IntoIterator<Item = &Net>;

    /// Returns the output ports of the primitive
    fn get_output_ports(&self) -> impl IntoIterator<Item = &Net>;

    /// Returns `true` if the type intakes a parameter with this name.
    fn has_parameter(&self, id: &Identifier) -> bool;

    /// Returns the parameter value for the given key, if it exists.
    fn get_parameter(&self, id: &Identifier) -> Option<Parameter>;

    /// Returns the old parameter value for the given key, if it existed.
    fn set_parameter(&mut self, id: &Identifier, val: Parameter) -> Option<Parameter>;

    /// Returns an iterator over the parameters of the primitive.
    fn parameters(&self) -> impl Iterator<Item = (Identifier, Parameter)>;

    /// Creates the primitive used to represent a constant value, like VDD or GND.
    /// If the implementer does not support the specific constant, `None` is returned.
    fn from_constant(val: Logic) -> Option<Self>;

    /// Returns the constant value represented by this primitive, if it is constant.
    fn get_constant(&self) -> Option<Logic>;

    /// Returns 'true' if the primitive is sequential.
    fn is_seq(&self) -> bool;

    /// Returns `true` if the primitive is parameterized (has at least one parameter).
    fn is_parameterized(&self) -> bool {
        self.parameters().next().is_some()
    }

    /// Returns the single output port of the primitive.
    fn get_single_output_port(&self) -> &Net {
        let mut iter = self.get_output_ports().into_iter();
        let ret = iter.next().expect("Primitive has no output ports");
        if iter.next().is_some() {
            panic!("Primitive has more than one output port");
        }
        ret
    }

    /// Returns the output port at the given index.
    /// # Panics
    ///
    /// If the index is out of bounds.
    fn get_output_port(&self, index: usize) -> &Net {
        self.get_output_ports()
            .into_iter()
            .nth(index)
            .expect("Index out of bounds for output ports")
    }

    /// Returns the input port at the given index.
    /// # Panics
    ///
    /// If the index is out of bounds.
    fn get_input_port(&self, index: usize) -> &Net {
        self.get_input_ports()
            .into_iter()
            .nth(index)
            .expect("Index out of bounds for output ports")
    }

    /// Returns the index of the input port with the given identifier, if it exists.
    /// **This method should be overriden if the implemenation is capable of O(1) lookup.**
    fn find_input(&self, id: &Identifier) -> Option<usize> {
        self.get_input_ports()
            .into_iter()
            .position(|n| n.get_identifier() == id)
    }

    /// Returns the index of the output port with the given identifier, if it exists.
    /// **This method should be overriden if the implemenation is capable of O(1) lookup.**
    fn find_output(&self, id: &Identifier) -> Option<usize> {
        self.get_output_ports()
            .into_iter()
            .position(|n| n.get_identifier() == id)
    }

    /// Returns `true` if the primitive has no input ports. In most cases, this means the cell represents a constant.
    /// **This method should be overriden if the implemenation of `get_input_ports()` is expensive.**
    fn is_driverless(&self) -> bool {
        self.get_input_ports().into_iter().next().is_none()
    }
}

/// A tagged union for objects in a digital circuit, which can be either an input net or an instance of a module or primitive.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Object<I>
where
    I: Instantiable,
{
    /// A principal input to the circuit
    Input(Net),
    /// An instance of a module or primitive
    Instance(Vec<Net>, Identifier, I),
}

impl<I> Object<I>
where
    I: Instantiable,
{
    /// Returns the net driven by this object.
    pub fn get_single_net(&self) -> &Net {
        match self {
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

    /// Returns the net driven by this object at the index
    pub fn get_net(&self, index: usize) -> &Net {
        match self {
            Object::Input(net) => {
                if index > 0 {
                    panic!("Index out of bounds for input net.")
                }
                net
            }
            Object::Instance(nets, _, _) => &nets[index],
        }
    }

    /// Returns the instance within the object, if the object represents one
    pub fn get_instance_type(&self) -> Option<&I> {
        match self {
            Object::Input(_) => None,
            Object::Instance(_, _, instance) => Some(instance),
        }
    }

    /// Returns a mutable reference to the instance type within the object, if the object represents one
    pub fn get_instance_type_mut(&mut self) -> Option<&mut I> {
        match self {
            Object::Input(_) => None,
            Object::Instance(_, _, instance) => Some(instance),
        }
    }

    /// Returns all the nets driven at this circuit node.
    pub fn get_nets(&self) -> &[Net] {
        match self {
            Object::Input(net) => std::slice::from_ref(net),
            Object::Instance(nets, _, _) => nets,
        }
    }

    /// Returns a mutable reference to all the nets driven at this circuit node.
    pub fn get_nets_mut(&mut self) -> &mut [Net] {
        match self {
            Object::Input(net) => std::slice::from_mut(net),
            Object::Instance(nets, _, _) => nets,
        }
    }
}

impl<I> std::fmt::Display for Object<I>
where
    I: Instantiable,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Object::Input(net) => write!(f, "Input({net})"),
            Object::Instance(_nets, name, instance) => {
                write!(f, "{}({})", instance.get_name(), name)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifier_parsing() {
        let id = Identifier::new("wire".to_string());
        assert!(!id.is_escaped());
        assert!(!id.is_sliced());
        assert!(id.get_bit_index().is_none());
        let id = Identifier::new("\\wire".to_string());
        assert!(id.is_escaped());
        assert!(!id.is_sliced());
        let id = Identifier::new("wire[3]".to_string());
        assert!(!id.is_escaped());
        assert!(id.is_sliced());
        assert_eq!(id.get_bit_index(), Some(3));
    }

    #[test]
    fn assume_escaped_identifier() {
        let id = Identifier::new("C++".to_string());
        assert!(id.is_escaped());
    }

    #[test]
    fn identifier_emission() {
        let id = Identifier::new("wire".to_string());
        assert_eq!(id.emit_name(), "wire");
        let id = Identifier::new("\\wire".to_string());
        assert!(id.is_escaped());
        assert_eq!(id.emit_name(), "\\wire ");
        assert_eq!(format!("{id}"), "\\wire ");
        let id = Identifier::new("wire[3]".to_string());
        assert!(id.is_sliced());
        assert_eq!(id.emit_name(), "wire[3]");
    }

    #[test]
    fn test_implicits() {
        let net: Net = "hey".into();
        assert_ne!(*net.get_type(), DataType::boolean());
        assert_ne!(*net.get_type(), DataType::tristate());
        assert_eq!(*net.get_type(), DataType::logic());
        assert_eq!(*net.get_type(), DataType::fourstate());
    }
}
