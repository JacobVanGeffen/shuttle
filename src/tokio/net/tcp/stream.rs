use std::fmt;
// use futures::future::Future;
// use std::collections::HashMap;
use super::SocketAddr;
use crate::sync::Mutex;
use std::io;
use std::io::Read;
use std::io::Write;
use std::net;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use tokio::io::ReadBuf;
use tokio::io::{AsyncRead, AsyncWrite};

// const ACTIVE_ADDRS: HashMap<&str, TcpStream> = HashMap::new();

/// TODO Document
pub struct TcpStream {
    inner: Mutex<Inner>,
}

struct Inner {
    stream: net::TcpStream,
    addr: SocketAddr,
    reader_has_waited: bool,
    // TODO need anything else in here?
}

impl TcpStream {
    /// TODO Document
    pub async fn connect(addr: SocketAddr) -> io::Result<TcpStream> {
        addr.wake_listener();
        let stream = net::TcpStream::connect(addr.get_addr())?;
        Ok(TcpStream::new(stream, addr))
    }

    pub(super) fn new(stream: net::TcpStream, addr: SocketAddr) -> TcpStream {
        stream.set_nonblocking(false).expect("Cannot set non-blocking");
        TcpStream {
            inner: Mutex::new(Inner { stream, addr, reader_has_waited: false }),
        }
    }

    // look at futures::io::ReadHalf?
    /// TODO Document
    pub fn split<'a>(&'a mut self) -> (tokio::net::tcp::ReadHalf<'a>, tokio::net::tcp::WriteHalf<'a>) {
        unimplemented!()
    }
}

impl AsyncRead for TcpStream {
    fn poll_read(self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        println!("Poll read");
        // TODO for now, wait until we can fill the whole buffer
        let mut inner = self.inner.lock().unwrap();
        let has_waited = inner.reader_has_waited;
        let addr = inner.addr.clone();
        let stream = &mut inner.stream;
        if !has_waited && addr.add_reader() {
            inner.reader_has_waited = true;
            return Poll::Pending;
        }
        // Safety: TODO
        let b = unsafe { &mut *(buf.unfilled_mut() as *mut [std::mem::MaybeUninit<u8>] as *mut [u8]) };
        let result = stream.read(b);
        match result {
            Ok(n) => {
                println!("poll_read Ready");
                // Safety: TODO
                unsafe {
                    buf.assume_init(n);
                };
                buf.advance(n);
                Poll::Ready(Ok(()))
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                println!("poll_read Pending");
                addr.add_reader();
                Poll::Pending
            }
            Err(e) => Poll::Ready(Err(e)),
        }
        // }
    }
}

impl AsyncWrite for TcpStream {
    fn poll_write(self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        println!("Poll write");
        let mut inner = self.inner.lock().unwrap();
        let addr = inner.addr.clone();
        let stream = &mut inner.stream;
        // NOTE: This should be a blocking call
        // TODO do I need to wake up waiters here? probably
        let res = stream.write(buf);
        match res {
            Ok(n) => {
                println!("poll_write Ready");
                // (*inner).data = inner.data.add(n);
                addr.wake_reader();
                Poll::Ready(Ok(n))
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                panic!("Write would block");
                // Poll::Pending
            }
            Err(e) => {
                println!("poll_write Ready (with error)");
                addr.wake_reader();
                Poll::Ready(Err(e))
            }
        }
    }

    #[inline]
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        // tcp flush is a no-op
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        // TODO
        Poll::Ready(Ok(()))
    }
}

impl fmt::Debug for TcpStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = self.inner.lock().unwrap();
        inner.stream.fmt(f)
    }
}
