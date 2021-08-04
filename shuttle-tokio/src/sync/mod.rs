//! Shuttle's implementation of [`tokio::sync`].

// pub use tokio::sync::*;

use shuttle::sync::MutexGuard;
use std::sync::LockResult;

/// A mutex, the same as [`std::sync::Mutex`].
#[derive(Debug)]
pub struct Mutex<T> {
    inner: shuttle::sync::Mutex<T>,
    //_p: PhantomData<T>,
}

// TODO tests (see tokio's tests)
impl<T> Mutex<T> {
    /// Creates a new mutex in an unlocked state ready for use.
    pub fn new(value: T) -> Self {
        Self {
            inner: shuttle::sync::Mutex::new(value),
        }
    }

    /// Acquires a mutex, blocking the current thread until it is able to do so.
    pub async fn lock(&self) -> MutexGuard<'_, T> {
        match self.inner.lock() {
            Ok(ret) => ret,
            Err(_e) => panic!("Lock future failed"),
        }
    }

    /// Calls the shuttle::sync::Mutex::lock function
    pub fn try_lock(&self) -> LockResult<MutexGuard<'_, T>> {
        self.inner.lock()
    }
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T: Default> Default for Mutex<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

// Temporarily use tokio channels, TODO remove later
// TODO/NEXT: just try using the futures crate channels, then test for concurrency (see Rajeev tests, think about edge cases)
// pub use tokio::sync::{mpsc, oneshot};
pub use futures::channel::oneshot;

/// TODO
pub mod mpsc {
    pub use futures::channel::mpsc::*;

    use futures::future::poll_fn;
    use futures::StreamExt;

    /// Mimic recv function from tokio
    pub async fn recv<T>(r: &mut Receiver<T>) -> Option<T> {
        poll_fn(|cx| r.poll_next_unpin(cx)).await
    }
}

// NOTE: Probably want to implement tokio Mutex's over shuttle's, but not include FIFO behavior
// TODO
// MPSC mock implementation
/*
pub mod mpsc {
    pub use crate::sync::mpsc::*;
    pub use crate::sync::mpsc::SyncSender as Sender;
    pub use crate::sync::mpsc::channel as unbounded;
    pub use crate::sync::mpsc::sync_channel as channel;
}
*/
