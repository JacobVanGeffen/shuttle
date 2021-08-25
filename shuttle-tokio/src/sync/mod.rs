//! Shuttle's implementation of [`tokio::sync`].

// pub use tokio::sync::*;
// TODO
pub use tokio::sync::Notify;

mod batch_semaphore;
mod semaphore;
mod mutex;
 
mod rwlock;
pub use rwlock::RwLock;
pub use rwlock::owned_read_guard::OwnedRwLockReadGuard;
pub use rwlock::owned_write_guard::OwnedRwLockWriteGuard;
pub use rwlock::owned_write_guard_mapped::OwnedRwLockMappedWriteGuard;
pub use rwlock::read_guard::RwLockReadGuard;
pub use rwlock::write_guard::RwLockWriteGuard;
pub use rwlock::write_guard_mapped::RwLockMappedWriteGuard;
   
pub use batch_semaphore::{AcquireError, TryAcquireError};
pub use semaphore::*;
pub use mutex::*;

// TODO
pub mod watch {
    use std::marker::PhantomData;

    pub struct Receiver<T> {
        _p: PhantomData<T>,
    }

    impl<T> Receiver<T> {
        pub fn new() -> Receiver<T> {
            Receiver { _p: PhantomData }
        }
    }
}

// Temporarily use tokio channels, TODO remove later
// TODO/NEXT: just try using the futures crate channels, then test for concurrency (see Rajeev tests, think about edge cases)
// pub use tokio::sync::{mpsc, oneshot};
pub use futures::channel::oneshot;

/// TODO
pub mod mpsc {
    pub use futures::channel::mpsc::*;
    pub use futures::channel::mpsc::unbounded as unbounded_channel;

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
