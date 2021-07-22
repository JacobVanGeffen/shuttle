//! Shuttle's implementation of [`tokio`].

pub mod fs;
pub mod io;
pub mod net;
pub mod task;
pub mod time;
pub mod sync;
pub mod runtime;

// TODO probably don't want this
pub use tokio::try_join;
pub use tokio::select;
pub use tokio::test;
pub use tokio::macros;
pub use tokio::pin;

pub use crate::asynch::spawn as spawn;