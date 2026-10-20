//! D6 fiber-scale memory-return leg (testing-strategy §3.E): a
//! create → unload → drain → teardown cycle at scale must hand every byte
//! back. Ignored in the fast track; the slow track runs it via
//! `just deep-memory`.
//!
//! The assertion is exact equality, not a slack multiple: the counting
//! allocator (metis-test-support) sees allocation *requests*, not RSS, so a
//! fully balanced cycle returns the counters to their baseline precisely.
//! Anything left over is a leak the fast track's structural tests cannot
//! see (an unconsumed disposer, a husk still owned by the arena).

use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

use metis_fiber_core::{Arena, Event, Fiber};
use metis_test_support::CountingAllocator;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator::new();

/// Big enough for size-class effects to show, small enough for the weekly
/// leg to stay noise (the cycle costs microseconds per fiber).
const FIBERS: usize = 4096;
const EFFECTS_PER_FIBER: usize = 8;

/// Run an already-ready future to completion (the test's disposers never
/// suspend); a local copy of the crate's `testutil::block_on_ready`, which
/// integration tests cannot reach.
fn block_on_ready<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop().clone();
    let mut cx = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut cx) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("test disposers must be immediately ready"),
    }
}

/// One create → unload → drain → teardown cycle over the arena; returns
/// the number of disposers actually executed.
fn cycle(arena: &mut Arena) -> usize {
    let executed = Arc::new(AtomicUsize::new(0));
    let mut ids = Vec::with_capacity(FIBERS);
    for _ in 0..FIBERS {
        let mut fiber = Fiber::new();
        fiber.transition(Event::StartLoad).unwrap();
        fiber.transition(Event::LoadSucceeded).unwrap();
        for _ in 0..EFFECTS_PER_FIBER {
            let executed = Arc::clone(&executed);
            fiber
                .record_disposer(Box::new(move || {
                    Box::pin(async move {
                        executed.fetch_add(1, Ordering::Relaxed);
                    })
                }))
                .unwrap();
        }
        ids.push(arena.insert(fiber));
    }
    for &id in &ids {
        let fiber = arena.get_mut(id).unwrap();
        fiber.transition(Event::Unload).unwrap();
        let drain = fiber.begin_drain().expect("Unload announces the drain");
        for (_, disposer) in drain {
            block_on_ready(disposer());
        }
        fiber
            .transition(Event::DrainFinished {
                disposer_errors: vec![],
            })
            .unwrap();
        assert!(arena.teardown(id).is_some(), "a Disposed husk tears down");
    }
    assert!(arena.is_empty(), "teardown empties the arena");
    executed.load(Ordering::Relaxed)
}

#[test]
#[ignore = "slow-track leg (testing-strategy §2.4): run via `just deep-memory`"]
fn fiber_memory_returns_to_baseline_after_full_teardown() {
    let mut arena = Arena::new();
    assert_eq!(cycle(&mut arena), FIBERS * EFFECTS_PER_FIBER);
    // Warmup complete: one-time growth the cycle cannot hand back (arena
    // slot-capacity retention, lazy std initialization) is now inside the
    // baseline.
    let baseline_bytes = GLOBAL.live_bytes();
    let baseline_allocs = GLOBAL.live_allocations();

    assert_eq!(cycle(&mut arena), FIBERS * EFFECTS_PER_FIBER);
    assert_eq!(
        GLOBAL.live_bytes(),
        baseline_bytes,
        "a fully drained cycle must hand every byte back"
    );
    assert_eq!(
        GLOBAL.live_allocations(),
        baseline_allocs,
        "allocation blocks return to the baseline too"
    );
}
