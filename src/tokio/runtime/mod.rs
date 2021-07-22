//! Shuttle's implementation of [`tokio::runtime`].

// pub use tokio::runtime::*;

use std::io;
use futures::future::Future;

/// Mock implementation of tokio's Builder that does nothing
#[derive(Debug)]
pub struct Builder {}

impl Builder {
    /// Mock implementation of tokio's Builder functions that does nothing
    pub fn new_current_thread() -> Builder {
        Builder {}
    }
    /// Mock implementation of tokio's Builder functions that does nothing
    pub fn new_multi_thread() -> Builder {
        Builder {}
    }

    /// Mock implementation of tokio's Builder functions that does nothing
    pub fn worker_threads(&mut self, _val: usize) -> &mut Self {
        self
    }

    /// Mock implementation of tokio's Builder functions that does nothing
    pub fn enable_all(&mut self) -> &mut Self {
        self
    }

    /// Mock implementation of tokio's build function that returns an empty Runtime
    pub fn build(&mut self) -> io::Result<Runtime> {
        Ok(Runtime {})
    }
}

/// Mock implementation of tokio's Runtime that spawns shuttle tasks
#[derive(Debug)]
pub struct Runtime {}

impl Runtime {
    /// Block on a future
    pub fn block_on<T, F>(&self, f: F) -> T
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        crate::asynch::block_on(async move { f.await })
    }

    /// Returns self
    pub fn handle(&self) -> &Self {
        self
    }
}