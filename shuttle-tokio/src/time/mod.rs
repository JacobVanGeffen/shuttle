//! Shuttle's implementation of [`tokio::time`].

use rand::Rng;
use shuttle::rand::thread_rng;
use futures::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

// NOTE: There doesn't seem to be any async in here, so this is fine
pub use tokio::time::{Duration, Instant};

/// Mock of tokio's Timeout, which randomly returns Pending or Ready(Err) each time it is polled and the inner future has not completed
#[derive(Debug)]
pub struct Timeout<T> {
    value: Pin<Box<T>>,
    counter: usize,
}

impl<T> Timeout<T> {
    /// Reset the timeout of the Sleep, which really means yield again
    pub fn reset(mut self: Pin<&mut Self>, _deadline: Instant) {
        self.counter = 10;
    }
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
            println!("timeout finished");
            return Poll::Ready(Ok(v));
        }

        // Now check the countdown timer, with shuttle randomness
        if self.counter == 0 {
            println!("timeout errored");
            Poll::Ready(Err(()))
        } else {
            (*self).counter = self.counter - 1;
            Poll::Pending
        }
    }
}

// TODO NEXT: We actually want to implement sleep also using Timeout, because it could be polled multiple times

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
    println!("timeout called");
    Timeout {
        value: Box::pin(future),
        // Randomly define the number of ticks the timeout should take
        // TODO what should the high be?
        counter: thread_rng().gen_range(0, 10),
    }
}

/// Mock of tokio's Sleep, implemented as a one-time yield
#[derive(Debug)]
pub struct Sleep {
    timeout: Pin<Box<Timeout<AlwaysPending>>>,
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.timeout.as_mut().poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(_) => Poll::Ready(()),
        }
    }
}

impl Sleep {
    /// Reset the timeout of the Sleep, which really means yield again
    pub fn reset(mut self: Pin<&mut Self>, deadline: Instant) {
        self.timeout.as_mut().reset(deadline);
    }
}

/// Mock of tokio's sleep, implemented as a one-time yield
pub fn sleep(dur: Duration) -> Sleep {
    Sleep { timeout: Box::pin(timeout(dur, AlwaysPending {})) }
}

#[derive(Debug)]
struct AlwaysPending {}

impl Future for AlwaysPending {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}