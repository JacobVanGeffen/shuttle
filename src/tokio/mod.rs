//! Shuttle's implementation of [`tokio`].

pub mod fs;
pub mod io;
pub mod net;
pub mod runtime;
pub mod sync;
pub mod task;
pub mod time;

mod pin;
mod join;

/// TODO
pub mod macros {
    /// TODO
    pub mod support {
        pub use std::pin::Pin as Pin;
        pub use std::task::Poll as Poll;
    }
}


// NOTE: Need this for tokio tests to also compile
pub use tokio::test;

// NOTE/TODO: This will not actually be able to let shuttle schedule interleavings
pub use tokio::select;

pub use crate::asynch::spawn;
pub use crate::task_local;
pub use crate::pin;
pub use crate::try_join;