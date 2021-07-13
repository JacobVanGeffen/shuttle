//! Shuttle's implementation of [`tokio::net`].

use futures::channel::oneshot::Sender;
use std::net::{IpAddr, SocketAddr};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Mutex;

thread_local! {
    // TODO Need to remove stuff from this table on close (also from port counter)
    // TODO to handle multiple connections to the same listener, this should actually be mpsc
    pub(crate) static CONNECT_TABLE: Mutex<HashMap<SocketAddr, RefCell<Sender<(TcpStream, SocketAddr)>>>> = Mutex::new(HashMap::new());
    pub(crate) static PORT_COUNTER: Mutex<HashMap<IpAddr, u16>> = Mutex::new(HashMap::new());
}

mod listener;
#[allow(unused)]
mod registration;
mod stream;

pub use listener::TcpListener;
pub use stream::TcpStream;
