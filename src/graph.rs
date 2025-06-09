/*!

  Graph utils for the `graph` module.

*/

use crate::circuit::Net;
use crate::netlist::{NetRef, Netlist};
use std::collections::{HashMap, HashSet};

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
    /// Contains nets which are outputs
    is_an_output: HashSet<Net>,
}

impl FanOutTable {
    /// Returns an iterator to the circuit nodes that use `net`.
    pub fn get_users(&self, net: &Net) -> impl Iterator<Item = NetRef> {
        self.fan_out
            .get(net)
            .into_iter()
            .flat_map(|users| users.iter().cloned())
    }

    /// Returns `true` if the net has any used by any cells in the circuit
    /// This does incude nets that are only used as outputs.
    pub fn has_uses(&self, net: &Net) -> bool {
        (self.fan_out.contains_key(net) && !self.fan_out.get(net).unwrap().is_empty())
            || self.is_an_output.contains(net)
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

        let mut is_an_output: HashSet<Net> = HashSet::new();
        for output in netlist.get_output_ports() {
            is_an_output.insert(output.clone());
        }

        Ok(FanOutTable {
            fan_out,
            is_an_output,
        })
    }
}
