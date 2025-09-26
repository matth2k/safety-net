/*!

  Error types.

*/

use thiserror::Error;

use crate::circuit::{Identifier, Net};

/// Errors for the `safety-net` library.
#[derive(Error, Debug)]
pub enum Error {
    /// Error for an analysis cannot run due to  cycles.
    #[error("Cycles detected along nets {0:?}")]
    CycleDetected(Vec<Net>),
    /// Errors in parsing literals/identifiers.
    #[error("Parsing error `{0}`")]
    ParseError(String),
    /// The labeled nets in the netlist are not unique.
    #[error("Non-unique nets: {0:?}")]
    NonuniqueNets(Vec<Net>),
    /// The labeled instances in the netlist are not unique.
    #[error("Non-unique instances: {0:?}")]
    NonuniqueInsts(Vec<Identifier>),
    /// The netlist has no outputs.
    #[error("No outputs in netlist")]
    NoOutputs,
    /// An error in the instantiable interface
    #[error("Error in the instantiable interface: {0}")]
    InstantiableError(String),
    /// A deletion would cause a dangling reference.
    #[error("Attempted to create a dangling reference to nets {0:?}")]
    DanglingReference(Vec<Net>),
    /// Mismatch in number of arguments
    #[error("Expected {0} arguments, got {1}")]
    ArgumentMismatch(usize, usize),
    /// An input needs an alias to be an output
    #[error("Input net {0} needs an alias to be an output")]
    InputNeedsAlias(Net),
    /// A net that was expected but not found
    #[error("Expected to find net {0} in netlist")]
    NetNotFound(Net),
}
