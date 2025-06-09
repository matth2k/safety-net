/*!

  Graph utils for the `graph` module.

*/

use crate::circuit::Net;
use crate::netlist::{NetRef, Netlist};
use std::collections::HashMap;

/// A common trait of analyses than can be performed on a netlist.
pub trait Analysis
where
    Self: Sized,
{
    /// Construct the analysis
    fn build(netlist: &Netlist) -> Result<Self, String>;
}

/// A table that maps nets to the circuit nodes they drive
pub struct FanOutTable {
    // Maps a net to the list of nets it drives
    fan_out: HashMap<Net, Vec<NetRef>>,
}

impl FanOutTable {
    /// Returns an iterator to the circuit nodes that use `net`.
    pub fn get_users(&self, net: &Net) -> impl Iterator<Item = NetRef> {
        self.fan_out
            .get(net)
            .into_iter()
            .flat_map(|users| users.iter().cloned())
    }
}

impl Analysis for FanOutTable {
    fn build(netlist: &Netlist) -> Result<Self, String> {
        let mut fan_out: HashMap<Net, Vec<NetRef>> = HashMap::new();

        for (net, nr) in netlist.connections() {
            if fan_out.contains_key(&net) {
                fan_out.get_mut(&net).unwrap().push(nr);
            } else {
                fan_out.insert(net.clone(), vec![nr]);
            }
        }

        Ok(FanOutTable { fan_out })
    }
}
