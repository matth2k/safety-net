#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs, unreachable_pub)]
/*!

`circuit`

TODO: overview, tutorial, testing, research papers

*/
#![doc = "## Simple Example\n```"]
#![doc = include_str!("../examples/simple.rs")]
#![doc = "\n```"]

pub mod attribute;
pub mod circuit;
pub mod graph;
pub mod netlist;
