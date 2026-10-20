//! L2 model-based state tests (testing-strategy §2, L2 row): the fiber
//! lifecycle, driven by random operation sequences, must agree with a small
//! independently written reference model — state, unsettled count,
//! rejection behavior, and disposer execution order (LIFO). The model is a
//! second encoding of the lifecycle rules on purpose: agreement between
//! two structurally different encodings over thousands of random paths is
//! the evidence this layer exists for.

use std::sync::{Arc, Mutex};

use metis_fiber_core::{Action, Disposer, Drain, EffectId, Event, Fiber, FiberState, Report};
use proptest::prelude::*;

/// Operations the glue can perform. `CancelEffect` carries an index into
/// the live-effects list (modulo its length), mirroring a plugin naming one
/// of its own effects.
#[derive(Clone, Debug)]
enum Op {
    StartLoad,
    LoadSucceed,
    LoadFail,
    RecordEffect,
    CancelEffect(usize),
    Unload,
    DrainStep,
    DrainFinish { with_errors: bool },
}

fn ops() -> impl Strategy<Value = Vec<Op>> {
    let op = prop_oneof![
        2 => Just(Op::StartLoad),
        2 => Just(Op::LoadSucceed),
        1 => Just(Op::LoadFail),
        4 => Just(Op::RecordEffect),
        1 => any::<usize>().prop_map(Op::CancelEffect),
        2 => Just(Op::Unload),
        3 => Just(Op::DrainStep),
        2 => any::<bool>().prop_map(|with_errors| Op::DrainFinish { with_errors }),
    ];
    proptest::collection::vec(op, 1..40)
}

/// The reference model: lifecycle state plus the book, as plainly as
/// possible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MState {
    Pending,
    Loading,
    Active,
    Unloading,
    Disposed,
    Failed,
}

struct Model {
    state: MState,
    /// Unsettled effect tags, in registration order (whether still on the
    /// book or already handed to the drain).
    live: Vec<u64>,
    /// Executed disposer tags, in execution order.
    executed: Vec<u64>,
    /// Landing target while Unloading: `true` = Failed (the load failed).
    fail_pending: bool,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            state: MState::Pending,
            live: Vec::new(),
            executed: Vec::new(),
            fail_pending: false,
        }
    }
}

/// The system under test: the fiber plus the glue-side drain handle, and
/// the live effect ids in registration order (parallel to `Model::live`).
#[derive(Default)]
struct Sut {
    fiber: Fiber,
    drain: Option<Drain>,
    live_ids: Vec<EffectId>,
    log: Arc<Mutex<Vec<u64>>>,
}

impl Sut {
    fn unsettled(&self) -> usize {
        self.fiber.ledger_len() + self.drain.as_ref().map_or(0, Drain::len)
    }

    fn executed(&self) -> Vec<u64> {
        self.log.lock().unwrap().clone()
    }

    fn mstate(&self) -> MState {
        match self.fiber.state() {
            FiberState::Pending => MState::Pending,
            FiberState::Loading => MState::Loading,
            FiberState::Active => MState::Active,
            FiberState::Unloading(_) => MState::Unloading,
            FiberState::Disposed => MState::Disposed,
            FiberState::Failed(_) => MState::Failed,
        }
    }

    /// Pick up the drain the machine announced (idempotent).
    fn take_drain(&mut self) {
        if self.drain.is_none() {
            self.drain = self.fiber.begin_drain();
        }
    }

    /// Execute one disposer and drop its id from the live list.
    fn execute(&mut self, id: EffectId, disposer: Disposer) {
        block_on_ready(disposer());
        self.live_ids.retain(|x| *x != id);
    }
}

/// Run an already-ready future to completion (tests never suspend).
fn block_on_ready<F: std::future::Future>(future: F) -> F::Output {
    use std::task::{Context, Poll, Waker};
    let waker = Waker::noop().clone();
    let mut cx = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut cx) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("test disposers are immediately ready"),
    }
}

/// One model step; returns `true` when the operation is accepted. `tag` is
/// the next free effect tag; the caller increments it when both sides
/// accept a `RecordEffect`.
fn model_apply(model: &mut Model, op: &Op, tag: u64) -> bool {
    match (model.state, op) {
        (MState::Pending, Op::StartLoad) => model.state = MState::Loading,
        (MState::Loading, Op::LoadSucceed) => model.state = MState::Active,
        (MState::Loading, Op::LoadFail) => {
            model.state = MState::Unloading;
            model.fail_pending = true;
        }
        (MState::Active, Op::Unload) => model.state = MState::Unloading,
        (MState::Loading | MState::Active, Op::RecordEffect) => {
            model.live.push(tag);
        }
        (MState::Loading | MState::Active, Op::CancelEffect(index)) => {
            if model.live.is_empty() {
                return false;
            }
            model.live.remove(index % model.live.len());
        }
        (MState::Unloading, Op::DrainStep) => {
            let Some(tag) = model.live.pop() else {
                return false;
            };
            model.executed.push(tag);
        }
        (MState::Unloading, Op::DrainFinish { with_errors }) => {
            // The glue contract: settle the whole book, then report. The
            // remaining disposers execute newest-first.
            while let Some(tag) = model.live.pop() {
                model.executed.push(tag);
            }
            model.state = if model.fail_pending || *with_errors {
                MState::Failed
            } else {
                MState::Disposed
            };
        }
        (MState::Failed | MState::Pending, Op::Unload) => model.state = MState::Disposed,
        _ => return false,
    }
    true
}

/// One SUT step; returns `true` when the operation is accepted.
fn sut_apply(sut: &mut Sut, op: &Op, tag: u64) -> bool {
    match op {
        Op::StartLoad => sut.fiber.transition(Event::StartLoad).is_ok(),
        Op::LoadSucceed => sut.fiber.transition(Event::LoadSucceeded).is_ok(),
        Op::LoadFail => match sut.fiber.transition(Event::LoadFailed(Report::new("boom"))) {
            Ok(actions) => {
                assert_eq!(
                    actions,
                    vec![Action::StartDrain],
                    "a failed load must announce the drain"
                );
                true
            }
            Err(_) => false,
        },
        Op::RecordEffect => {
            let log = Arc::clone(&sut.log);
            let disposer: Disposer = Box::new(move || {
                Box::pin(async move {
                    log.lock().unwrap().push(tag);
                })
            });
            match sut.fiber.record_disposer(disposer) {
                Ok(id) => {
                    sut.live_ids.push(id);
                    true
                }
                Err(_) => false,
            }
        }
        Op::CancelEffect(index) => {
            if sut.live_ids.is_empty() {
                // Mirror the model's rejection path (empty book) honestly:
                // cancel of a missing id is a `None`, still an accepted call.
                return sut.fiber.cancel_disposer(0).unwrap_or(None).is_some();
            }
            let id = sut.live_ids[index % sut.live_ids.len()];
            if sut.fiber.cancel_disposer(id).ok().flatten().is_none() {
                return false;
            }
            sut.live_ids.retain(|x| *x != id);
            true
        }
        Op::Unload => match sut.fiber.transition(Event::Unload) {
            Ok(actions) => {
                // StartDrain is the glue's drain signal: exactly the
                // transitions that land in Unloading carry it (Pending and
                // Failed dispose without one).
                assert_eq!(
                    matches!(sut.fiber.state(), FiberState::Unloading(_)),
                    actions == vec![Action::StartDrain],
                    "StartDrain must be emitted exactly when entering Unloading"
                );
                true
            }
            Err(_) => false,
        },
        Op::DrainStep => {
            sut.take_drain();
            let Some(drain) = sut.drain.as_mut() else {
                return false;
            };
            let Some((id, disposer)) = drain.next() else {
                return false;
            };
            sut.execute(id, disposer);
            true
        }
        Op::DrainFinish { with_errors } => {
            // Settle the whole book first (glue contract), then report.
            sut.take_drain();
            if let Some(drain) = sut.drain.take() {
                for (id, disposer) in drain {
                    sut.execute(id, disposer);
                }
            }
            let disposer_errors = if *with_errors {
                vec![Report::new("disposer error")]
            } else {
                vec![]
            };
            sut.fiber
                .transition(Event::DrainFinished { disposer_errors })
                .is_ok()
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        // Regression files live in VCS under the §3.H-canonical location;
        // Direct pins the path so the corpus lands in
        // tests/proptest-regressions/ instead of a per-source-file dir.
        failure_persistence: Some(Box::new(
            proptest::test_runner::FileFailurePersistence::Direct(
                "tests/proptest-regressions/lifecycle_model.txt",
            ),
        )),
        ..ProptestConfig::default()
    })]

    #[test]
    fn fiber_lifecycle_agrees_with_the_reference_model(sequence in ops()) {
        let mut model = Model::default();
        let mut sut = Sut::default();
        let mut next_tag = 0_u64;

        for (step, op) in sequence.iter().enumerate() {
            let expected = model_apply(&mut model, op, next_tag);
            let actual = sut_apply(&mut sut, op, next_tag);
            if expected && actual && matches!(op, Op::RecordEffect) {
                next_tag += 1;
            }
            let sut_state = sut.mstate();
            prop_assert_eq!(
                actual,
                expected,
                "step {}: acceptance of {:?} disagrees (model state: {:?}, sut state: {:?})",
                step,
                op,
                model.state,
                sut_state,
            );
            prop_assert_eq!(
                sut_state,
                model.state,
                "step {}: states diverge after {:?}",
                step,
                op,
            );
            prop_assert_eq!(
                sut.unsettled(),
                model.live.len(),
                "step {}: unsettled count diverges after {:?}",
                step,
                op,
            );
            let sut_executed = sut.executed();
            prop_assert_eq!(
                sut_executed.as_slice(),
                model.executed.as_slice(),
                "step {}: disposer execution order diverges after {:?} (LIFO contract)",
                step,
                op,
            );
        }
    }
}
