//! Pure fiber core (ADR-0010, ADR-0019): the fiber state machine, disposer
//! ledger with LIFO drain ordering, epoch fingerprint comparison, and
//! dependent-closure computation over the reverse index.
//!
//! Purity is structural, not a convention: this crate must not depend on
//! tokio, mlua, or tracing. Transitions return action descriptions (data) for
//! the runtime glue to interpret, which is the shape the L2 model-based state
//! tests and the L4 formalization targets (Kani/hax) require.
