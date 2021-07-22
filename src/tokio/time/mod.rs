//! Shuttle's implementation of [`tokio::time`].

use crate::runtime::execution::ExecutionState;
use futures::future::Future;
use rand::Rng;
use std::task::{Poll, Context};
use std::pin::Pin;

// NOTE: There doesn't seem to be any async in here, so this is fine
pub use tokio::time::{Instant, Duration};

/// Mock of tokio's Sleep, implemented as a one-time yield
#[derive(Debug)]
pub struct Sleep {
    has_yielded: bool,
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.has_yielded {
            Poll::Ready(())
        } else {
            cx.waker().wake_by_ref();
            ExecutionState::request_yield();
            self.has_yielded = true;
            Poll::Pending
        }
    }
}

impl Sleep {
    /// Reset the timeout of the Sleep, which really means yield again
    pub fn reset(mut self: Pin<&mut Self>, _deadline: Instant) {
        self.has_yielded = false;
    }
}

/// Mock of tokio's sleep, implemented as a one-time yield
pub fn sleep(_dur: Duration) -> Sleep {
    Sleep { has_yielded: false }
}

/// Mock of tokio's Timeout, which randomly returns Pending or Ready(Err) each time it is polled and the inner future has not completed
#[derive(Debug)]
pub struct Timeout<T> {
    value: Pin<Box<T>>,
}

impl<T> Future for Timeout<T>
where
    T: Future
{
    type Output = Result<T::Output, ()>;

    // TODO what if the timeout is 0 (or instant was in the past)?
    //      for now, maybe just panic
    // NOTE: The implementation is structured like this, so "value" will get polled at least once
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // First, try polling the future
        if let Poll::Ready(v) = self.value.as_mut().poll(cx) {
            return Poll::Ready(Ok(v));
        }

        // Now check the timer, with shuttle randomness
        // TODO what should the probability be? Maybe it could increase every time poll is called or smth. For now, just make it low.
        // TODO another solution: When Timeout is created, rand gen a number of polls that are allowed to get called
        if crate::rand::thread_rng().gen_bool(0.1) {
            Poll::Ready(Err(()))
        } else {
            Poll::Pending
        }
    }
}

/// Mock of tokio's timeout, which randomly returns Pending or Ready(Err) each time it is polled and the inner future has not completed
pub fn timeout_at<T>(_: Instant, future: T) -> Timeout<T>
where
    T: Future
{
    timeout(Duration::new(0, 0), future)
}

/// Mock of tokio's timeout, which randomly returns Pending or Ready(Err) each time it is polled and the inner future has not completed
pub fn timeout<T>(_: Duration, future: T) -> Timeout<T>
where
    T: Future
{
    Timeout { value: Box::pin(future) }
}