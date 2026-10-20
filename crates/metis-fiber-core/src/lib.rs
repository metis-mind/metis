//! Pure fiber core (ADR-0010, ADR-0019): the fiber state machine, disposer
//! ledger with LIFO drain ordering, epoch fingerprint comparison, and
//! dependent-closure computation over the reverse index.
//!
//! Purity is structural, not a convention: this crate must not depend on
//! tokio, mlua, or tracing. Transitions return action descriptions (data) for
//! the runtime glue to interpret, which is the shape the L2 model-based state
//! tests and the L4 formalization targets (Kani/hax) require.
//!
//! # Testing provenance (D11)
//!
//! Composition (agent-written throughout; maintainer review happens at
//! station acceptance per the operating protocol): L0 unit tests (in-file
//! `mod tests`), an L1 property suite with an independent naive-fixpoint
//! oracle (epoch closure, including id/topology-decorrelated ordering), an
//! L2 dual-reference-model suite over the lifecycle
//! (`tests/lifecycle_model.rs`), L4 Kani proof harnesses (`#[cfg(kani)] mod
//! proofs`; verified by the weekly slow-track leg, never `just ci`), and the
//! D6 memory-return leg (`tests/memory.rs`, `#[ignore]`, run via
//! `just deep-memory`).
//!
//! Mutation audit (testing-strategy §3.C, on-demand run): cargo-mutants
//! 27.1.0, 2026-10-20 — 91 mutants, 78 caught, 13 unviable, 0 missed, 0
//! timeouts. Exemptions live in `.cargo/mutants.toml` with per-entry reasons
//! (the `#[cfg(kani)]` proof modules and `Arena::with_capacity`). Repro:
//! `cargo mutants -p metis-fiber-core --test-tool nextest`.

mod arena;
mod epoch;
mod fiber;
mod id;
mod ledger;
mod state;

pub use arena::Arena;
pub use epoch::{Epoch, RestartPlan, invalidation_closure, is_stale};
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
