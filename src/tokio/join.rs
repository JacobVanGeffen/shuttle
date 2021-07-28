/// Performs a try_join
// TODO allow shuttle to handle the polling of the futures
// Copied from Tokio
#[macro_export]
macro_rules! try_join {
    (@ {
        // One `_` for each branch in the `try_join!` macro. This is not used once
        // normalization is complete.
        ( $($count:tt)* )

        // Normalized try_join! branches
        $( ( $($skip:tt)* ) $e:expr, )*

    }) => {{
        use futures::future::{maybe_done, poll_fn, Future};
        use std::pin::Pin;
        use std::task::Poll::{Ready, Pending};

        // Safety: nothing must be moved out of `futures`. This is to satisfy
        // the requirement of `Pin::new_unchecked` called below.
        let mut futures = ( $( maybe_done($e), )* );

        poll_fn(move |cx| {
            let mut is_pending = false;

            $(
                // Extract the future for this branch from the tuple.
                let ( $($skip,)* fut, .. ) = &mut futures;

                // Safety: future is stored on the stack above
                // and never moved.
                let mut fut = unsafe { Pin::new_unchecked(fut) };

                // Try polling
                if fut.as_mut().poll(cx).is_pending() {
                    is_pending = true;
                } else if fut.as_mut().output_mut().expect("expected completed future").is_err() {
                    return Ready(Err(fut.take_output().expect("expected completed future").err().unwrap()))
                }
            )*

            if is_pending {
                Pending
            } else {
                Ready(Ok(($({
                    // Extract the future for this branch from the tuple.
                    let ( $($skip,)* fut, .. ) = &mut futures;

                    // Safety: future is stored on the stack above
                    // and never moved.
                    let mut fut = unsafe { Pin::new_unchecked(fut) };

                    fut
                        .take_output()
                        .expect("expected completed future")
                        .ok()
                        .expect("expected Ok(_)")
                },)*)))
            }
        }).await
    }};

    // ===== Normalize =====

    (@ { ( $($s:tt)* ) $($t:tt)* } $e:expr, $($r:tt)* ) => {
        $crate::try_join!(@{ ($($s)* _) $($t)* ($($s)*) $e, } $($r)*)
    };

    // ===== Entry point =====

    ( $($e:expr),* $(,)?) => {
        $crate::try_join!(@{ () } $($e,)*)
    };
}