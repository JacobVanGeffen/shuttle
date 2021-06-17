use shuttle::{asynch, check_random, tokio_utils};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use test_env_log::test;
use tokio::io::{copy, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

// Number of TCP clients that are going to send messages to the server
const NUM_MESSAGES: u64 = 3;
// How many iterations to run each test to try to see all executions
const ITERATIONS: usize = 500;

#[test]
fn james_demo() {
    let outcomes_orig = Arc::new(Mutex::new(HashMap::new()));
    let outcomes = outcomes_orig.clone();
    check_random(
        move || {
            let outcomes = outcomes.clone();
            let rt = Arc::new(
                tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .build()
                    .unwrap(),
            );
            // We need to `enter` the Tokio runtime here to set up the reactor for all the uses of
            // the Tokio library below. `enter` is a thread-local thing, but because we are running
            // under Shuttle, we have only one thread.
            let rt_clone = rt.clone();
            let _enter = rt_clone.enter();

            // Bind synchronously, and let the OS choose a port to listen on.
            let listener = shuttle::asynch::block_on(TcpListener::bind("127.0.0.1:0")).unwrap();
            let addr = listener.local_addr().unwrap();

            // The server will wait to receive `NUM_MESSAGES` messages, and then record them in
            // `outcomes`.
            let server = shuttle::asynch::spawn(async move {
                let mut result = vec![];
                for _ in 0..NUM_MESSAGES {
                    let (mut socket, _) = listener.accept().await.unwrap();
                    let msg = socket.read_u64().await.unwrap();
                    result.push(msg);
                }
                *outcomes.lock().unwrap().entry(result).or_insert(0) += 1;
            });

            // Each client will connect to the server and send its single message.
            let _clients = (0..NUM_MESSAGES)
                .map(|i| {
                    shuttle::asynch::spawn(async move {
                        let mut stream = TcpStream::connect(addr).await.unwrap();
                        stream.write_u64(i).await.unwrap();
                    })
                })
                .collect::<Vec<_>>();

            let _res = tokio_utils::run_tokio_server_with_runtime(rt.clone(), server);
        },
        ITERATIONS,
    );
    assert_eq!(outcomes_orig.lock().unwrap().len(), 6);
}

#[test]
fn echo_server() {
    check_random(
        // Task 0
        move || {
            let r = Arc::new(AtomicBool::new(false));
            let r1 = r.clone();
            let t = Arc::new(AtomicBool::new(false));
            let t1 = t.clone();
            let rt = Arc::new(
                tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .build()
                    .unwrap(),
            );
            let _enter = rt.clone().enter();

            // Bind the server's socket.
            // Task 1
            let listener_0 = shuttle::asynch::block_on(TcpListener::bind("127.0.0.1:0")).unwrap();
            let addr_0 = listener_0.local_addr().unwrap();
            let listener_12345 = shuttle::asynch::block_on(TcpListener::bind("127.0.0.1:12345")).unwrap();
            let addr_12345 = listener_12345.local_addr().unwrap();

            // Pull out a stream of sockets for incoming connections
            let server = asynch::spawn_named(
                async move {
                    let (mut socket_reader, _) = listener_0.accept().await.unwrap();
                    let (mut reader, _) = socket_reader.split();
                    let mut writer = TcpStream::connect(addr_12345).await.unwrap();
                    let result = copy(&mut reader, &mut writer).await;
                    println!("Result: {:?}", result);
                    assert!(result.is_ok() || r1.load(Ordering::SeqCst));
                    t1.store(true, Ordering::SeqCst);
                },
                Some("Server".to_string()),
            );

            // Connect to client to send some data
            // Task 3
            asynch::spawn_named(
                async move {
                    let mut stream = TcpStream::connect(addr_0).await.unwrap();
                    let res = stream.write_u64(12345678).await;
                    println!("Write res: {:?}", res);
                    r.store(true, Ordering::SeqCst);
                },
                Some("Write client".to_string()),
            );

            // Read from the port
            let read_client = asynch::spawn_named(
                async move {
                    match listener_12345.accept().await {
                        Err(_) => assert!(t.load(Ordering::SeqCst)),
                        Ok((mut socket, _)) => match socket.read_u64().await {
                            Err(_) => assert!(t.load(Ordering::SeqCst)),
                            Ok(msg) => {
                                assert_eq!(msg, 12345678);
                                println!("Read client: {:?}", msg);
                            }
                        },
                    }
                },
                Some("Read client".to_string()),
            );

            // Start the Tokio runtime
            // Task 4
            let _res = tokio_utils::run_tokio_server_with_runtime(rt, server);
            let _block = asynch::block_on(read_client);
        },
        ITERATIONS,
    );
}

#[test]
fn tokio_files() {
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;
    check_random(
        move || {
            let rt = Arc::new(
                tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .build()
                    .unwrap(),
            );
            let _enter = rt.clone().enter();
            let file_io = asynch::spawn_named(
                async move {
                    println!("Starting in the server");
                    let mut file = tokio_utils::nonblocking(File::create("foo.txt")).await.unwrap();
                    let _res = file.write_all(b"hello, world!").await;
                    let _res = tokio_utils::nonblocking(async move { file.set_len(10).await });
                },
                Some("Server".to_string()),
            );
            let _res = asynch::block_on(file_io);
            let _res = std::fs::remove_file("foo.txt");
        },
        ITERATIONS,
    );
}
