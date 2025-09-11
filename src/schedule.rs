/*!

  Allocate and schedule a netlist to a target architecture.

*/

use crate::circuit::Instantiable;
use crate::netlist::{Gate, NetRef};
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::marker::PhantomData;

type Time = u32;

/// An instantiable type of cell/primitive that can be scheduled
pub trait Schedulable: Instantiable + Debug + PartialEq + Eq + Hash {}

/// An operator that can be used to implement a operation
pub trait Operator: Clone + Debug + PartialEq + Eq + Hash {
    /// The latency of the operator in time units
    fn get_latency(&self) -> Time;

    /// The initiation interval of the operator in time units
    fn get_interval(&self) -> Time;
}

/// A library of operators to implement the [Schedulable] operations
pub trait OperatorLib<K: Schedulable, V: Operator> {
    /// Return the max number of concurrent instances of operator `v`
    fn get_max(&self, v: &V) -> Option<usize>;

    /// Return the operator implementing the schedulable `k` if it exists
    fn get_operator(&self, k: &K) -> Option<V>;
}

/// A schedule for a set of schedulable [Instantiable]s
pub struct Schedule<K: Schedulable> {
    /// Maps a netref to its (start, end) times
    sched: HashMap<NetRef<K>, (Time, Time)>,
    /// Keeps track of unscheduled operations
    unscheduled: HashSet<NetRef<K>>,
    /// The domain of operations to schedule
    opset: HashSet<NetRef<K>>,
}

impl<K: Schedulable> Display for Schedule<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut ops: Vec<_> = self.sched.iter().collect();
        ops.sort_by_key(|(_n, (s, e))| (s, e));
        for (n, (s, e)) in ops {
            writeln!(f, "{n} : ({s}, {e})")?;
        }
        for n in &self.unscheduled {
            writeln!(f, "{n} : UNSCHEDULED")?;
        }
        Ok(())
    }
}

impl<K: Schedulable> Schedule<K> {
    /// Create a new empty schedule
    pub fn new(opset: impl Iterator<Item = NetRef<K>>) -> Self {
        let opset: HashSet<NetRef<K>> = opset.collect();
        Self {
            sched: HashMap::new(),
            unscheduled: opset.clone(),
            opset,
        }
    }

    /// Get the start time of the schedulable `k`
    ///
    /// # Panics
    ///
    /// Panics if `op` is not in the original opset.
    pub fn get_start(&self, op: &NetRef<K>) -> Option<Time> {
        if !self.opset.contains(op) {
            panic!("Operation not in original opset");
        }

        self.sched.get(op).cloned().map(|x| x.0)
    }

    /// Get the end time of the schedulable `k`
    ///
    /// # Panics
    ///
    /// Panics if `op` is not in the original opset.
    pub fn get_end(&self, op: &NetRef<K>) -> Option<Time> {
        if !self.opset.contains(op) {
            panic!("Operation not in original opset");
        }

        self.sched.get(op).cloned().map(|x| x.1)
    }

    /// Returns true if all the operations have been scheduled
    pub fn fully_scheduled(&self) -> bool {
        self.unscheduled.is_empty()
    }

    /// Get the start and end time of the schedulable `k` as a tuple `(start, end)`
    ///
    /// # Panics
    ///
    /// Panics if `op` is not in the original opset.
    pub fn get_interval(&self, op: &NetRef<K>) -> Option<(Time, Time)> {
        if !self.opset.contains(op) {
            panic!("Operation not in original opset");
        }

        self.sched.get(op).cloned()
    }

    /// Returns true if the operation is scheduled
    ///
    /// # Panics
    ///
    /// Panics if `op` is not in the original opset.
    pub fn is_scheduled(&self, op: &NetRef<K>) -> bool {
        self.get_interval(op).is_some()
    }

    /// Returns true if the operation was previously unscheduled and is now scheduled
    ///
    /// # Panics
    ///
    /// Panics if `op` is not in the original opset.
    pub fn schedule_op(&mut self, op: NetRef<K>, start: Time, end: Time) -> bool {
        if !self.opset.contains(&op) {
            panic!("Operation not in original opset");
        }

        self.sched.insert(op.clone(), (start, end));
        self.unscheduled.remove(&op)
    }

    /// Clear the current schedule
    pub fn clear(&mut self) {
        self.sched.clear();
        self.opset.clear();
        self.unscheduled.clear();
    }

    /// Returns an iterator to operations that finished at `step`
    pub fn finished_at(&self, step: Time) -> impl Iterator<Item = &NetRef<K>> {
        self.sched.iter().filter_map(move |(n, i)| match i {
            (_, s) if *s == step => Some(n),
            _ => None,
        })
    }

    /// Returns an iterator to operations that finished after `step`
    pub fn finished_after(&self, step: Time) -> impl Iterator<Item = &NetRef<K>> {
        self.sched.iter().filter_map(move |(n, i)| match i {
            (_, s) if *s > step => Some(n),
            _ => None,
        })
    }

    /// Returns an iterator over all operations in the original set
    pub fn ops(&self) -> impl Iterator<Item = &NetRef<K>> {
        self.opset.iter()
    }

    /// Returns an iterator over all unscheduled operations
    pub fn unscheduled_ops(&self) -> impl Iterator<Item = &NetRef<K>> {
        self.unscheduled.iter()
    }

    /// Check dependency constraints
    pub fn check(&self) -> Result<(), String> {
        if !self.unscheduled.is_empty() {
            return Err("Not all ops scheduled".to_string());
        }

        for node in self.opset.iter() {
            let my_start = self.get_start(node);

            for pre_req in node.drivers().flatten() {
                if !self.opset.contains(&pre_req) {
                    return Err("Pre-req driver not in the opset".to_string());
                }

                let preq_end = self.get_end(&pre_req);
                if preq_end > my_start {
                    return Err("Dependency constraint not met".to_string());
                }
            }
        }

        Ok(())
    }
}

impl<K: Schedulable> FromIterator<NetRef<K>> for Schedule<K> {
    fn from_iter<T: IntoIterator<Item = NetRef<K>>>(iter: T) -> Self {
        Schedule::new(iter.into_iter())
    }
}

/// A trait that implements a schedule over an [OperatorLib]
pub trait Scheduler<K: Schedulable, V: Operator, L: OperatorLib<K, V>> {
    /// A scheduling error
    type Error;

    /// Schedule the operators without any constraints other than the dependencies and operator limits
    fn schedule(
        &mut self,
        ops: impl Iterator<Item = NetRef<K>>,
    ) -> Result<Schedule<K>, Self::Error>;
}

/// An operator to represent scheduling to identical instances
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IdenticalOperator;

impl Operator for IdenticalOperator {
    fn get_latency(&self) -> Time {
        1
    }

    fn get_interval(&self) -> Time {
        1
    }
}

/// An operator library to represent sequential scheduling
pub struct SequentialLib<K: Schedulable> {
    phantom: PhantomData<K>,
}

impl<K: Schedulable> Default for SequentialLib<K> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<K: Schedulable> OperatorLib<K, IdenticalOperator> for SequentialLib<K> {
    fn get_max(&self, _v: &IdenticalOperator) -> Option<usize> {
        Some(1)
    }

    fn get_operator(&self, _k: &K) -> Option<IdenticalOperator> {
        Some(IdenticalOperator)
    }
}

impl Schedulable for Gate {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{format_id, netlist::*};
    use std::rc::Rc;

    /// A greedy list scheduler
    struct ListScheduler<K: Schedulable, V: Operator, L: OperatorLib<K, V>> {
        lib: L,
        ready: HashSet<NetRef<K>>,
        unready: HashSet<NetRef<K>>,
        resource_table: HashMap<V, Option<usize>>,
    }

    impl<K: Schedulable, V: Operator, L: OperatorLib<K, V>> ListScheduler<K, V, L> {
        /// Create a new list scheduler
        fn new(lib: L) -> Self {
            Self {
                lib,
                ready: HashSet::new(),
                unready: HashSet::new(),
                resource_table: HashMap::new(),
            }
        }

        /// Initialize the resource table for all operations
        fn init_resources(&mut self) {
            self.resource_table.clear();

            for op in self.ready.iter().chain(self.unready.iter()) {
                if let Some(op) = op.get_instance_type() {
                    let v = self.lib.get_operator(&op);
                    match &v {
                        Some(v) => {
                            self.resource_table.insert(v.clone(), self.lib.get_max(v));
                        }
                        None => panic!("Operator library does not implement node"),
                    }
                }
            }
        }

        /// Update a specific resource type resource table
        fn update_resource(&mut self, v: &V, incr: bool) {
            if let Some(c) = self.resource_table.get_mut(v).unwrap() {
                if incr {
                    *c += 1;
                } else {
                    *c -= 1;
                }
            }
        }
    }

    impl<K: Schedulable, V: Operator, L: OperatorLib<K, V>> Scheduler<K, V, L>
        for ListScheduler<K, V, L>
    {
        type Error = String;

        fn schedule(
            &mut self,
            ops: impl Iterator<Item = NetRef<K>>,
        ) -> Result<Schedule<K>, Self::Error> {
            self.ready.clear();
            self.unready.clear();
            let mut ops: Vec<_> = ops.collect();
            ops.sort_by_key(|a| a.get_instance_type().is_some());
            let mut sched: Schedule<_> = ops.clone().into_iter().collect();

            // Initialize ready ops
            for op in ops {
                let mut ready = true;
                for dep in op.drivers().flatten() {
                    if !sched.is_scheduled(&dep) {
                        ready = false;
                        break;
                    }
                }

                if ready {
                    if op.get_instance_type().is_none() {
                        sched.schedule_op(op.clone(), 0, 0);
                    } else {
                        self.ready.insert(op.clone());
                    }
                } else {
                    self.unready.insert(op.clone());
                }
            }

            // Greedily schedule each time step
            let mut step: Time = 0;
            self.init_resources();
            while !sched.fully_scheduled() {
                // Replenish resources
                for fin in sched.finished_at(step) {
                    if let Some(k) = fin.get_instance_type() {
                        self.update_resource(&self.lib.get_operator(&*k).unwrap(), true);
                    }
                }

                // Schedule ops that are ready
                let mut just_scheduled = HashSet::new();
                for op in self.ready.clone() {
                    if let Some(k) = op.get_instance_type() {
                        let v = self
                            .lib
                            .get_operator(&*k)
                            .ok_or("No operator for op".to_string())?;
                        if v.get_interval() > 1 {
                            return Err(
                                "ListScheduler does not support initiation interval greater than 1"
                                    .to_string(),
                            );
                        }
                        let latency = v.get_latency();
                        if latency == 0 {
                            return Err("Sub-cycle scheduling not supported".to_string());
                        }
                        if self.resource_table[&v].unwrap_or(1) > 0 {
                            sched.schedule_op(op.clone(), step, step + latency);
                            self.update_resource(&v, false);
                            just_scheduled.insert(op.clone());
                        }
                    } else {
                        sched.schedule_op(op.clone(), step, step);
                        just_scheduled.insert(op.clone());
                    }
                }

                // Remove scheduled ops
                for op in &just_scheduled {
                    self.ready.remove(op);
                }

                if just_scheduled.is_empty()
                    && !self.ready.is_empty()
                    && sched.finished_after(step).next().is_none()
                {
                    return Err("Resources constraints impossible to satisfy".to_string());
                }

                // Ready new ops
                for op in &self.unready {
                    let mut ready = true;
                    for dep in op.drivers().flatten() {
                        if !sched.is_scheduled(&dep) {
                            ready = false;
                            break;
                        }
                    }

                    if ready {
                        self.ready.insert(op.clone());
                    }
                }

                // Remove newly readied ops
                for rdy in &self.ready {
                    self.unready.remove(rdy);
                }

                step += 1;
            }

            Ok(sched)
        }
    }

    fn full_adder() -> Gate {
        Gate::new_logical_multi(
            "FA".into(),
            vec!["CIN".into(), "A".into(), "B".into()],
            vec!["S".into(), "COUT".into()],
        )
    }

    fn ripple_adder() -> Rc<GateNetlist> {
        let netlist = Netlist::new("ripple_adder".to_string());
        let bitwidth = 4;

        // Add the the inputs
        let a = netlist.insert_input_escaped_logic_bus("a".to_string(), bitwidth);
        let b = netlist.insert_input_escaped_logic_bus("b".to_string(), bitwidth);
        let mut carry: DrivenNet<Gate> = netlist.insert_input("cin".into());

        for (i, (a, b)) in a.into_iter().zip(b.into_iter()).enumerate() {
            // Instantiate a full adder for each bit
            let fa = netlist
                .insert_gate(full_adder(), format_id!("fa_{i}"), &[carry, a, b])
                .unwrap();

            // Expose the sum
            fa.expose_net(&fa.get_net(0)).unwrap();

            carry = fa.find_output(&"COUT".into()).unwrap();

            if i == bitwidth - 1 {
                // Last full adder, expose the carry out
                fa.get_output(1).expose_with_name("cout".into()).unwrap();
            }
        }

        netlist
    }

    #[test]
    fn test_schedule_seq() {
        let nl = ripple_adder();
        let mut ls: ListScheduler<_, IdenticalOperator, SequentialLib<_>> =
            ListScheduler::new(SequentialLib::default());
        let schedule = ls.schedule(nl.objects()).unwrap();
        assert_eq!(schedule.to_string(), "cool".to_string());
    }
}
