//! Shuttle's implementation of [`tokio::task`].

pub use tokio::task::*;
pub use crate::asynch::JoinHandle;
pub use crate::asynch::JoinError;