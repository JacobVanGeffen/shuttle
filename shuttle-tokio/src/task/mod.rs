//! Shuttle's implementation of [`tokio::task`].

use futures::future::Future;

pub use shuttle::asynch::JoinError;
pub use shuttle::asynch::JoinHandle;
pub use shuttle::asynch::spawn;
pub use shuttle::asynch::yield_now;

pub fn spawn_blocking<T, F>(fut: F) -> JoinHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    spawn(fut)
}

//pub use tokio::task::*;
