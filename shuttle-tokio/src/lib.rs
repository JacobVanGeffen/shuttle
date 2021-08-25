//! Shuttle's implementation of [`tokio`].

mod util;
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

// TODO move this to own file
/// TODO
pub mod signal {
    /// TODO
    pub mod unix {
        use futures::future::Future;
        use std::pin::Pin;
        use std::task::{Poll, Context};

        /// TODO
        pub struct SignalKind {}
        impl SignalKind {
            pub fn interrupt() {}
        }

        /// TODO
        pub fn signal(_: ()) -> Option<SignalFuture> {
            Some(SignalFuture {})
        }

        /// TODO
        pub struct SignalFuture {}

        impl SignalFuture {
            pub fn recv(self) -> Self {
                self
            }
        }

        impl Future for SignalFuture {
            type Output = ();

            fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
                Poll::Pending
            }
        }
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
// TODO
pub use tokio::join;