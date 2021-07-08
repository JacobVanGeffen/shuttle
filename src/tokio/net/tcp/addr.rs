use std::net::{self, IpAddr};
use std::ops::{Add, Sub};
use std::sync::{Arc, Mutex};

use crate::runtime::execution::ExecutionState;
use crate::runtime::task::TaskId;

#[derive(Debug, Clone)]
/// TODO Document
/// This is an immutable wrapper around std::net::SocketAddr (in the sense that the inner SocketAddr is immutable through the wrapper).
/// Unlike std::net::SocketAddr, the only way to obtain SocketAddr
/// is by calling TcpStream::bind. the SocketAddr can then control the waking
/// of TcpListener::accept, as well as waking any TcpStream::read calls.
pub struct SocketAddr {
    addr: net::SocketAddr,
    listener: Arc<Mutex<Option<TaskId>>>,
    readers: Arc<Mutex<Vec<TaskId>>>,
    // TODO this just assumes that writes and reads are 1-1
    // TODO really, this should keep track of #bytes and stuff like that
    writers: Arc<Mutex<usize>>,
}

impl SocketAddr {
    pub(crate) fn new(addr: net::SocketAddr) -> SocketAddr {
        let listener = Arc::new(Mutex::new(None));
        let readers = Arc::new(Mutex::new(Vec::new()));
        let writers = Arc::new(Mutex::new(0usize));
        SocketAddr {
            addr,
            listener,
            readers,
            writers,
        }
    }

    pub(crate) fn from_old(addr: net::SocketAddr, old: SocketAddr) -> SocketAddr {
        println!("from_old with old {:?} and new {:?}", old.addr, addr);
        let listener = old.listener.clone();
        let readers = old.readers.clone();
        let writers = old.writers.clone();
        SocketAddr {
            addr,
            listener,
            readers,
            writers,
        }
    }

    // TODO call this when TcpListener::accept (on a pending)
    pub(crate) fn set_listener(&self) {
        let id = ExecutionState::with(|state| state.current().id());
        let mut listener = self.listener.lock().unwrap();
        *listener = Some(id);
    }

    // TODO call this when TcpStream::connect
    pub(crate) fn wake_listener(&self) {
        let mut listener = self.listener.lock().unwrap();
        if let Some(id) = listener.as_ref() {
            let waker = ExecutionState::with(|state| state.get(*id).waker());
            waker.wake_by_ref();
            *listener = None;
        }
    }

    // TODO call this when reading (on a pending)
    // Return true whenever the reader was added
    // Return false when there is already a writer registered (and thus the reader is not added)
    pub(crate) fn add_reader(&self) -> bool {
        println!("add_reader for {:?}", self.addr);
        let mut writers = self.writers.lock().unwrap();
        if writers.gt(&0usize) {
            println!("Decrementing writers");
            *writers = writers.sub(1usize);
            false
        } else {
            let id = ExecutionState::with(|state| state.current().id());
            let mut readers = self.readers.lock().unwrap();
            println!("Adding reader id: {:?}", id);
            readers.push(id);
            true
        }
    }

    // TODO call this when writing
    // TODO could name this something better b/c it doesn't always wake a reader
    pub(crate) fn wake_reader(&self) {
        println!("wake_reader for {:?}", self.addr);
        let mut readers = self.readers.lock().unwrap();
        // NOTE: This is just gonna pop everything
        println!("All the readers: {:?}", readers);
        if let Some(id) = readers.pop() {
            println!("Waking reader id: {:?}", id);
            let waker = ExecutionState::with(|state| state.get(id).waker());
            waker.wake_by_ref();
        } else {
            println!("Incrementing writers");
            let mut writers = self.writers.lock().unwrap();
            *writers = writers.add(1usize);
        }
    }

    pub(crate) fn get_addr(&self) -> net::SocketAddr {
        self.addr
    }

    /// TODO Document
    pub fn ip(&self) -> IpAddr {
        self.addr.ip()
    }

    /// TODO Document
    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    /// TODO Document
    pub fn is_ipv4(&self) -> bool {
        matches!(self.addr, net::SocketAddr::V4(_))
    }

    /// TODO Document
    pub fn is_ipv6(&self) -> bool {
        matches!(self.addr, net::SocketAddr::V6(_))
    }

    /// TODO Document
    pub fn matches_addr(&self, other: &net::SocketAddr) -> bool {
        println!("Other: {:?}", other);
        println!("Mine: {:?}", self.addr);
        self.addr.eq(other)
    }
}
