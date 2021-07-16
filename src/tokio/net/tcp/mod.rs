//! Shuttle's implementation of [`tokio::net`].

use futures::channel::mpsc::UnboundedSender;
use std::net::{IpAddr, SocketAddr};
// use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Mutex;

thread_local! {
    // TODO Need to remove stuff from this table on close (also from port counter)
    pub(crate) static CONNECT_TABLE: Mutex<HashMap<SocketAddr, UnboundedSender<(TcpStream, SocketAddr)>>> = Mutex::new(HashMap::new());
    pub(crate) static PORT_COUNTER: Mutex<HashMap<IpAddr, u16>> = Mutex::new(HashMap::new());
}

mod listener;
#[allow(unused)]
mod registration;
mod stream;

pub use listener::TcpListener;
pub use stream::TcpStream;
