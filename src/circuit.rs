/*!

  TODO: circuit trait docs

*/

/// Signals in a circuit can be binary, tri-state, or four-state.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
        Self {
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
            IdentifierType::Escaped => format!("{} ", self.name),
        }
    }
}

/// A net in a circuit, which is identified with a name and data type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Net {
    indentifier: Identifier,
    data_type: DataType,
}

impl Net {
    /// Creates a new net with the given identifier and data type
    pub fn new(identifier: Identifier, data_type: DataType) -> Self {
        Self {
            indentifier: identifier,
            data_type,
        }
    }

    /// Create a new net for SystemVerilog-like four-state logic
    pub fn new_logic(name: String) -> Self {
        Self::new(Identifier::new(name), DataType::logic())
    }

    /// Returns the name of the net
    pub fn get_name(&self) -> &str {
        self.indentifier.get_name()
    }

    /// Returns the full identifier to the net
    pub fn get_identifier(&self) -> &Identifier {
        &self.indentifier
    }

    /// Returns the data type of the net
    pub fn get_type(&self) -> &DataType {
        &self.data_type
    }

    /// Returns a net of the same type but with a different name
    pub fn with_name(&self, name: String) -> Self {
        Self::new(Identifier::new(name), self.data_type.clone())
    }
}

/// A trait for primitives in a digital circuit, such as gates or other components.
pub trait Instantiable {
    /// Returns the name of the primitive
    fn get_name(&self) -> &str;

    /// Returns the input ports of the primitive
    fn get_input_ports(&self) -> &[Net];

    /// Returns the output ports of the primitive
    fn get_output_ports(&self) -> &[Net];

    /// Returns the single output port of the primitive.
    fn get_single_output_port(&self) -> &Net {
        if self.get_output_ports().len() > 1 {
            panic!("Primitive has more than one output port");
        }
        self.get_input_ports().first().unwrap()
    }

    /// Returns the output port at the given index.
    fn get_output_port_at(&self, index: usize) -> &Net {
        &self.get_output_ports()[index]
    }

    /// Returns the input port of the primitive at index `index`.
    fn get_input_port_at(&self, index: usize) -> &Net {
        &self.get_input_ports()[index]
    }
}

/// A tagged union for objects in a digital circuit, which can be either an input net or an instance of a module or primitive.
#[derive(Debug)]
pub enum Object<I>
where
    I: Instantiable,
{
    /// A principal input to the circuit
    Input(Net),
    /// An instance of a module or primitive
    Instance(Vec<Net>, String, I),
}

impl<I> Object<I>
where
    I: Instantiable,
{
    /// Returns the net driven by this object.
    pub fn get_single_output(&self) -> &Net {
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

    /// Returns the net driven by this object as index
    pub fn get_output_at(&self, index: usize) -> &Net {
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
}
