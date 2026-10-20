//! The fiber state machine (fiber.md §3): `state + event → state + actions`.
//!
//! The transition function is pure data in, pure data out — no IO, no clock,
//! no randomness. That is the shape the L2 model-based tests and the L4
//! targets require (ADR-0019 C2): the runtime glue interprets the returned
//! actions and reports their outcomes back as further events.
//!
//! Invariants the machine itself guarantees:
//!
//! - **`Unloading` is entered exactly once** and carries its landing target,
//!   so a failed load and a normal unload reuse the same drain path
//!   (fiber.md §3).
//! - **A fiber that reaches `Failed` or `Disposed` has an empty ledger**:
//!   both are only reachable past `DrainFinished` (or from `Pending`, which
//!   never had effects). Later stations' teardown relies on this.
//! - **Every event not listed is rejected**, loudly — the glue drives the
//!   machine and must not guess.

use thiserror::Error;

/// Lifecycle states (fiber.md §3 diagram). Enum-with-data, per the frozen
/// decision against typestate (§8.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FiberState {
    /// Config entry mounted; the loader has not picked the fiber up yet.
    Pending,
    /// Loader scheduled; setup running.
    Loading,
    /// Setup succeeded; the fiber serves traffic.
    Active,
    /// Settling the ledger. Carries where to land once the drain reports
    /// back, so failed loads and normal unloads share one drain path.
    Unloading(UnloadTarget),
    /// Fully settled. Terminal: the arena removes the husk.
    Disposed,
    /// Something failed; the report rides the state. Terminal until policy
    /// (human or automatic) disposes of it — the core never auto-retries.
    Failed(Report),
}

/// Where an `Unloading` fiber lands after a clean drain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnloadTarget {
    /// Normal unload (hot reload / disable / parent unload): land `Disposed`.
    Disposed,
    /// The load failed; the drain settles the half-built book, then land
    /// `Failed` with the load's report (fiber.md §3: drain the half-finished
    /// account first).
    Failed(Report),
}

/// An error report riding `FiberState::Failed`. Pure data; the runtime
/// renders it for journal/observability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    message: String,
}

impl Report {
    /// A report with a single message.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// The message text.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Compose the report for a drain that finished with errors. Disposer
    /// errors are swallowed one by one (they never interrupt the remaining
    /// cleanup — luau-abi A1.2, carried by wit-abi W4.5), so by the time the
    /// drain reports back there may be several; they aggregate here into the
    /// single `Report` the state carries. The aggregate is unbounded by
    /// construction — the honest bound is that drain errors are rare and
    /// per-fiber effect counts are modest; the glue truncates at render time
    /// (journal station) if a pathological drain ever says otherwise.
    fn drain_failure(load_failure: Option<&Report>, disposer_errors: &[Report]) -> Self {
        use std::fmt::Write as _;
        let mut message = String::new();
        if let Some(report) = load_failure {
            message.push_str("load failed: ");
            message.push_str(report.message());
            message.push_str("; ");
        }
        write!(
            message,
            "drain finished with {} disposer error(s)",
            disposer_errors.len()
        )
        .expect("writing to a String cannot fail");
        for error in disposer_errors {
            message.push_str(": ");
            message.push_str(error.message());
        }
        Self { message }
    }
}

impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// Inputs to the machine, reported by the glue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// The loader picked up a `Pending` fiber.
    StartLoad,
    /// Setup succeeded.
    LoadSucceeded,
    /// Setup failed; the report describes why.
    LoadFailed(Report),
    /// Unload requested (hot reload / disable / parent unload / shutdown).
    ///
    /// Rejected while `Loading`: a load must first conclude (succeed or
    /// fail — setup has its own budget, wit-abi W1.5), so the glue defers
    /// the unload until then. Rejected while `Unloading`: one drain is
    /// already running and the second request carries no information.
    Unload,
    /// The glue finished executing the drain. Disposer errors ride along
    /// (they never abort the remaining cleanup); an empty list means the
    /// book settled cleanly.
    DrainFinished { disposer_errors: Vec<Report> },
}

/// Action descriptions for the glue to interpret (ADR-0019 C2: data, not
/// effects).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Begin settling the ledger (LIFO). The glue takes the `Drain` from the
    /// fiber, executes each disposer (they may be async), then reports
    /// `Event::DrainFinished`.
    StartDrain,
}

/// A rejected transition.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("event {event} is not valid in state {state}")]
pub struct TransitionError {
    state: &'static str,
    event: &'static str,
}

impl FiberState {
    /// `state + event → state + actions`. The state is cloned, not moved, so
    /// a rejected transition leaves the machine untouched.
    pub fn transition(&self, event: Event) -> Result<(Self, Vec<Action>), TransitionError> {
        use FiberState as S;
        match (self, event) {
            (S::Pending, Event::StartLoad) => Ok((S::Loading, vec![])),
            // Pending was never loaded and Failed has an already-settled
            // book (see the type docs): neither has anything to drain, so
            // unload is bookkeeping, not a drain.
            (S::Pending | S::Failed(_), Event::Unload) => Ok((S::Disposed, vec![])),
            (S::Loading, Event::LoadSucceeded) => Ok((S::Active, vec![])),
            (S::Loading, Event::LoadFailed(report)) => Ok((
                S::Unloading(UnloadTarget::Failed(report)),
                vec![Action::StartDrain],
            )),
            (S::Active, Event::Unload) => Ok((
                S::Unloading(UnloadTarget::Disposed),
                vec![Action::StartDrain],
            )),
            (S::Unloading(target), Event::DrainFinished { disposer_errors }) => {
                let load_failure = match target {
                    UnloadTarget::Failed(report) => Some(report.clone()),
                    UnloadTarget::Disposed => None,
                };
                let landed = if disposer_errors.is_empty() {
                    match load_failure {
                        Some(report) => S::Failed(report),
                        None => S::Disposed,
                    }
                } else {
                    S::Failed(Report::drain_failure(
                        load_failure.as_ref(),
                        &disposer_errors,
                    ))
                };
                Ok((landed, vec![]))
            }
            (state, event) => Err(TransitionError {
                state: state.name(),
                event: event.name(),
            }),
        }
    }

    /// Static name for error reporting.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Loading => "Loading",
            Self::Active => "Active",
            Self::Unloading(_) => "Unloading",
            Self::Disposed => "Disposed",
            Self::Failed(_) => "Failed",
        }
    }
}

impl Event {
    /// Static name for error reporting.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::StartLoad => "StartLoad",
            Self::LoadSucceeded => "LoadSucceeded",
            Self::LoadFailed(_) => "LoadFailed",
            Self::Unload => "Unload",
            Self::DrainFinished { .. } => "DrainFinished",
        }
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;

    fn any_state() -> FiberState {
        match kani::any::<u8>() % 6 {
            0 => FiberState::Pending,
            1 => FiberState::Loading,
            2 => FiberState::Active,
            3 => FiberState::Unloading(UnloadTarget::Disposed),
            4 => FiberState::Unloading(UnloadTarget::Failed(Report::new("load"))),
            _ => FiberState::Failed(Report::new("failed")),
        }
    }

    fn any_event() -> Event {
        match kani::any::<u8>() % 6 {
            0 => Event::StartLoad,
            1 => Event::LoadSucceeded,
            2 => Event::LoadFailed(Report::new("boom")),
            3 => Event::Unload,
            4 => Event::DrainFinished {
                disposer_errors: vec![],
            },
            _ => {
                let n: usize = kani::any();
                kani::assume(n <= 2);
                Event::DrainFinished {
                    disposer_errors: (0..n).map(|i| Report::new(format!("e{i}"))).collect(),
                }
            }
        }
    }

    /// The acceptance grid, stated independently of the implementation.
    fn has_legal_outcome(before: &FiberState, event: &Event) -> bool {
        use FiberState as S;
        matches!(
            (before, event),
            (S::Pending, Event::StartLoad | Event::Unload)
                | (S::Loading, Event::LoadSucceeded | Event::LoadFailed(_))
                | (S::Active, Event::Unload)
                | (S::Unloading(_), Event::DrainFinished { .. })
                | (S::Failed(_), Event::Unload)
        )
    }

    /// The full legal-edge relation over outcomes, stated independently of
    /// the implementation (a third encoding, as the specification).
    fn legal(before: &FiberState, event: &Event, after: &FiberState, actions: &[Action]) -> bool {
        use FiberState as S;
        match (before, event, after, actions) {
            (S::Pending, Event::StartLoad, S::Loading, []) => true,
            (S::Pending, Event::Unload, S::Disposed, []) => true,
            (S::Loading, Event::LoadSucceeded, S::Active, []) => true,
            (
                S::Loading,
                Event::LoadFailed(given),
                S::Unloading(UnloadTarget::Failed(carried)),
                [Action::StartDrain],
            ) => given == carried,
            (
                S::Active,
                Event::Unload,
                S::Unloading(UnloadTarget::Disposed),
                [Action::StartDrain],
            ) => true,
            (S::Unloading(target), Event::DrainFinished { disposer_errors }, landed, []) => {
                let load_failure = match target {
                    UnloadTarget::Failed(report) => Some(report.clone()),
                    UnloadTarget::Disposed => None,
                };
                let expected = if disposer_errors.is_empty() {
                    match load_failure {
                        Some(report) => S::Failed(report),
                        None => S::Disposed,
                    }
                } else {
                    S::Failed(Report::drain_failure(
                        load_failure.as_ref(),
                        disposer_errors,
                    ))
                };
                *landed == expected
            }
            (S::Failed(_), Event::Unload, S::Disposed, []) => true,
            _ => false,
        }
    }

    /// Single-step transitions implement exactly the legal edges: every
    /// accepted step is legal, and every rejected (state, event) pair has
    /// no legal outcome. (Kani's hard bound = no concurrency, so this is
    /// the single-step/no-interleaving shape testing-strategy §2.1
    /// assigns; interleaving coverage belongs to L2.)
    #[kani::proof]
    fn single_step_implements_exactly_the_legal_edges() {
        let before = any_state();
        let event = any_event();
        match before.transition(event.clone()) {
            Ok((after, actions)) => assert!(
                legal(&before, &event, &after, &actions),
                "accepted an illegal edge"
            ),
            Err(_) => assert!(!has_legal_outcome(&before, &event), "rejected a legal edge"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(text: &str) -> Report {
        Report::new(text)
    }

    fn transition(state: &FiberState, event: Event) -> (FiberState, Vec<Action>) {
        state
            .transition(event)
            .expect("transition must be accepted")
    }

    #[test]
    fn happy_path_runs_pending_to_disposed() {
        let (state, actions) = transition(&FiberState::Pending, Event::StartLoad);
        assert_eq!(state, FiberState::Loading);
        assert!(actions.is_empty());

        let (state, actions) = transition(&state, Event::LoadSucceeded);
        assert_eq!(state, FiberState::Active);
        assert!(actions.is_empty());

        let (state, actions) = transition(&state, Event::Unload);
        assert_eq!(state, FiberState::Unloading(UnloadTarget::Disposed));
        assert_eq!(actions, vec![Action::StartDrain]);

        let (state, actions) = transition(
            &state,
            Event::DrainFinished {
                disposer_errors: vec![],
            },
        );
        assert_eq!(state, FiberState::Disposed);
        assert!(actions.is_empty());
    }

    #[test]
    fn failed_load_drains_then_lands_failed_with_the_load_report() {
        let (state, actions) = transition(&FiberState::Loading, Event::LoadFailed(report("boom")));
        assert_eq!(
            state,
            FiberState::Unloading(UnloadTarget::Failed(report("boom")))
        );
        assert_eq!(actions, vec![Action::StartDrain]);

        let (state, actions) = transition(
            &state,
            Event::DrainFinished {
                disposer_errors: vec![],
            },
        );
        assert_eq!(state, FiberState::Failed(report("boom")));
        assert!(actions.is_empty());
    }

    #[test]
    fn drain_errors_land_failed_with_an_aggregate_report() {
        let (state, _) = transition(&FiberState::Active, Event::Unload);
        let (state, _) = transition(
            &state,
            Event::DrainFinished {
                disposer_errors: vec![report("close failed"), report("flush failed")],
            },
        );
        let FiberState::Failed(report) = state else {
            panic!("drain errors must land Failed");
        };
        assert_eq!(
            report.message(),
            "drain finished with 2 disposer error(s): close failed: flush failed"
        );
    }

    #[test]
    fn load_failure_and_drain_errors_compose_into_one_report() {
        let (state, _) = transition(&FiberState::Loading, Event::LoadFailed(report("boom")));
        let (state, _) = transition(
            &state,
            Event::DrainFinished {
                disposer_errors: vec![report("close failed")],
            },
        );
        let FiberState::Failed(report) = state else {
            panic!("must land Failed");
        };
        assert_eq!(
            report.message(),
            "load failed: boom; drain finished with 1 disposer error(s): close failed"
        );
    }

    #[test]
    fn pending_unload_disposes_without_drain() {
        let (state, actions) = transition(&FiberState::Pending, Event::Unload);
        assert_eq!(state, FiberState::Disposed);
        assert!(actions.is_empty());
    }

    #[test]
    fn failed_unload_disposes_without_a_second_drain() {
        let (state, actions) = transition(&FiberState::Failed(report("boom")), Event::Unload);
        assert_eq!(state, FiberState::Disposed);
        assert!(actions.is_empty());
    }

    #[test]
    fn rejected_transitions_name_state_and_event_and_keep_the_state() {
        let cases: Vec<(FiberState, Event, &'static str, &'static str)> = vec![
            (FiberState::Active, Event::StartLoad, "Active", "StartLoad"),
            (
                FiberState::Pending,
                Event::LoadSucceeded,
                "Pending",
                "LoadSucceeded",
            ),
            (FiberState::Loading, Event::Unload, "Loading", "Unload"),
            (
                FiberState::Unloading(UnloadTarget::Disposed),
                Event::Unload,
                "Unloading",
                "Unload",
            ),
            (
                FiberState::Active,
                Event::DrainFinished {
                    disposer_errors: vec![],
                },
                "Active",
                "DrainFinished",
            ),
            (
                FiberState::Active,
                Event::LoadFailed(report("boom")),
                "Active",
                "LoadFailed",
            ),
            (FiberState::Disposed, Event::Unload, "Disposed", "Unload"),
            (
                FiberState::Failed(report("boom")),
                Event::StartLoad,
                "Failed",
                "StartLoad",
            ),
        ];
        for (state, event, state_name, event_name) in cases {
            let error = state
                .transition(event)
                .expect_err("transition must be rejected");
            assert_eq!(
                error.to_string(),
                format!("event {event_name} is not valid in state {state_name}")
            );
        }
    }

    #[test]
    fn transition_is_pure_rejection_keeps_the_input_state() {
        let state = FiberState::Active;
        let _ = state.transition(Event::StartLoad);
        assert_eq!(state, FiberState::Active, "rejection must not mutate");
    }
}
