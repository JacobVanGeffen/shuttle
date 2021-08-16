//! Shuttle's implementation of [`tokio::net`].

// TODO: OwnedReadHalf, OwnedWriteHalf

use futures::channel::mpsc::UnboundedSender;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Mutex;

thread_local! {
    // TODO Need to remove stuff from this table on close (also from port counter)
    pub(crate) static CONNECT_TABLE: Mutex<HashMap<SocketAddr, UnboundedSender<(TcpStream, SocketAddr)>>> = Mutex::new(HashMap::new());
    pub(crate) static PORT_COUNTER: Mutex<HashMap<IpAddr, u16>> = Mutex::new(HashMap::new());
}

/// Reset the global connection table. This hack is necessary because shuttle doesn't yet support `lazy_static`
pub fn reset_connect_table() {
    CONNECT_TABLE.with(|state| {
        let mut state = state.lock().unwrap();
        *state = HashMap::new();
    });
    PORT_COUNTER.with(|state| {
        let mut state = state.lock().unwrap();
        *state = HashMap::new();
    });
}

fn new_socket_addr<F>(used: F, ip: IpAddr) -> SocketAddr
where
    F: Fn(&SocketAddr) -> bool,
{
    PORT_COUNTER.with(|state| {
        let mut state = state.lock().unwrap();
        let mut port = match state.get(&ip) {
            Some(p) => *p,
            None => 1u16,
        };
        while used(&SocketAddr::new(ip, port)) {
            port = port + 1;
        }
        state.insert(ip, port + 1);
        SocketAddr::new(ip, port)
    })
}

mod listener;
mod stream;

pub use listener::TcpListener;
pub use stream::OwnedReadHalf;
pub use stream::OwnedWriteHalf;
pub use stream::TcpStream;
