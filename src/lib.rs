#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs, unreachable_pub)]
/*!

`safety-net`

An experimental library for representing circuit netlists for EDA tool development.
Take a look at some [examples](https://github.com/matth2k/safety-net/tree/main/examples) and the [documentation](https://matth2k.github.io/safety-net/).

*/
#![doc = "## Simple Example\n```"]
#![doc = include_str!("../examples/simple.rs")]
#![doc = "\n```"]

pub mod attribute;
pub mod circuit;
pub mod error;
pub mod graph;
pub mod logic;
pub mod netlist;
mod util;
