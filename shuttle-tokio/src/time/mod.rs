//! Shuttle's implementation of [`tokio::time`].

use rand::Rng;
use shuttle::rand::thread_rng;
use shuttle::thread;
use futures::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

// NOTE: There doesn't seem to be any async in here, so this is fine
pub use tokio::time::{Duration, Instant};

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
            thread::request_yield();
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
    counter: usize,
}

impl<T> Future for Timeout<T>
where
    T: Future,
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

        // Now check the countdown timer, with shuttle randomness
        // TODO what should the probability be? Maybe it could increase every time poll is called or smth. For now, just make it low.
        // TODO another solution: When Timeout is created, rand gen a number of polls that are allowed to get called
        // TODO
        if self.counter == 0 {
            Poll::Ready(Err(()))
        } else {
            (*self).counter = self.counter - 1;
            Poll::Pending
        }
    }
}

/// Mock of tokio's timeout, which randomly returns Pending or Ready(Err) each time it is polled and the inner future has not completed
pub fn timeout_at<T>(_: Instant, future: T) -> Timeout<T>
where
    T: Future,
{
    timeout(Duration::new(0, 0), future)
}

/// Mock of tokio's timeout, which randomly returns Pending or Ready(Err) each time it is polled and the inner future has not completed
pub fn timeout<T>(_: Duration, future: T) -> Timeout<T>
where
    T: Future,
{
    Timeout {
        value: Box::pin(future),
        // Randomly define the number of ticks the timeout should take
        // TODO what should the high be?
        counter: thread_rng().gen_range(0, 10000),
    }
}
