//! Tokio utils
// TODO
//use crate::runtime::execution::ExecutionState;
use crate::asynch;
use futures::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::runtime::Runtime;

// A wrapper for a future that wakes itself up whenever polled.
// This means that it won't block when yielding
struct NonBlockingWrapper<F> {
    future: Pin<Box<F>>,
}

impl<T, F> NonBlockingWrapper<F>
where
    F: Future<Output = T> + Send + 'static,
{
    fn new(future: F) -> Self {
        Self {
            future: Box::pin(future),
        }
    }
}

impl<T, F> Future for NonBlockingWrapper<F>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.future.as_mut().poll(cx) {
            Poll::Ready(result) => Poll::Ready(result),
            Poll::Pending => {
                // This is the non-blocking part
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }
}

/// open tokio file
pub async fn nonblocking<T, F>(future: F) -> T
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    NonBlockingWrapper::new(future).await
}

/// Run a server on a given tokio runtime
// TODO if the server is not meant to stop running,
// TODO we don't want to panic when we're out of our 1000 polls
pub fn run_tokio_server_with_runtime<T, F>(rt: Arc<Runtime>, server: F) -> T
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let poller_stop = Arc::new(AtomicBool::new(false));
    let _poller = {
        let poller_stop = poller_stop.clone();

        super::thread::spawn_named(
            move || {
                for _ in 0..1000 {
                    if poller_stop.load(Ordering::SeqCst) {
                        return;
                    }
                    // std::thread::sleep(std::time::Duration::from_secs(1));
                    rt.block_on(tokio::task::yield_now());
                    super::thread::yield_now();
                }

                // Note that PCT scheduling won't work here -- we're relying on the random
                // scheduler being unlikely to choose the poller thread often.
                panic!("poller exhausted; this schedule must be very unfair!");
            },
            Some("Tokio Runtime loop".to_string()),
            None,
        )
    };

    let ret = asynch::block_on(server);
    poller_stop.store(true, Ordering::SeqCst);
    ret
}

/// Run a server on a new tokio runtime
pub fn run_tokio_server<T, F>(server: F) -> T
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let rt = Arc::new(
        tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap(),
    );
    let _enter = rt.enter();
    run_tokio_server_with_runtime(rt, server)
}
