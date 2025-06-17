/*!

  Attributes and parameters for nets and node (gates) in the netlist.

*/

use bitvec::vec::BitVec;

/// A Verilog attribute assigned to a net or gate in the netlist: (* dont_touch *)
pub type AttributeKey = String;
/// A Verilog attribute can be assigned a string value: bitvec = (* dont_touch = true *)
pub type AttributeValue = Option<String>;

/// A dedicated type to parameters for instantiables
pub enum Parameter {
    /// An integer parameter
    Integer(i32),
    /// A floating-point parameter
    Real(f32),
    /// A bit vector parameter, like for a truth table
    BitVec(BitVec),
}
