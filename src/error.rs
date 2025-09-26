/*!

  Error types.

*/

use thiserror::Error;

use crate::circuit::Net;

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
    /// The netlist has no outputs.
    #[error("No outputs in netlist")]
    NoOutputs,
    /// A deletion would cause a dangling reference.
    #[error("Attempted to create a dangling reference to nets {0:?}")]
    DanglingReference(Vec<Net>),
}
