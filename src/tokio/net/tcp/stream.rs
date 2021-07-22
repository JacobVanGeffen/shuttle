use crate::runtime::execution::ExecutionState;

use super::{SocketAddr, CONNECT_TABLE, PORT_COUNTER};
use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::future::poll_fn;
use futures::SinkExt;
use futures::StreamExt;
use std::fmt;
use std::io;
use std::net::IpAddr;
use std::pin::Pin;
use std::sync::Mutex;
use std::task::Context;
use std::task::Poll;
use std::time::Duration;
use tokio::io::ReadBuf;
use tokio::io::{AsyncRead, AsyncWrite};

/// TODO Document
pub struct TcpStream {
    addr: SocketAddr,
    peer: SocketAddr,
    inner: Mutex<Inner>,
}

struct Inner {
    sender: UnboundedSender<u8>,
    receiver: UnboundedReceiver<u8>,
    written_before_yield: Option<usize>,
}

impl TcpStream {
    /// TODO Document
    pub async fn connect(addr: SocketAddr) -> io::Result<TcpStream> {
        poll_fn(|cx| TcpStream::poll_connect(addr, cx)).await
    }

    fn poll_connect(addr: SocketAddr, cx: &mut Context<'_>) -> Poll<io::Result<TcpStream>> {
        CONNECT_TABLE.with(|state| {
            let mut state = state.lock().unwrap();
            // NOTE: This might not be used if sender is none, but we need to keep the mutable sender reference.
            // NOTE: Since we need an immutable reference to state to get peer, make the peer first
            let peer = TcpStream::new_socket_addr(|s| state.contains_key(s), addr.ip());
            let sender = state.get_mut(&addr);
            match sender {
                None => Poll::Pending,
                Some(sender) => {
                    // TODO is this clone necessary?
                    let sender_ready = sender.clone().poll_ready_unpin(cx);
                    match sender_ready {
                        Poll::Pending => Poll::Pending,
                        Poll::Ready(_) => {
                            // NOTE: Data channels are mpsc, whereas the connection channels are oneshot
                            // TODO: the streams should actually get two channels (one sender/receiver each)
                            // TODO: I think these channels should have infinite capacity?
                            let (other_sender, my_receiver) = unbounded::<u8>();
                            let (my_sender, other_receiver) = unbounded::<u8>();
                            // TODO handle when this is an error
                            let _ = sender.start_send((TcpStream::new(peer, addr, other_sender, other_receiver), peer));
                            Poll::Ready(Ok(TcpStream::new(addr, peer, my_sender, my_receiver)))
                        }
                    }
                }
            }
        })
    }

    fn new(addr: SocketAddr, peer: SocketAddr, sender: UnboundedSender<u8>, receiver: UnboundedReceiver<u8>) -> TcpStream {
        let inner = Mutex::new(Inner { sender, receiver, written_before_yield: None});
        TcpStream { addr, peer, inner}
    }

    fn new_socket_addr<F>(used: F, ip: IpAddr) -> SocketAddr
    where
        F: Fn(&SocketAddr) -> bool,
    {
        PORT_COUNTER.with(|state|{
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

    // look at futures::io::ReadHalf?
    /// TODO Document
    pub fn split<'a>(&'a mut self) -> (tokio::net::tcp::ReadHalf<'a>, tokio::net::tcp::WriteHalf<'a>) {
        unimplemented!()
    }

    /// Sets the behavior of the stream after being closed
    pub fn set_linger(&self, _: Option<Duration>) -> io::Result<()>{
        // TODO in reality, this should do something b/c this will change the behavior
        // .    of the stream after the write portion closes
        // Don't do anything because we don't model time
        // TODO Should this use shuttle random to return Err?
        Ok(())
    }
}

impl AsyncRead for TcpStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let mut inner =  self.inner.lock().unwrap();
        let receiver = &mut inner.receiver;
        let mut written = false;
        loop {
            match receiver.poll_next_unpin(cx) {
                // TODO might want to assert that the buffer is not full on the first two branches
                Poll::Pending => 
                    if !written {
                        return Poll::Pending;
                    } else {
                        return Poll::Ready(Ok(()));
                    },
                Poll::Ready(Some(x)) => {
                    written = true;
                    buf.put_slice(&[x; 1]);
                    if buf.remaining() == 0 {
                        return Poll::Ready(Ok(()));
                    }
                }
                Poll::Ready(None) => {
                    return Poll::Ready(Ok(()));
                },
            }
        }
    }
}

impl AsyncWrite for TcpStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let mut inner = self.inner.lock().unwrap();
        let written = inner.written_before_yield;
        let buf = if written.is_some() {
            &buf[written.unwrap()..]
        } else {
            buf
        };
        let sender = &mut inner.sender;
        // TODO don't make this a loop b/c it only ever does one or zero iterations
        for i in buf {
            let res = sender.poll_ready_unpin(cx);
            match res {
                Poll::Pending =>
                    if written.is_none() {
                        return Poll::Pending;
                    } else {
                        return Poll::Ready(Ok(written.unwrap()));
                    },
                Poll::Ready(Ok(_)) => {
                    let res = sender.start_send(*i);
                    if res.is_err() {
                        return Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, "SendError during start_send")));
                    }
                    (*inner).written_before_yield = match written {
                        None => Some(1),
                        Some(x) => Some(x + 1),
                    };
                    // Perform a yield
                    cx.waker().wake_by_ref();
                    ExecutionState::request_yield();
                    return Poll::Pending;
                },
                Poll::Ready(Err(_e)) =>
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::Other,
                        "Sending over the data channel failed",
                    ))),
            }
        }
        (*inner).written_before_yield = None;
        Poll::Ready(Ok(written.unwrap()))
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
        f.debug_tuple("").field(&self.addr).field(&self.peer).finish()
    }
}
