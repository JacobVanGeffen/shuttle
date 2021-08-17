use super::{SocketAddr, new_socket_addr, CONNECT_TABLE};
use shuttle::{asynch, thread};
use shuttle::sync::{Arc, Mutex};
use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::future::poll_fn;
use futures::SinkExt;
use futures::StreamExt;
use std::fmt;
use std::io::{self, IoSlice, IoSliceMut};
use std::net::ToSocketAddrs;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::time::Duration;
use tokio::io::{AsyncReadExt, Interest, ReadBuf};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

/// TODO Document
pub struct TcpStream {
    addr: SocketAddr,
    peer: SocketAddr,
    inner: Arc<Mutex<Inner>>,
}

// TODO read/write half should also have a mutex to inner
/// TODO Document
#[derive(Debug)]
pub struct OwnedReadHalf {
    inner: Arc<Mutex<Inner>>,
}

/// TODO Document
#[derive(Debug)]
pub struct OwnedWriteHalf {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Debug)]
struct Inner {
    sender: UnboundedSender<u8>,
    receiver: UnboundedReceiver<u8>,
    written_before_yield: Option<usize>,
}

impl TcpStream {
    /// TODO Document
    pub async fn connect<A: ToSocketAddrs>(addrs: A) -> io::Result<TcpStream> {
        // TODO is this the correct place to yield?
        asynch::yield_now();
        // NOTE: The first address should be successful
        // TODO could loop over the addrs
        poll_fn(|cx| TcpStream::poll_connect(addrs.to_socket_addrs().unwrap().next().unwrap(), cx)).await
    }

    // TODO actually allow these to work with poll_ready_unpin to see if stuff is ready
    pub async fn readable(&self) -> io::Result<()> { Ok(()) }
    pub async fn writable(&self) -> io::Result<()> { Ok(()) }
    pub async fn ready(&self, _i: Interest) -> io::Result<()> { Ok(()) }
    
    pub fn try_write(&'static mut self, src: &'static [u8]) -> io::Result<usize> {
        asynch::block_on(self.write(src))
    }

    pub fn try_write_vectored(&'static mut self, bufs: &'static [IoSlice<'static>]) -> io::Result<usize> {
        asynch::block_on(self.write_vectored(bufs))
    }

    pub fn try_read(&'static mut self, buf: &'static mut [u8]) -> io::Result<usize> {
        asynch::block_on(self.read(buf))
    }

    pub fn try_read_vectored(&'static mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        Ok(0)
        // TODO
        // asynch::block_on(self.read_vectored(bufs))
    }

    pub async fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        let mut total = 0;
        for buf in bufs {
            match self.read(buf).await {
                Ok(n) => total += n,
                Err(e) => return Err(e),
            }
        }
        Ok(total)
    }

    fn poll_connect(peer: SocketAddr, cx: &mut Context<'_>) -> Poll<io::Result<TcpStream>> {
        CONNECT_TABLE.with(|state| {
            let mut state = state.lock().unwrap();
            // NOTE: This might not be used if sender is none, but we need to keep the mutable sender reference.
            // NOTE: Since we need an immutable reference to state to get peer, make the peer first
            let addr = new_socket_addr(|s| state.contains_key(s), peer.ip());
            let sender = state.get_mut(&peer);
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

    fn new(
        addr: SocketAddr,
        peer: SocketAddr,
        sender: UnboundedSender<u8>,
        receiver: UnboundedReceiver<u8>,
    ) -> TcpStream {
        let inner = Mutex::new(Inner {
            sender,
            receiver,
            written_before_yield: None,
        });
        TcpStream {
            addr,
            peer,
            inner: Arc::new(inner),
        }
    }

    /// TODO Document
    pub fn into_split(self) -> (OwnedReadHalf, OwnedWriteHalf) {
        (
            OwnedReadHalf {
                inner: self.inner.clone(),
            },
            OwnedWriteHalf {
                inner: self.inner.clone(),
            },
        )
    }

    /// Sets the behavior of the stream after being closed
    pub fn set_linger(&self, _: Option<Duration>) -> io::Result<()> {
        // TODO in reality, this should do something b/c this will change the behavior
        // .    of the stream after the write portion closes
        // Don't do anything because we don't model time
        // TODO Should this use shuttle random to return Err?
        // unimplemented!()
        Ok(())
    }

    /// Get own SocketAddr
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Get peer SocketAddr
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer
    }
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        //println!("Stream at {:?} being dropped", self.local_addr());
    }
}

impl AsyncRead for TcpStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let mut inner = self.inner.lock().unwrap();
        inner.poll_read(cx, buf)
    }
}

impl AsyncWrite for TcpStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let mut inner = self.inner.lock().unwrap();
        inner.poll_write(cx, buf)
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

impl AsyncRead for OwnedReadHalf {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let mut inner = self.inner.lock().unwrap();
        inner.poll_read(cx, buf)
    }
}

impl AsyncWrite for OwnedWriteHalf {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let mut inner = self.inner.lock().unwrap();
        inner.poll_write(cx, buf)
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

impl Inner {
    fn poll_read(&mut self, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let receiver = &mut self.receiver;
        let mut written = false;
        loop {
            let res = receiver.poll_next_unpin(cx);
            match res {
                // TODO might want to assert that the buffer is not full on the first two branches
                Poll::Pending => {
                    if !written {
                        return Poll::Pending;
                    } else {
                        return Poll::Ready(Ok(()));
                    }
                }
                Poll::Ready(Some(x)) => {
                    written = true;
                    buf.put_slice(&[x; 1]);
                    if buf.remaining() == 0 {
                        return Poll::Ready(Ok(()));
                    }
                }
                Poll::Ready(None) => {
                    return Poll::Ready(Ok(()));
                }
            }
        }
    }

    fn poll_write(&mut self, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let written = self.written_before_yield;
        let buf = if written.is_some() {
            &buf[written.unwrap()..]
        } else {
            buf
        };
        let sender = &mut self.sender;
        // TODO don't make this a loop b/c it only ever does one or zero iterations
        for i in buf {
            let written = self.written_before_yield;
            let res = sender.poll_ready_unpin(cx);
            match res {
                Poll::Pending => {
                    if written.is_none() {
                        return Poll::Pending;
                    } else {
                        return Poll::Ready(Ok(written.unwrap()));
                    }
                }
                Poll::Ready(Ok(_)) => {
                    let res = sender.start_send(*i);
                    if res.is_err() {
                        return Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, "SendError during start_send")));
                    }
                    self.written_before_yield = match written {
                        None => Some(1),
                        Some(x) => Some(x + 1),
                    };
                    // Perform a yield
                    // TODO uncomment
                    // TODO why is the receiver disconnecting?
                    //cx.waker().wake_by_ref();
                    //thread::request_yield();
                    //return Poll::Pending;
                }
                Poll::Ready(Err(e)) => {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::Other,
                        "Sending over the data channel failed",
                    )))
                }
            }
        }
        let written = self.written_before_yield;
        self.written_before_yield = None;
        Poll::Ready(Ok(written.unwrap()))
    }
}

impl fmt::Debug for TcpStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("").field(&self.addr).field(&self.peer).finish()
    }
}