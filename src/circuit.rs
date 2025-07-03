/*!

  Types for the constructs found within a digital circuit.

*/

use crate::attribute::Parameter;

/// Signals in a circuit can be binary, tri-state, or four-state.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
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

/// The type of identifier labelling a circuit node
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IdentifierType {
    /// A normal identifier
    Normal,
    /// An identifier that is part of a wire bus
    BitSlice(usize),
    /// An identifier that is escaped, as defined by Verilog
    Escaped,
}

/// An identifier of a node in a circuit
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    /// The name of the identifier
    name: String,
    /// The type of identiefier
    id_type: IdentifierType,
}

impl Identifier {
    /// Creates a new identifier with the given name
    pub fn new(name: String) -> Self {
        if let Some(root) = name.strip_prefix('\\') {
            return Identifier {
                name: root.to_string(),
                id_type: IdentifierType::Escaped,
            };
        }

        // Certainly not an exhaustive list.
        // TODO(matth2k): Implement isEscaped()
        let esc_chars = ['[', ']', ' ', '\\', '(', ')', ',', '+', '-'];
        if name.chars().any(|c| esc_chars.contains(&c)) {
            return Identifier {
                name,
                id_type: IdentifierType::Escaped,
            };
        }

        Identifier {
            name,
            id_type: IdentifierType::Normal,
        }
    }

    /// Returns the name of the identifier
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Returns the bit index, if the identifier is a bit-slice
    pub fn get_bit_index(&self) -> Option<usize> {
        match self.id_type {
            IdentifierType::BitSlice(index) => Some(index),
            _ => None,
        }
    }

    /// Returns `true` if the identifier is a slice of a wire bus
    pub fn is_sliced(&self) -> bool {
        matches!(self.id_type, IdentifierType::BitSlice(_))
    }

    /// The identifier is escaped, as defined by Verilog
    pub fn is_escaped(&self) -> bool {
        matches!(self.id_type, IdentifierType::Escaped)
    }

    /// Emit the name as suitable for an HDL like Verilog. This takes into account bit-slicing and escaped identifiers
    pub fn emit_name(&self) -> String {
        match &self.id_type {
            IdentifierType::Normal => self.name.clone(),
            IdentifierType::BitSlice(index) => format!("{}[{}]", self.name, index),
            IdentifierType::Escaped => format!("\\{} ", self.name),
        }
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
        match &self.id_type {
            IdentifierType::Normal => write!(f, "{}", self.name),
            IdentifierType::BitSlice(index) => write!(f, "{}[{}]", self.name, index),
            IdentifierType::Escaped => write!(f, "\\{} ", self.name),
        }
    }
}

/// A net in a circuit, which is identified with a name and data type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    pub fn new_logic(name: String) -> Self {
        Self::new(Identifier::new(name), DataType::logic())
    }

    /// Create a wire bus as escaped SystemVerilog signals
    pub fn new_escaped_logic_bus(name: String, bw: usize) -> Vec<Self> {
        let mut vec: Vec<Self> = Vec::with_capacity(bw);
        for i in 0..bw {
            vec.push(Self::new(
                Identifier {
                    name: format!("{}[{}]", name, i),
                    id_type: IdentifierType::Escaped,
                },
                DataType::logic(),
            ));
        }
        vec
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

    /// Returns a net of the same type but with a different [IdentifierType::Normal] name
    pub fn with_name(&self, name: String) -> Self {
        Self::new(Identifier::new(name), self.data_type)
    }
}

impl std::fmt::Display for Net {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.identifier.fmt(f)
    }
}

impl From<&str> for Net {
    fn from(name: &str) -> Self {
        Net::new_logic(name.to_string())
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

    /// Returns an iterator over the parameters of the primitive.
    fn parameters(&self) -> impl Iterator<Item = (Identifier, Parameter)>;

    /// Returns `true` if the primitive is parameterized (has at least one parameter).
    fn is_parameterized(&self) -> bool {
        self.parameters().next().is_some()
    }

    /// Returns the single output port of the primitive.
    fn get_single_output_port(&self) -> &Net {
        self.get_input_ports()
            .into_iter()
            .next()
            .expect("Primitive has no output ports")
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
}

/// A tagged union for objects in a digital circuit, which can be either an input net or an instance of a module or primitive.
#[derive(Debug, Clone)]
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
            Object::Input(net) => write!(f, "Input({})", net),
            Object::Instance(_nets, name, instance) => {
                write!(f, "{}({})", instance.get_name(), name)
            }
        }
    }
}
