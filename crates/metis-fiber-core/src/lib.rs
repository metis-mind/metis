//! Pure fiber core (ADR-0010, ADR-0019): the fiber state machine, disposer
//! ledger with LIFO drain ordering, epoch fingerprint comparison, and
//! dependent-closure computation over the reverse index.
//!
//! Purity is structural, not a convention: this crate must not depend on
//! tokio, mlua, or tracing. Transitions return action descriptions (data) for
//! the runtime glue to interpret, which is the shape the L2 model-based state
//! tests and the L4 formalization targets (Kani/hax) require.

mod arena;
mod fiber;
mod id;
mod ledger;
mod state;

pub use arena::Arena;
pub use fiber::{Fiber, LedgerClosed};
pub use id::FiberId;
pub use ledger::{Disposer, Drain, EffectId, Ledger};
pub use state::{Action, Event, FiberState, Report, TransitionError, UnloadTarget};

/// Test-only helpers shared across modules.
#[cfg(test)]
pub(crate) mod testutil {
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    /// Run an already-ready future to completion (tests never suspend).
    pub(crate) fn block_on_ready<F: Future>(future: F) -> F::Output {
        let waker = Waker::noop().clone();
        let mut cx = Context::from_waker(&waker);
        let mut future = std::pin::pin!(future);
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("test disposers must be immediately ready"),
        }
    }
}
