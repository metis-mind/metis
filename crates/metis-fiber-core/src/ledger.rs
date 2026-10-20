//! The disposer ledger: one fiber's account book of undo actions (fiber.md
//! §1–§2). Every registered effect must record its undo action at the same
//! time; unload = settling the book page by page.
//!
//! The ledger is a dumb container: the *policy* points (refusing new accounts
//! while settling, entering drain only once) live in the fiber state machine,
//! not here. What the ledger owns is the ordering discipline:
//!
//! - **LIFO settlement** — disposers come out newest-first (later effects
//!   tend to depend on earlier ones; fiber.md §2.1).
//! - **Structural idempotence** — `begin_drain` moves the entries out, so a
//!   second drain finds an empty book. Combined with the state machine's
//!   single entry into `Unloading`, double drain cannot happen (§2.2).
//! - **Early cancellation** — one effect can be undone before any wholesale
//!   drain; removal preserves the order of the remaining entries.

use std::future::Future;
use std::pin::Pin;

use indexmap::IndexMap;

/// Unified async signature (ADR-0010 §8.3): every disposer returns a future
/// the runtime joins; a synchronous undo wraps into an immediately-ready one.
/// `Send` because the runtime drains on its own tasks.
pub type Disposer = Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>;

/// Monotonic per-fiber effect id; never reused within the fiber's life.
pub type EffectId = u64;

/// One fiber's disposer account book, in registration order.
#[derive(Default)]
pub struct Ledger {
    entries: IndexMap<EffectId, Disposer>,
    next_id: EffectId,
}

/// An in-progress LIFO settlement. Yields `(EffectId, Disposer)` pairs
/// newest-first; the glue executes each disposer (they may be async) and
/// reports the outcome back to the state machine.
pub struct Drain {
    inner: std::iter::Rev<indexmap::map::IntoIter<EffectId, Disposer>>,
}

impl Iterator for Drain {
    type Item = (EffectId, Disposer);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for Drain {}

#[cfg(kani)]
mod proofs {
    use super::*;
    use std::task::{Context, Poll};

    /// A trivially-ready future (keeps async-block lowering out of the
    /// proof; only ordering is proven, never execution).
    struct Ready;

    impl Future for Ready {
        type Output = ();

        fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<()> {
            Poll::Ready(())
        }
    }

    fn dummy() -> Disposer {
        Box::new(|| Box::pin(Ready))
    }

    /// Bounded inserts: the drain yields exactly the reverse registration
    /// order, consumes the book, and a second drain is empty.
    #[kani::proof]
    #[kani::unwind(6)]
    fn drain_is_exact_reverse_and_consuming() {
        let n: usize = kani::any();
        kani::assume((1..=4).contains(&n));
        let mut ledger = Ledger::new();
        for expected_id in 0..n as u64 {
            assert_eq!(ledger.record(dummy()), expected_id);
        }
        let drained: Vec<EffectId> = ledger.begin_drain().map(|(id, _)| id).collect();
        let mut expected: Vec<EffectId> = (0..n as u64).collect();
        expected.reverse();
        assert_eq!(drained, expected);
        assert_eq!(ledger.len(), 0, "drain consumes the book");
        assert_eq!(ledger.begin_drain().count(), 0, "second drain is empty");
    }
}

impl Ledger {
    /// An empty book.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a new effect's disposer; returns its id for later cancellation.
    pub fn record(&mut self, disposer: Disposer) -> EffectId {
        let id = self.next_id;
        self.next_id += 1;
        self.entries.insert(id, disposer);
        id
    }

    /// Undo one effect early, before any wholesale drain. Returns the
    /// disposer if the id was on the book; the remaining entries keep their
    /// registration order.
    pub fn cancel(&mut self, id: EffectId) -> Option<Disposer> {
        self.entries.shift_remove(&id)
    }

    /// Begin LIFO settlement. Consumes the current entries (the book is left
    /// empty), so a second drain has nothing to settle.
    pub fn begin_drain(&mut self) -> Drain {
        Drain {
            inner: std::mem::take(&mut self.entries).into_iter().rev(),
        }
    }

    /// Number of unsettled entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` when nothing is on the book.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::block_on_ready;
    use std::sync::{Arc, Mutex};

    /// A disposer that appends its tag to the shared log when run.
    fn tagged(log: &Arc<Mutex<Vec<u64>>>, tag: u64) -> Disposer {
        let log = Arc::clone(log);
        Box::new(move || {
            Box::pin(async move {
                log.lock().unwrap().push(tag);
            })
        })
    }

    #[test]
    fn drain_is_lifo_and_consumes_the_book() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut ledger = Ledger::new();
        let first = ledger.record(tagged(&log, 1));
        let second = ledger.record(tagged(&log, 2));
        let third = ledger.record(tagged(&log, 3));
        assert_eq!((first, second, third), (0, 1, 2));
        assert_eq!(ledger.len(), 3);

        let drain: Vec<_> = ledger.begin_drain().collect();
        assert!(ledger.is_empty(), "drain consumes the book");
        assert_eq!(
            drain.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            vec![third, second, first],
            "settlement runs newest-first"
        );
        for (_, disposer) in drain {
            block_on_ready(disposer());
        }
        assert_eq!(*log.lock().unwrap(), vec![3, 2, 1]);

        let again: Vec<_> = ledger.begin_drain().collect();
        assert!(again.is_empty(), "a second drain finds an empty book");
    }

    #[test]
    fn cancel_removes_one_entry_and_preserves_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut ledger = Ledger::new();
        let a = ledger.record(tagged(&log, 1));
        let b = ledger.record(tagged(&log, 2));
        let c = ledger.record(tagged(&log, 3));

        // Cancel the FIRST entry: swap_remove would move the tail into the
        // vacated slot (drain [b, c]); only shift_remove preserves [b, c]
        // registration order (drain [c, b]). This is what pins order
        // preservation against a mutation to swap_remove.
        let cancelled = ledger.cancel(a).expect("a is on the book");
        block_on_ready(cancelled());
        assert_eq!(*log.lock().unwrap(), vec![1]);
        assert_eq!(ledger.len(), 2);
        assert!(ledger.cancel(a).is_none(), "cancel is not repeatable");

        let remaining: Vec<u64> = ledger.begin_drain().map(|(id, _)| id).collect();
        assert_eq!(remaining, vec![c, b], "remaining order survives removal");
    }
}
