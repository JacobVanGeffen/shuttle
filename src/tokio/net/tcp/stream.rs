use super::{SocketAddr, CONNECT_TABLE, PORT_COUNTER};
use futures::channel::mpsc::{channel, Receiver, Sender};
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
use tokio::io::ReadBuf;
use tokio::io::{AsyncRead, AsyncWrite};

/// TODO Document
pub struct TcpStream {
    addr: SocketAddr,
    peer: SocketAddr,
    inner: Mutex<Inner>,
}

struct Inner {
    sender: Sender<u8>,
    receiver: Receiver<u8>,
}

impl TcpStream {
    /// TODO Document
    pub async fn connect(addr: SocketAddr) -> io::Result<TcpStream> {
        poll_fn(|cx| TcpStream::poll_connect(addr, cx)).await
    }

    fn poll_connect(addr: SocketAddr, cx: &mut Context<'_>) -> Poll<io::Result<TcpStream>> {
        CONNECT_TABLE.with(|state| {
            println!("doing poll_connect on addr: {:?}", addr);
            let mut state = state.lock().unwrap();
            println!("removing at addr: {:?}", addr);
            // NOTE: This might not be used if sender is none, but we need to keep the mutable sender reference.
            // NOTE: Since we need an immutable reference to state to get peer, make the peer first
            let peer = TcpStream::new_socket_addr(|s| state.contains_key(s), addr.ip());
            let sender = state.get_mut(&addr);
            println!("removed sender: {:?}", sender);
            match sender {
                None => Poll::Pending,
                Some(sender) => {
                    let sender_ready = sender.poll_ready_unpin(cx);
                    println!("Sender ready: {:?}", sender_ready);
                    match sender_ready {
                        Poll::Pending => Poll::Pending,
                        Poll::Ready(_) => {
                            // NOTE: Data channels are mpsc, whereas the connection channels are oneshot
                            // TODO: the streams should actually get two channels (one sender/receiver each)
                            // TODO: I think these channels should have infinite capacity?
                            let (other_sender, my_receiver) = channel::<u8>(0);
                            let (my_sender, other_receiver) = channel::<u8>(0);
                            // TODO handle when this is an error
                            let _res = sender.start_send((TcpStream::new(peer, addr, other_sender, other_receiver), peer));
                            println!("Send the connection stream");
                            Poll::Ready(Ok(TcpStream::new(addr, peer, my_sender, my_receiver)))
                        }
                    }
                }
            }
        })
    }

    fn new(addr: SocketAddr, peer: SocketAddr, sender: Sender<u8>, receiver: Receiver<u8>) -> TcpStream {
        let inner = Mutex::new(Inner { sender, receiver });
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
}

impl AsyncRead for TcpStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        println!("Poll read from {:?} with buffer {:?}", self, buf);
        let mut inner =  self.inner.lock().unwrap();
        let receiver = &mut inner.receiver;
        let mut written = false;
        loop {
            match receiver.poll_next_unpin(cx) {
                // TODO might want to assert that the buffer is not full on the first two branches
                Poll::Pending => 
                    if !written {
                        println!("Reading pending, and haven't written");
                        return Poll::Pending;
                    } else {
                        println!("Reading pending, but have written");
                        return Poll::Ready(Ok(()));
                    },
                Poll::Ready(Some(x)) => {
                    println!("Reading Some({:?})", x);
                    written = true;
                    buf.put_slice(&[x; 1]);
                }
                Poll::Ready(None) => {
                    println!("Reading None");
                    return Poll::Ready(Ok(()));
                },
            }
        }
    }
}

impl AsyncWrite for TcpStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        println!("Poll write from {:?} with buffer {:?}", self, buf);
        let mut inner = self.inner.lock().unwrap();
        let sender = &mut inner.sender;
        let mut written = 0usize;
        for i in buf {
            let res = sender.poll_ready_unpin(cx);
            match res {
                Poll::Pending =>
                    if written == 0 {
                        return Poll::Pending;
                    } else {
                        return Poll::Ready(Ok(written));
                    },
                Poll::Ready(Ok(_)) => {
                    written = written + 1;
                    let res = sender.start_send(*i);
                    if res.is_err() {
                        return Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, "SendError during start_send")));
                    }
                },
                Poll::Ready(Err(_e)) =>
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::Other,
                        "Sending over the data channel failed",
                    ))),
            }
        }
        Poll::Ready(Ok(buf.len()))
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
