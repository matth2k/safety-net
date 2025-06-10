#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs, unreachable_pub)]
/*!

`circuit`

TODO: overview, tutorial, testing, research papers

*/
#![doc = "## Simple Example\n```"]
#![doc = include_str!("../examples/simple.rs")]
#![doc = "\n```"]
#![doc = "If your Instantiable type can pattern match, then you can even match on the netlist:
```
for node in filter_nodes!(netlist, Gate::And(_, _)) {
    println!(\"Found AND gate: {}\", node);
}
```"]
pub mod circuit;
pub mod graph;
pub mod netlist;
