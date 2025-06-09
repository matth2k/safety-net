/*!

  Graph utils for the `graph` module.

*/

use crate::circuit::Net;
use crate::netlist::{NetRef, Netlist};
use std::collections::{HashMap, HashSet};

/// A common trait of analyses than can be performed on a netlist.
pub trait Analysis<'a>
where
    Self: Sized + 'a,
{
    /// Construct the analysis
    fn build(netlist: &'a Netlist) -> Result<Self, String>;
}

/// A table that maps nets to the circuit nodes they drive
pub struct FanOutTable<'a> {
    // A reference to the underlying netlist
    _netlist: &'a Netlist,
    // Maps a net to the list of nets it drives
    fan_out: HashMap<Net, Vec<NetRef>>,
    /// Contains nets which are outputs
    is_an_output: HashSet<Net>,
}

impl FanOutTable<'_> {
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

impl<'a> Analysis<'a> for FanOutTable<'a> {
    fn build(netlist: &'a Netlist) -> Result<Self, String> {
        let mut fan_out: HashMap<Net, Vec<NetRef>> = HashMap::new();
        let mut is_an_output: HashSet<Net> = HashSet::new();

        for (net, nr) in netlist.connections() {
            if fan_out.contains_key(&net) {
                fan_out.get_mut(&net).unwrap().push(nr);
            } else {
                fan_out.insert(net.clone(), vec![nr]);
            }
        }

        for output in netlist.get_output_ports() {
            is_an_output.insert(output.clone());
        }

        for nr in netlist.objects() {
            if let Some(idx) = netlist.drives_an_output(nr.clone()) {
                is_an_output.insert(nr.get_net(idx).clone());
            }
        }

        Ok(FanOutTable {
            _netlist: netlist,
            fan_out,
            is_an_output,
        })
    }
}
