use crate::runtime::task::TaskId;
use scoped_tls::scoped_thread_local;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;

scoped_thread_local! {
    static REGISTRATION: RefCell<Registration>
}

pub(crate) struct Registration {
    waiters: HashMap<SocketAddr, TaskId>,
    sockets: HashSet<SocketAddr>,
}

impl Registration {
    /// Construct a new execution that will use the given scheduler. The execution should then be
    /// invoked via its `run` method, which takes as input the closure for task 0.
    pub(crate) fn new() -> Self {
        Self {
            waiters: HashMap::new(),
            sockets: HashSet::new(),
        }
    }

    // pub(crate) fn
}
