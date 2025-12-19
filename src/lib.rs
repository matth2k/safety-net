#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs, unreachable_pub)]
/*!

`safety-net`

An experimental library for representing circuit netlists for EDA tool development.
Take a look at some [examples](https://github.com/matth2k/safety-net/tree/main/examples) and the [documentation](https://matth2k.github.io/safety-net/).

The most important API is the [Netlist](https://matth2k.github.io/safety-net/safety_net/netlist/struct.Netlist.html) struct.

*/
#![doc = "## Simple Example\n```"]
#![doc = include_str!("../examples/simple.rs")]
#![doc = "\n```"]

mod attribute;
mod circuit;
mod error;
mod graph;
mod logic;
mod netlist;
mod util;

#[cfg(feature = "derive")]
pub use inst_derive::Instantiable;

pub use {attribute::*, circuit::*, error::*, graph::*, logic::*, netlist::*};
