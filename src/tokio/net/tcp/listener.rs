use super::CONNECT_TABLE;
use futures::channel::mpsc::{unbounded, UnboundedReceiver};
use futures::StreamExt;
//use crate::runtime::task::TaskId;
use super::TcpStream;
//use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;
use std::io;
use std::net::{self, SocketAddr};
use std::sync::Mutex;
use std::task::{Context, Poll};

// static REGISTRATION: HashMap<SocketAddr, TaskId>;

/// TODO Document
pub struct TcpListener {
    receiver: Mutex<UnboundedReceiver<(TcpStream, SocketAddr)>>,
    addr: SocketAddr,
}

#[allow(unused)]
impl TcpListener {
    /// TODO Document
    pub async fn bind(addr: &str) -> io::Result<TcpListener> {
        // This should be blocking? Should I return pending first?
        let addr = addr.parse().expect("Unable to parse socket address");
        match TcpListener::bind_addr(addr) {
            Ok(listener) => Ok(listener),
            Err(e) => Err(e),
        }
    }

    /// TODO Document
    fn bind_addr(addr: SocketAddr) -> io::Result<TcpListener> {
        let (sender, receiver) = unbounded::<(TcpStream, SocketAddr)>();
        CONNECT_TABLE.with(|state| state.lock().unwrap().insert(addr, sender));
        Ok(TcpListener {
            receiver: Mutex::new(receiver),
            addr,
        })
    }

    /// TODO Document
    pub async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        futures::future::poll_fn(|cx| self.poll_accept(cx)).await
    }

    /// TODO Document
    fn poll_accept(&self, cx: &mut Context<'_>) -> Poll<io::Result<(TcpStream, SocketAddr)>> {
        let mut receiver = self.receiver.lock().unwrap();
        let result = receiver.poll_next_unpin(cx);
        match result {
            Poll::Pending => Poll::Pending,
            // TODO actually use e
            // Poll::Ready(Err(e)) => Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, "Channel cancelled"))),
            Poll::Ready(None) => Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, "Channel received unexpected end of file"))),
            Poll::Ready(Some(pair)) => Poll::Ready(Ok(pair)),
        }
    }

    /// TODO Document
    pub fn from_std(_listener: net::TcpListener) -> io::Result<TcpListener> {
        // TODO
        unimplemented!()
    }

    /// TODO Document
    pub fn into_std(self) -> io::Result<std::net::TcpListener> {
        // TODO
        unimplemented!()
    }

    /// TODO Document
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(self.addr)
    }
}

impl TryFrom<net::TcpListener> for TcpListener {
    type Error = io::Error;

    fn try_from(stream: net::TcpListener) -> Result<Self, Self::Error> {
        Self::from_std(stream)
    }
}

impl fmt::Debug for TcpListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.addr.fmt(f)
    }
}
