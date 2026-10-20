//! The fiber: one plugin instance's lifecycle container (fiber.md §0–§1) —
//! state machine + disposer ledger. It answers three questions: is this
//! instance alive, what has it done, how does it die cleanly.
//! (Parent/child links arrive with the arena, which owns the wiring.)
//!
//! Policy points the fiber owns on top of its parts:
//!
//! - **No new accounts while settling** (fiber.md §2.4, wit-abi W4.5
//!   D11.2): `record_disposer` / `cancel_disposer` are open only in
//!   `Loading` and `Active`. Effects that outlive the instance are
//!   meaningless once `Unloading` begins, so the registration channel
//!   closes loudly; in-flight *calls* of already-registered effects are
//!   not ledger operations and stay unaffected.
//! - **Drain is handed out once, in `Unloading` only**: `begin_drain`
//!   answers `Some` exactly where `Action::StartDrain` was emitted, and
//!   the consuming `Ledger::begin_drain` makes the second call empty.

use thiserror::Error;

use crate::ledger::{Disposer, Drain, EffectId, Ledger};
use crate::state::{Action, Event, FiberState, TransitionError};

/// The registration channel is closed outside `Loading`/`Active`.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
#[error("effects can only be registered or cancelled while Loading or Active, not {state}")]
pub struct LedgerClosed {
    state: &'static str,
}

/// One plugin instance's lifecycle container.
pub struct Fiber {
    state: FiberState,
    ledger: Ledger,
}

impl Default for Fiber {
    fn default() -> Self {
        Self {
            state: FiberState::Pending,
            ledger: Ledger::default(),
        }
    }
}

impl Fiber {
    /// A fresh fiber in `Pending`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Current lifecycle state.
    #[must_use]
    pub fn state(&self) -> &FiberState {
        &self.state
    }

    /// Number of unsettled disposers on the book (observability).
    #[must_use]
    pub fn ledger_len(&self) -> usize {
        self.ledger.len()
    }

    /// Drive the state machine. On success the new state is installed and
    /// the action descriptions are returned for the glue to interpret; a
    /// rejection leaves the fiber untouched.
    pub fn transition(&mut self, event: Event) -> Result<Vec<Action>, TransitionError> {
        let (state, actions) = self.state.transition(event)?;
        // Terminal states require a settled book (state.rs type docs): the
        // glue must take the drain before reporting DrainFinished, otherwise
        // leftover disposers are dropped uncalled — their undo silently lost.
        // Debug-only: the glue-trusting boundary stays zero-cost in release.
        debug_assert!(
            !matches!(state, FiberState::Disposed | FiberState::Failed(_))
                || self.ledger.is_empty(),
            "terminal state with an unsettled ledger: drain before DrainFinished"
        );
        self.state = state;
        Ok(actions)
    }

    /// Record a new effect's disposer. Closed outside `Loading`/`Active`
    /// (module docs: no new accounts while settling).
    pub fn record_disposer(&mut self, disposer: Disposer) -> Result<EffectId, LedgerClosed> {
        self.guard_ledger()?;
        Ok(self.ledger.record(disposer))
    }

    /// Undo one effect early. Same gate as `record_disposer`: once settling
    /// begins the whole book belongs to the drain, and pulling an entry out
    /// would leak that effect.
    pub fn cancel_disposer(&mut self, id: EffectId) -> Result<Option<Disposer>, LedgerClosed> {
        self.guard_ledger()?;
        Ok(self.ledger.cancel(id))
    }

    fn guard_ledger(&self) -> Result<(), LedgerClosed> {
        match &self.state {
            FiberState::Loading | FiberState::Active => Ok(()),
            state => Err(LedgerClosed {
                state: state.name(),
            }),
        }
    }

    /// Hand the LIFO drain to the glue. `Some` only in `Unloading` — the
    /// state `Action::StartDrain` was emitted in; the second call finds an
    /// empty book (consuming drain, `Ledger::begin_drain`).
    #[must_use]
    pub fn begin_drain(&mut self) -> Option<Drain> {
        match &self.state {
            FiberState::Unloading(_) => Some(self.ledger.begin_drain()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Report, UnloadTarget};
    use crate::testutil::block_on_ready;
    use std::sync::{Arc, Mutex};

    fn tagged(log: &Arc<Mutex<Vec<u64>>>, tag: u64) -> Disposer {
        let log = Arc::clone(log);
        Box::new(move || {
            Box::pin(async move {
                log.lock().unwrap().push(tag);
            })
        })
    }

    fn noop() -> Disposer {
        Box::new(|| Box::pin(async {}))
    }

    fn load_fiber(fiber: &mut Fiber) {
        fiber.transition(Event::StartLoad).unwrap();
    }

    fn activate_fiber(fiber: &mut Fiber) {
        load_fiber(fiber);
        fiber.transition(Event::LoadSucceeded).unwrap();
    }

    #[test]
    fn full_lifecycle_records_drains_lifo_and_disposes() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut fiber = Fiber::new();
        assert_eq!(fiber.state(), &FiberState::Pending);

        load_fiber(&mut fiber);
        let a = fiber.record_disposer(tagged(&log, 1)).unwrap();
        fiber.transition(Event::LoadSucceeded).unwrap();
        let b = fiber.record_disposer(tagged(&log, 2)).unwrap();
        assert_eq!(fiber.ledger_len(), 2);

        let actions = fiber.transition(Event::Unload).unwrap();
        assert_eq!(actions, vec![Action::StartDrain]);
        assert_eq!(
            fiber.state(),
            &FiberState::Unloading(UnloadTarget::Disposed)
        );

        let drain: Vec<_> = fiber
            .begin_drain()
            .expect("StartDrain was emitted")
            .collect();
        assert_eq!(
            drain.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            vec![b, a],
            "settlement is LIFO"
        );
        assert_eq!(fiber.ledger_len(), 0, "drain consumes the book");
        for (_, disposer) in drain {
            block_on_ready(disposer());
        }
        assert_eq!(*log.lock().unwrap(), vec![2, 1]);

        fiber
            .transition(Event::DrainFinished {
                disposer_errors: vec![],
            })
            .unwrap();
        assert_eq!(fiber.state(), &FiberState::Disposed);
    }

    #[test]
    fn ledger_closes_outside_loading_and_active() {
        let mut fiber = Fiber::new();
        let error = fiber
            .record_disposer(noop())
            .expect_err("Pending is closed");
        assert_eq!(
            error.to_string(),
            "effects can only be registered or cancelled while Loading or Active, not Pending"
        );
        assert!(fiber.cancel_disposer(0).is_err());

        load_fiber(&mut fiber);
        assert!(fiber.record_disposer(noop()).is_ok(), "Loading is open");
        fiber.transition(Event::LoadSucceeded).unwrap();
        assert!(fiber.record_disposer(noop()).is_ok(), "Active is open");

        fiber.transition(Event::Unload).unwrap();
        let error = fiber
            .record_disposer(noop())
            .expect_err("Unloading is closed");
        assert_eq!(
            error.to_string(),
            "effects can only be registered or cancelled while Loading or Active, not Unloading"
        );
        assert!(fiber.cancel_disposer(0).is_err());

        // Terminal states require a settled book (debug_assert in
        // Fiber::transition): settle before reporting DrainFinished.
        assert_eq!(fiber.begin_drain().map(|d| d.len()), Some(2));
        fiber
            .transition(Event::DrainFinished {
                disposer_errors: vec![],
            })
            .unwrap();
        assert!(
            fiber.record_disposer(noop()).is_err(),
            "Disposed stays closed"
        );
    }

    #[test]
    fn failed_load_still_settles_the_half_built_book() {
        let mut fiber = Fiber::new();
        load_fiber(&mut fiber);
        fiber.record_disposer(noop()).unwrap();

        let actions = fiber
            .transition(Event::LoadFailed(Report::new("boom")))
            .unwrap();
        assert_eq!(actions, vec![Action::StartDrain]);
        assert_eq!(fiber.begin_drain().map(|d| d.len()), Some(1));

        fiber
            .transition(Event::DrainFinished {
                disposer_errors: vec![],
            })
            .unwrap();
        assert_eq!(
            fiber.state(),
            &FiberState::Failed(Report::new("boom")),
            "lands Failed with the load report"
        );
        assert!(fiber.begin_drain().is_none(), "Failed hands out no drain");
        assert!(
            fiber.record_disposer(noop()).is_err(),
            "Failed keeps the ledger closed"
        );
    }

    #[test]
    fn begin_drain_is_unloading_only_and_consuming() {
        let mut fiber = Fiber::new();
        assert!(fiber.begin_drain().is_none(), "Pending has no drain");
        activate_fiber(&mut fiber);
        assert!(fiber.begin_drain().is_none(), "Active has no drain");
        fiber.record_disposer(noop()).unwrap();

        fiber.transition(Event::Unload).unwrap();
        assert_eq!(fiber.begin_drain().map(|d| d.len()), Some(1));
        assert_eq!(
            fiber.begin_drain().map(|d| d.len()),
            Some(0),
            "the second call finds the consumed book"
        );
    }

    #[test]
    fn cancel_disposer_undoes_one_effect_early() {
        let mut fiber = Fiber::new();
        activate_fiber(&mut fiber);
        let a = fiber.record_disposer(noop()).unwrap();
        fiber.record_disposer(noop()).unwrap();

        assert!(fiber.cancel_disposer(a).unwrap().is_some());
        assert_eq!(fiber.ledger_len(), 1);
        assert!(fiber.cancel_disposer(a).unwrap().is_none());
    }
}
