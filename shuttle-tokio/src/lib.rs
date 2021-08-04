//! Shuttle's implementation of [`tokio`].

pub mod fs;
pub mod io;
pub mod net;
pub mod runtime;
pub mod sync;
pub mod task;
pub mod time;

mod join;
mod pin;

/// TODO
pub mod macros {
    /// TODO
    pub mod support {
        pub use std::pin::Pin;
        pub use std::task::Poll;
    }
}

// NOTE: Need this for tokio tests to also compile
pub use tokio::test;

// NOTE/TODO: This will not actually be able to let shuttle schedule interleavings
pub use tokio::select;

pub use shuttle::asynch::spawn;
pub use shuttle::task_local;
pub use crate::my_pin as pin;
pub use crate::my_try_join as try_join;
