use super::tokio_utils;
use shuttle::replay;
#[allow(unused)]
use shuttle::{asynch, check, check_random};
use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use test_env_log::test;
// TODO don't use copy
use shuttle::tokio::net::tcp::{TcpListener, TcpStream};
use tokio::io::{copy, AsyncReadExt, AsyncWriteExt};

// Number of TCP clients that are going to send messages to the server
const NUM_MESSAGES: u64 = 3;
// How many iterations to run each test to try to see all executions
const ITERATIONS: usize = 500;

//#[test]
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

            println!("Got the listeners");

            // The server will wait to receive `NUM_MESSAGES` messages, and then record them in
            // `outcomes`.
            let server = shuttle::asynch::spawn_named(
                async move {
                    let mut result = vec![];
                    for _ in 0..NUM_MESSAGES {
                        println!("hit server");
                        let (mut socket, _) = listener.accept().await.unwrap();
                        println!("server got listener");
                        let msg = socket.read_u64().await.unwrap();
                        println!("server got message");
                        result.push(msg);
                    }
                    *outcomes.lock().unwrap().entry(result).or_insert(0) += 1;
                },
                Some("Server".to_string()),
            );

            // Each client will connect to the server and send its single message.
            let _clients = (0..NUM_MESSAGES)
                .map(|i| {
                    let addr = addr.clone();
                    shuttle::asynch::spawn_named(
                        async move {
                            println!("hit client");
                            let mut stream = TcpStream::connect(addr).await.unwrap();
                            println!("client got stream");
                            stream.write_u64(i).await.unwrap();
                            println!("client wrote message");
                        },
                        Some(format!("client {:?}", i)),
                    )
                })
                .collect::<Vec<_>>();

            let _res = tokio_utils::run_tokio_server_with_runtime(rt.clone(), server);
            // asynch::block_on(server); //
        },
        1, // ITERATIONS,
    );
    // assert_eq!(outcomes_orig.lock().unwrap().len(), 6);
}

//#[test]
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

//#[test]
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
            let _file_io = asynch::spawn_named(
                async move {
                    println!("Starting in the server");
                    let mut file = File::create("foo.txt").await.unwrap(); // tokio_utils::nonblocking(File::create("foo.txt")).await.unwrap(); //
                    println!("Got file {:?}", file);
                    let _res = file.write_all(b"hello, world!").await;
                    println!("Wrote file");
                    let _ = file.set_len(10).await; // tokio_utils::nonblocking(async move { let _ = file.set_len(10); }).await; //
                    println!("Trunc'd file");
                },
                Some("Server".to_string()),
            );
            // NOTE: Need to block here so that the tokio runtime stays active while file_io runs
            // let _res = asynch::block_on(file_io);
            // let _res = std::fs::remove_file("foo.txt");
        },
        1,
    );
}

//#[test]
fn normal_files() {
    use std::fs::File;
    check_random(
        move || {
            asynch::spawn(async move {
                asynch::yield_now().await;
                println!("hi")
            });
            asynch::spawn(async move {
                asynch::yield_now().await;
                let mut file = File::create("bar.txt").unwrap();
                let _ = file.write_all(b"hi, world!");
                println!("done");
            });
        },
        ITERATIONS,
    );
}

//#[test]
fn send_three_bytes() {
    check_random(
        move || {
            let rt = Arc::new(
                tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .build()
                    .unwrap(),
            );
            let rt_clone = rt.clone();
            let _enter = rt_clone.enter();

            // Bind synchronously, and let the OS choose a port to listen on.
            let listener = shuttle::asynch::block_on(TcpListener::bind("127.0.0.1:8080")).unwrap();
            let addr = listener.local_addr().unwrap();

            println!("Got the listeners");

            // The server will wait to receive `NUM_MESSAGES` messages, and then record them in
            // `outcomes`.
            let server = shuttle::asynch::spawn_named(
                async move {
                    let (mut socket, _) = listener.accept().await.unwrap();
                    let mut buf: [u8; 4] = [0; 4];
                    let msg = socket.read(&mut buf).await.unwrap();
                    println!("Read {:?} bytes", msg);
                },
                Some("Server".to_string()),
            );

            // Each client will connect to the server and send its single message.
            let _client = shuttle::asynch::spawn_named(
                async move {
                    let mut stream = TcpStream::connect(addr).await.unwrap();
                    let res = stream.write_u8(7).await;
                    println!("res {:?}", res);
                    let res = stream.write_u8(8).await;
                    println!("res {:?}", res);
                    let res = stream.write_u8(9).await;
                    println!("res {:?}", res);
                },
                Some("client".to_string()),
            );

            // let _res = tokio_utils::run_tokio_server_with_runtime(rt.clone(), server);
            let _ = asynch::block_on(server); //
        },
        // TODO just call a function that cleans up the global map
        1, // ITERATIONS,
    );
}

//#[test]
fn one_way_server() {
    check_random(
        move || {
            let rt = Arc::new(
                tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .build()
                    .unwrap(),
            );
            let rt_clone = rt.clone();
            let _enter = rt_clone.enter();

            // Bind synchronously, and let the OS choose a port to listen on.
            let listener = shuttle::asynch::block_on(TcpListener::bind("127.0.0.1:8080")).unwrap();
            let addr = listener.local_addr().unwrap();

            println!("Got the listeners");

            // The server will wait to receive `NUM_MESSAGES` messages, and then record them in
            // `outcomes`.
            let server = shuttle::asynch::spawn_named(
                async move {
                    println!("hit server");
                    let (mut socket, _) = listener.accept().await.unwrap();
                    println!("server got listener");
                    let msg = socket.read_u16().await.unwrap();
                    println!("server got message: {:?}", msg);
                    let msg = socket.read_u16().await.unwrap();
                    println!("server got message: {:?}", msg);
                    socket.write_u16(0xabcd).await.unwrap();
                    socket.write_u16(0xef90).await.unwrap();
                    println!("server wrote 2 messages");
                },
                Some("Server".to_string()),
            );

            // Each client will connect to the server and send its single message.
            let _client = shuttle::asynch::spawn_named(
                async move {
                    println!("hit client");
                    let mut stream = TcpStream::connect(addr).await.unwrap();
                    println!("client got stream");
                    stream.write_u32(0x12345678).await.unwrap();
                    println!("client wrote message");
                    let msg = stream.read_u32().await.unwrap();
                    println!("client got message: {:?}", msg);
                },
                Some("client".to_string()),
            );

            // let _res = tokio_utils::run_tokio_server_with_runtime(rt.clone(), server);
            let _ = asynch::block_on(server); //
        },
        1, // ITERATIONS,
    );
}

//#[test]
fn disconnect_writer() {
    check_random(
        move || {
            let rt = Arc::new(
                tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .build()
                    .unwrap(),
            );
            let rt_clone = rt.clone();
            let _enter = rt_clone.enter();

            // Bind synchronously, and let the OS choose a port to listen on.
            let listener = shuttle::asynch::block_on(TcpListener::bind("127.0.0.1:8080")).unwrap();
            let addr = listener.local_addr().unwrap();

            println!("Got the listeners");

            // The server will wait to receive `NUM_MESSAGES` messages, and then record them in
            // `outcomes`.
            let server = shuttle::asynch::spawn_named(
                async move {
                    let (mut stream, _) = listener.accept().await.unwrap();
                    let read_res = stream.read_u32().await;
                    println!("Result of read: {:?}", read_res);
                },
                Some("Server".to_string()),
            );

            // Each client will connect to the server and send its single message.
            let _client = shuttle::asynch::spawn_named(
                async move {
                    let _ = TcpStream::connect(addr).await.unwrap();
                    // Disconnect before performing a write
                },
                Some("client".to_string()),
            );

            // let _res = tokio_utils::run_tokio_server_with_runtime(rt.clone(), server);
            let _ = asynch::block_on(server); //
        },
        1, // ITERATIONS,
    );
}

//#[test]
fn disconnect_reader() {
    check_random(
        move || {
            let rt = Arc::new(
                tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .build()
                    .unwrap(),
            );
            let rt_clone = rt.clone();
            let _enter = rt_clone.enter();

            // Bind synchronously, and let the OS choose a port to listen on.
            let listener = shuttle::asynch::block_on(TcpListener::bind("127.0.0.1:8080")).unwrap();
            let addr = listener.local_addr().unwrap();

            println!("Got the listeners");

            // The server will wait to receive `NUM_MESSAGES` messages, and then record them in
            // `outcomes`.
            let server = shuttle::asynch::spawn_named(
                async move {
                    let _ = listener.accept().await.unwrap();
                    // Disconnect before receiving the write
                },
                Some("Server".to_string()),
            );

            // Each client will connect to the server and send its single message.
            let _client = shuttle::asynch::spawn_named(
                async move {
                    let mut stream = TcpStream::connect(addr).await.unwrap();
                    let write_res = stream.write_u32(0x12345678).await;
                    println!("Result of write: {:?}", write_res);
                },
                Some("client".to_string()),
            );

            // let _res = tokio_utils::run_tokio_server_with_runtime(rt.clone(), server);
            let _ = asynch::block_on(server); //
        },
        1, // ITERATIONS,
    );
}

fn triple_echo_closure() {
    let rt = Arc::new(
        tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap(),
    );
    let _enter = rt.clone().enter();

    let listener = Arc::new(asynch::block_on(TcpListener::bind("127.0.0.1:8080")).unwrap());
    let addr = listener.local_addr().unwrap();

    let mut listener2s = Vec::with_capacity(3);
    listener2s.push(asynch::block_on(TcpListener::bind("127.0.0.1:8181")).unwrap());
    listener2s.push(asynch::block_on(TcpListener::bind("127.0.0.1:8282")).unwrap());
    listener2s.push(asynch::block_on(TcpListener::bind("127.0.0.1:8383")).unwrap());
    let mut addr2s = Vec::with_capacity(3);
    addr2s.push(listener2s[0].local_addr().unwrap());
    addr2s.push(listener2s[1].local_addr().unwrap());
    addr2s.push(listener2s[2].local_addr().unwrap());

    let _clients = (0..3)
        .zip(listener2s)
        .map(|(i, l)| {
            let addr = addr.clone();
            asynch::spawn_named(
                async move {
                    println!("hit client {:?}", i);
                    // TODO but also has:
                    // (1) an Arc<Option<TaskId>> (also mutex?) for the task of the listener accepting (which is updated on l.accept(), used on TcpStream::connect)
                    // .      (should this be a vec instead of an option? in case somehow multiple tasks are trying to accept the listener)
                    // (2) an Arc<Vec<TaskId>> (also mutex?) for the tasks
                    let mut stream = TcpStream::connect(addr).await.unwrap();
                    println!("client {:?} got stream", i);
                    let res = stream.write_u8(i + 1).await;
                    println!("Write res for client {:?}: {:?}", i, res);
                    println!("client wrote: {:?}", i);
                    let mut buf: [u8; 1024] = [0; 1024];
                    // TODO remove
                    // let addr = l.local_addr().unwrap();
                    // TODO does this cause a problem? if so, why?
                    // let stream = asynch::block_on(async move { TcpStream::connect(addr).await.unwrap() });
                    // println!("Got stream: {:?}", stream);
                    let (mut socket, _) = l.accept().await.unwrap();
                    println!("Got socket for client {:?}", i);
                    #[allow(unused)]
                    match socket.read(&mut buf).await {
                        Ok(n) => println!("Got back: {:?}", &buf[0..n]), // panic!("expected at client {:?}", i), //
                        Err(e) => println!("failed to read from socket on client; err = {:?}", e), // panic!("expected at client {:?}", i), //
                    };
                },
                Some(format!("client {:?}", i)),
            )
        })
        .collect::<Vec<_>>();

    let mut servers = Vec::with_capacity(3);
    for i in 0..3 {
        let listener = listener.clone();
        let addr2s = addr2s.clone();
        let (mut socket, _) = asynch::block_on(async move { listener.accept().await }).unwrap();

        let server = asynch::spawn_named(
            async move {
                println!("before server {:?} receiving socket", i);
                // NOTE: CANNOT call listener.accept() multiple times before getting a single connection
                // .     This is because accept() is backed by mpsc, so only one listener will get woken up
                // let (mut socket, _) = listener.accept().await.unwrap();
                println!("past server {:?} receiving socket", i);
                let mut buf = [0; 1024];

                // In a loop, read data from the socket and write the data back.
                loop {
                    println!("About to read from the server {:?}", i);
                    let n = match socket.read(&mut buf).await {
                        // socket closed
                        Ok(n) if n == 0 => return,
                        Ok(n) => n,
                        Err(e) => {
                            eprintln!("failed to read from socket; err = {:?}", e);
                            return;
                        }
                    };
                    println!("Read {:?} from server {:?}", buf[0], i);

                    // Write the data back
                    println!("About to TcpStream::connect on the server {:?}", i);
                    let mut stream = TcpStream::connect(addr2s[(buf[0] - 1) as usize].clone()).await.unwrap();
                    println!("About to write from the server {:?}", i);
                    // TODO write back both buf AND the server number, then assert the set is the right size (outside of the check_random)
                    if let Err(e) = stream.write_all(&buf[0..n]).await {
                        eprintln!("failed to write to socket; err = {:?}", e);
                        return;
                    }
                    println!("Finished write from the server {:?}", i);
                }
            },
            Some(format!("server {:?}", i)),
        );
        servers.push(server);
    }

    let _server = asynch::spawn(async move {
        for i in 0..3 {
            let _ = servers.get_mut(i as usize).unwrap().await;
        }
    });

    println!("Blocking on server");
    // let _ = asynch::block_on(server);
    // let _ = tokio_utils::run_tokio_server_with_runtime(rt, server);
}

//#[test]
fn triple_echo() {
    check_random(triple_echo_closure, 1);
}

//#[test]
fn triple_echo_replay() {
    replay(
        triple_echo_closure,
        "91041ba7a3eea8c5c8b497c90100084000000600049ca2964a4721830c5a",
    );
}