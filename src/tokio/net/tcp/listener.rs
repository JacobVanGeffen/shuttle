//use crate::runtime::task::TaskId;
use super::SocketAddr;
use super::TcpStream;
//use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt;
use std::io;
use std::net;
use std::task::{Context, Poll};

// static REGISTRATION: HashMap<SocketAddr, TaskId>;

/// TODO Document
pub struct TcpListener {
    io: net::TcpListener,
    addr: SocketAddr,
}

#[allow(unused)]
impl TcpListener {
    /// TODO Document
    pub async fn bind(addr: &str) -> io::Result<TcpListener> {
        // This should be blocking? Should I return pending first?
        let addr: net::SocketAddr = addr.parse().expect("Unable to parse socket address");
        match TcpListener::bind_addr(addr) {
            Ok(listener) => Ok(listener),
            Err(e) => Err(e),
        }
    }

    /// TODO Document
    fn bind_addr(addr: net::SocketAddr) -> io::Result<TcpListener> {
        let listener = net::TcpListener::bind(addr)?;
        listener.set_nonblocking(true).expect("Cannot set non-blocking");
        TcpListener::new(listener, addr)
    }

    /// TODO Document
    pub async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        futures::future::poll_fn(|cx| self.poll_accept(cx)).await
    }

    /// TODO Document
    fn poll_accept(&self, cx: &mut Context<'_>) -> Poll<io::Result<(TcpStream, SocketAddr)>> {
        println!("Poll accept");
        let accept = self.io.accept();
        match accept {
            Ok(accept) => {
                println!("poll_accept Ready");
                let (stream, addr) = accept;
                let addr = SocketAddr::from_old(addr, self.addr.clone());
                let stream = TcpStream::new(stream, addr.clone());
                Poll::Ready(Ok((stream, addr.clone())))
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                println!("poll_accept Pending");
                // TODO Should also register the desired socket addr and task ID
                // TODO Will this (technically different) future have the same task ID as the original?
                // TODO For now, the hack will be to just wake ourselves
                self.addr.clone().set_listener();
                Poll::Pending
            }
            Err(e) => {
                println!("here instead");
                Poll::Ready(Err(e))
            }
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
    pub(crate) fn new(io: net::TcpListener, addr: net::SocketAddr) -> io::Result<TcpListener> {
        let addr = SocketAddr::new(addr);
        Ok(TcpListener { io, addr })
    }

    /// TODO Document
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        let addr = self.io.local_addr();
        assert!(addr.is_ok());
        let addr = addr.unwrap();
        assert!(self.addr.matches_addr(&addr));
        Ok(self.addr.clone())
    }

    /// TODO Document
    pub fn ttl(&self) -> io::Result<u32> {
        self.io.ttl()
    }

    /// TODO Document
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.io.set_ttl(ttl)
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
        self.io.fmt(f)
    }
}
