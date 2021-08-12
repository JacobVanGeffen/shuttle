use futures::FutureExt;
use futures::future::Future;
use shuttle_tokio::io::{AsyncReadExt, AsyncWriteExt, Interest};
use shuttle_tokio::net::{TcpListener, TcpStream};
use shuttle_tokio::try_join;
use shuttle_tokio as tokio;
use shuttle::{asynch, check_dfs, thread};

use std::io;
use std::pin::Pin;
use std::task::{Poll, Waker};
use std::time::Duration;

use futures::future::poll_fn;

/*
fn check<F, T>(f: F)
where
    F: Future<Output = T> + Send + Sync + 'static,
    T: Send + 'static,
{
    check_dfs(move || { asynch::block_on(f); }, None);
}
*/

macro_rules! check {
    ($x:expr) => {{
        check_dfs(move || { asynch::block_on($x); }, None);
    }}
}

/*
fn poll_future<F, T>(f: Pin<&mut F>) -> Poll<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    asynch::block_on(async move { poll_fn(|cx| { Poll::Ready(f.poll(cx)) }}))
}
*/

macro_rules! assert_ok {
    ($x:expr) => {{
        use std::result::Result::*;
        match $x {
            Ok(v) => v,
            Err(e) => panic!("assertion failed: Err({:?})", e),
        }
    }};
}

macro_rules! assert_pending {
    ($x:expr) => {{ assert!($x.is_pending()) }};
}

macro_rules! assert_ready_ok {
    ($x:expr) => {{
        match $x {
            Poll::Pending => panic!("assertion failed: Poll::Pending"),
            Poll::Ready(v) => assert_ok!(v),
        }
    }};
}

// TODO TEST: Spawn a task that never completes (always return pending), then drop it. See what happens (I think it will "deadlock")
// TODO TEST: Determinism of implemented libraries: Make a test that fails, then replay that schedule N times to ensure that it fails the same way every time
// TODO TEST: Figure out some tests that purposfully fail in BA (see timeout stuff)
// TODO: See if BA uses RNG anywhere else, this would be a problem

// NOTE: These tests are from tokio net tcp tests
#[test]
fn test_set_linger() {
    check!(set_linger());
}

async fn set_linger() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

    let stream = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();

    assert_ok!(stream.set_linger(Some(Duration::from_secs(1))));
    // assert_eq!(stream.linger().unwrap().unwrap().as_secs(), 1);

    assert_ok!(stream.set_linger(None));
    // assert!(stream.linger().unwrap().is_none());
}

#[test]
fn test_try_read_write() {
    check!(try_read_write());
}

async fn try_read_write() {
    const DATA: &[u8] = b"this is some data to write to the socket";

    // Create listener
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

    // Create socket pair
    let mut client = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    let (mut server, _) = listener.accept().await.unwrap();
    let mut written = DATA.to_vec();

    // Track the server receiving data
    let readable = server.readable();
    // assert_pending!(readable.poll_unpin(x).);

    // Write data.
    client.writable().await.unwrap();
    assert_eq!(DATA.len(), client.write(DATA).await.unwrap());

    readable.await;
    /*
    // The task should be notified
    while !readable.is_woken() {
        asynch::yield_now().await;
    }
    */

    // Fill the write buffer using non-vectored I/O
    loop {
        // Still ready
        let writable = client.writable();
        writable.await;
        //assert_ready_ok!(writable.poll_unpin());

        match client.write(DATA).await {
            Ok(n) => written.extend(&DATA[..n]),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                break;
            }
            Err(e) => panic!("error = {:?}", e),
        }
    }

    {
        // Write buffer full
        let writable = client.writable();
        //assert_pending!(writable.poll_unpin());

        // Drain the socket from the server end using non-vectored I/O
        let mut read = vec![0; written.len()];
        let mut i = 0;

        while i < read.len() {
            server.readable().await.unwrap();

            match server.read(&mut read[i..]).await {
                Ok(n) => i += n,
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => panic!("error = {:?}", e),
            }
        }

        assert_eq!(read, written);
    }

    written.clear();
    client.writable().await.unwrap();

    // Fill the write buffer using vectored I/O
    let data_bufs: Vec<_> = DATA.chunks(10).map(io::IoSlice::new).collect();
    loop {
        // Still ready
        let writable = client.writable();
        writable.await.unwrap();
        //assert_ready_ok!(writable.poll_unpin());

        match client.write_vectored(&data_bufs).await {
            Ok(n) => written.extend(&DATA[..n]),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                break;
            }
            Err(e) => panic!("error = {:?}", e),
        }
    }

    {
        // Write buffer full
        let writable = client.writable();
        //assert_pending!(writable.poll_unpin());

        // Drain the socket from the server end using vectored I/O
        let mut read = vec![0; written.len()];
        let mut i = 0;

        while i < read.len() {
            server.readable().await.unwrap();

            let mut bufs: Vec<_> = read[i..]
                .chunks_mut(0x10000)
                .map(io::IoSliceMut::new)
                .collect();
            match server.read_vectored(&mut bufs).await {
                Ok(n) => i += n,
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => panic!("error = {:?}", e),
            }
        }

        assert_eq!(read, written);
    }

    // Now, we listen for shutdown

    loop {
        let ready = server.ready(Interest::READABLE).await;

        if ready.is_ok() {
            return;
        } else {
            tokio::task::yield_now().await;
        }
    }
}

/*
macro_rules! assert_readable_by_polling {
    ($stream:expr) => {
        assert_ok!(poll_fn(|cx| $stream.poll_read_ready(cx)).await);
    };
}

macro_rules! assert_not_readable_by_polling {
    ($stream:expr) => {
        poll_fn(|cx| {
            assert_pending!($stream.poll_read_ready(cx));
            Poll::Ready(())
        })
        .await;
    };
}

macro_rules! assert_writable_by_polling {
    ($stream:expr) => {
        assert_ok!(poll_fn(|cx| $stream.poll_write_ready(cx)).await);
    };
}

macro_rules! assert_not_writable_by_polling {
    ($stream:expr) => {
        poll_fn(|cx| {
            assert_pending!($stream.poll_write_ready(cx));
            Poll::Ready(())
        })
        .await;
    };
}

#[tokio::test]
async fn poll_read_ready() {
    let (mut client, mut server) = create_pair().await;

    // Initial state - not readable.
    assert_not_readable_by_polling!(server);

    // There is data in the buffer - readable.
    assert_ok!(client.write_all(b"ping").await);
    assert_readable_by_polling!(server);

    // Readable until calls to `poll_read` return `Poll::Pending`.
    let mut buf = [0u8; 4];
    assert_ok!(server.read_exact(&mut buf).await);
    assert_readable_by_polling!(server);
    read_until_pending(&mut server);
    assert_not_readable_by_polling!(server);

    // Detect the client disconnect.
    drop(client);
    assert_readable_by_polling!(server);
}

#[tokio::test]
async fn poll_write_ready() {
    let (mut client, server) = create_pair().await;

    // Initial state - writable.
    assert_writable_by_polling!(client);

    // No space to write - not writable.
    write_until_pending(&mut client);
    assert_not_writable_by_polling!(client);

    // Detect the server disconnect.
    drop(server);
    assert_writable_by_polling!(client);
}

async fn create_pair() -> (TcpStream, TcpStream) {
    let listener = assert_ok!(TcpListener::bind("127.0.0.1:0").await);
    let addr = assert_ok!(listener.local_addr());
    let (client, (server, _)) = assert_ok!(try_join!(TcpStream::connect(&addr), listener.accept()));
    (client, server)
}

async fn read_until_pending(stream: &mut TcpStream) {
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        match stream.read(&mut buf).await {
            Ok(_) => (),
            Err(err) => {
                assert_eq!(err.kind(), io::ErrorKind::WouldBlock);
                break;
            }
        }
    }
}

async fn write_until_pending(stream: &mut TcpStream) {
    let buf = vec![0u8; 1024 * 1024];
    loop {
        match stream.write(&buf).await {
            Ok(_) => (),
            Err(err) => {
                assert_eq!(err.kind(), io::ErrorKind::WouldBlock);
                break;
            }
        }
    }
}

#[tokio::test]
async fn try_read_buf() {
    const DATA: &[u8] = b"this is some data to write to the socket";

    // Create listener
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

    // Create socket pair
    let client = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    let (server, _) = listener.accept().await.unwrap();
    let mut written = DATA.to_vec();

    // Track the server receiving data
    let mut readable = server.readable();
    //assert_pending!(readable.poll_unpin());

    // Write data.
    client.writable().await.unwrap();
    assert_eq!(DATA.len(), client.try_write(DATA).unwrap());

    // The task should be notified
    readable.await;

    // Fill the write buffer
    loop {
        // Still ready
        let writable = client.writable();
        writable.await.unwrap();
        //assert_ready_ok!(writable.poll_unpin());

        match client.try_write(DATA) {
            Ok(n) => written.extend(&DATA[..n]),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                break;
            }
            Err(e) => panic!("error = {:?}", e),
        }
    }

    {
        // Write buffer full
        let mut writable = asynch::spawn(client.writable());
        //assert_pending!(writable.poll_unpin());

        // Drain the socket from the server end
        let mut read = Vec::with_capacity(written.len());
        let mut i = 0;

        while i < read.capacity() {
            server.readable().await.unwrap();

            match server.try_read_buf(&mut read) {
                Ok(n) => i += n,
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => panic!("error = {:?}", e),
            }
        }

        assert_eq!(read, written);
    }

    // Now, we listen for shutdown
    drop(client);

    loop {
        let ready = server.ready(Interest::READABLE).await.unwrap();

        if ready.is_read_closed() {
            return;
        } else {
            tokio::task::yield_now().await;
        }
    }
}
*/