//! The fiber arena (fiber.md §4): every fiber is centrally owned here; the
//! outside world only ever holds `FiberId`s. Slot reuse bumps the
//! generation, so a retired fiber's id fails lookups automatically — the
//! ABA protection hot reload depends on.
//!
//! Two rules the arena itself enforces:
//!
//! - **Teardown is `Disposed`-only.** Removing a live fiber would orphan its
//!   unsettled book (and its disposers would be dropped uncalled). Failed
//!   fibers go through `Event::Unload` first — one door into the grave, and
//!   the state machine guards it. Teardown also cleans the husk's backlink
//!   from a still-live parent (one lookup, not a tree walk — the children
//!   direction is glue's ordering business, §8.4), so a parent's child list
//!   only ever names live fibers.
//! - **Topology is written here, not by callers.** `attach_child` /
//!   `detach_child` keep the parent and child links in sync from one place;
//!   the "child = one effect of the parent" rule (fiber.md §8.4) stays with
//!   the glue, which records the child's unload as a disposer in the
//!   parent's ledger (LIFO settles children first for free). One parent at
//!   a time: re-parenting is detach-then-attach, never a silent rewrite.

use slotmap::SlotMap;

use crate::fiber::Fiber;
use crate::id::FiberId;
use crate::state::FiberState;

/// The runtime-wide owner of all fibers.
#[derive(Default)]
pub struct Arena {
    fibers: SlotMap<FiberId, Fiber>,
}

impl Arena {
    /// An empty arena.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty arena with room for `capacity` fibers before reallocating.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            fibers: SlotMap::with_capacity_and_key(capacity),
        }
    }

    /// Give the arena ownership of a fiber; returns its id.
    pub fn insert(&mut self, fiber: Fiber) -> FiberId {
        self.fibers.insert(fiber)
    }

    /// Look up a fiber. Stale ids (reused slot, old generation) answer
    /// `None` — that is the ABA protection working, not an error.
    #[must_use]
    pub fn get(&self, id: FiberId) -> Option<&Fiber> {
        self.fibers.get(id)
    }

    /// Look up a fiber mutably, with the same stale-id semantics as `get`.
    pub fn get_mut(&mut self, id: FiberId) -> Option<&mut Fiber> {
        self.fibers.get_mut(id)
    }

    /// Number of fibers currently owned (including disposed husks awaiting
    /// teardown).
    #[must_use]
    pub fn len(&self) -> usize {
        self.fibers.len()
    }

    /// `true` when no fibers are owned.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fibers.is_empty()
    }

    /// Iterate `(id, &fiber)` pairs in no guaranteed order.
    pub fn iter(&self) -> impl Iterator<Item = (FiberId, &Fiber)> {
        self.fibers.iter()
    }

    /// Remove a disposed husk, freeing the slot (and bumping its
    /// generation). Returns `None` for stale ids and for fibers that have
    /// not reached `Disposed` — teardown of a live fiber is refused (module
    /// docs). On success the husk is handed back for the glue's
    /// journal/observability trail; its backlink is cleaned from a
    /// still-live parent's child list on the way out.
    pub fn teardown(&mut self, id: FiberId) -> Option<Fiber> {
        match self.fibers.get(id) {
            Some(fiber) if matches!(fiber.state(), FiberState::Disposed) => {
                let parent = fiber.parent();
                let removed = self.fibers.remove(id);
                if let Some(parent) = parent
                    && let Some(parent_fiber) = self.fibers.get_mut(parent)
                {
                    parent_fiber.remove_child(id);
                }
                removed
            }
            _ => None,
        }
    }

    /// Link `child` under `parent` (both directions kept in sync). Returns
    /// `false` without changing anything when either id is stale, when
    /// asked to parent a fiber to itself, or when the child already has a
    /// parent — re-parenting is detach-then-attach (module docs).
    pub fn attach_child(&mut self, parent: FiberId, child: FiberId) -> bool {
        if parent == child || !self.fibers.contains_key(parent) || !self.fibers.contains_key(child)
        {
            return false;
        }
        if self.fibers[child].parent().is_some() {
            return false;
        }
        self.fibers[parent].push_child(child);
        self.fibers[child].set_parent(Some(parent));
        true
    }

    /// Drop the parent/child link (both directions). Returns `false` when
    /// either id is stale; an absent link is a no-op (`true` — idempotent,
    /// like the drain it usually rides on).
    pub fn detach_child(&mut self, parent: FiberId, child: FiberId) -> bool {
        if !self.fibers.contains_key(parent) || !self.fibers.contains_key(child) {
            return false;
        }
        self.fibers[parent].remove_child(child);
        if self.fibers[child].parent() == Some(parent) {
            self.fibers[child].set_parent(None);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Event;

    /// Drive a fiber Pending → Disposed (no disposers on the book).
    fn dispose(fiber: &mut Fiber) {
        fiber.transition(Event::StartLoad).unwrap();
        fiber.transition(Event::LoadSucceeded).unwrap();
        fiber.transition(Event::Unload).unwrap();
        fiber
            .transition(Event::DrainFinished {
                disposer_errors: vec![],
            })
            .unwrap();
    }

    fn disposed_fiber() -> Fiber {
        let mut fiber = Fiber::new();
        dispose(&mut fiber);
        fiber
    }

    #[test]
    fn insert_and_lookup_roundtrip() {
        let mut arena = Arena::new();
        assert!(arena.is_empty());
        let id = arena.insert(Fiber::new());
        assert_eq!(arena.len(), 1);
        assert!(!arena.is_empty(), "a non-empty arena answers false");
        assert!(arena.get(id).is_some());
        assert!(arena.get_mut(id).is_some());
        let ids: Vec<FiberId> = arena.iter().map(|(id, _)| id).collect();
        assert_eq!(ids, vec![id]);
    }

    #[test]
    fn get_mut_mutates_the_owned_fiber() {
        // Mutation through `get_mut` must land on the fiber `get` returns —
        // a `get_mut` that hands out anything else (e.g. a default husk)
        // fails here.
        let mut arena = Arena::new();
        let id = arena.insert(Fiber::new());
        arena
            .get_mut(id)
            .unwrap()
            .transition(Event::StartLoad)
            .unwrap();
        assert!(matches!(
            arena.get(id).unwrap().state(),
            FiberState::Loading
        ));
    }

    #[test]
    fn with_capacity_is_a_plain_arena_after_warmup() {
        let mut arena = Arena::with_capacity(4);
        let a = arena.insert(Fiber::new());
        let b = arena.insert(disposed_fiber());
        assert_eq!(arena.len(), 2);
        assert!(arena.teardown(b).is_some());
        assert!(arena.get(a).is_some());
        assert!(arena.get(b).is_none());
    }

    #[test]
    fn stale_ids_fail_after_slot_reuse() {
        let mut arena = Arena::new();
        let old = arena.insert(disposed_fiber());
        assert!(arena.teardown(old).is_some());
        let new = arena.insert(Fiber::new());
        // slotmap reuses freed slots with a bumped generation: the next
        // insert lands in the same slot but issues a different key.
        assert_ne!(old, new, "generation must bump on reuse");
        assert!(arena.get(new).is_some(), "the new generation owns the slot");
        assert!(arena.get(old).is_none(), "the stale id is dead");
        assert!(arena.teardown(old).is_none(), "stale teardown is refused");
    }

    #[test]
    fn teardown_is_disposed_only() {
        let mut arena = Arena::new();
        let pending = arena.insert(Fiber::new());
        assert!(
            arena.teardown(pending).is_none(),
            "teardown of a live fiber is refused"
        );
        assert_eq!(arena.len(), 1, "the fiber survives the refusal");

        let disposed = arena.insert(disposed_fiber());
        assert!(arena.teardown(disposed).is_some());
        assert!(arena.get(disposed).is_none());
        assert!(arena.teardown(disposed).is_none(), "stale id teardown");
    }

    #[test]
    fn topology_links_stay_in_sync() {
        let mut arena = Arena::new();
        let parent = arena.insert(Fiber::new());
        let child = arena.insert(Fiber::new());

        assert!(arena.attach_child(parent, child));
        assert_eq!(arena.get(parent).unwrap().children(), &[child]);
        assert_eq!(arena.get(child).unwrap().parent(), Some(parent));

        assert!(arena.detach_child(parent, child));
        assert!(arena.get(parent).unwrap().children().is_empty());
        assert_eq!(arena.get(child).unwrap().parent(), None);
        // Idempotent: detaching an absent link is a no-op success.
        assert!(arena.detach_child(parent, child));
    }

    #[test]
    fn attach_rejects_double_attach_and_reparenting() {
        let mut arena = Arena::new();
        let p1 = arena.insert(Fiber::new());
        let p2 = arena.insert(Fiber::new());
        let child = arena.insert(Fiber::new());

        assert!(arena.attach_child(p1, child));
        assert!(
            !arena.attach_child(p1, child),
            "double attach would duplicate the child link"
        );
        assert_eq!(arena.get(p1).unwrap().children(), &[child]);
        assert!(
            !arena.attach_child(p2, child),
            "re-parenting silently rewrites the link; detach first"
        );
        assert_eq!(arena.get(child).unwrap().parent(), Some(p1));

        assert!(arena.detach_child(p1, child));
        assert!(arena.attach_child(p2, child), "attach after detach works");
        assert!(arena.get(p1).unwrap().children().is_empty());
        assert_eq!(arena.get(p2).unwrap().children(), &[child]);
    }

    #[test]
    fn teardown_cleans_the_parent_backlink() {
        let mut arena = Arena::new();
        let parent = arena.insert(Fiber::new());
        let child = arena.insert(disposed_fiber());
        assert!(arena.attach_child(parent, child));

        let husk = arena.teardown(child).expect("child is disposed");
        assert_eq!(husk.parent(), Some(parent), "the husk keeps its history");
        assert!(
            arena.get(parent).unwrap().children().is_empty(),
            "the parent's child list only names live fibers"
        );
    }

    #[test]
    fn teardown_with_a_stale_parent_link_is_no_panic() {
        let mut arena = Arena::new();
        let parent = arena.insert(disposed_fiber());
        let child = arena.insert(disposed_fiber());
        assert!(arena.attach_child(parent, child));
        assert!(arena.teardown(parent).is_some());

        // The child's parent link is now stale; teardown must tolerate the
        // missing parent (nothing to clean) instead of panicking.
        assert!(arena.teardown(child).is_some());
        assert!(arena.is_empty());
    }

    #[test]
    fn topology_rejects_stale_ids_and_self_parenting() {
        let mut arena = Arena::new();
        let live = arena.insert(Fiber::new());
        let husk = arena.insert(disposed_fiber());
        arena.teardown(husk).unwrap();

        assert!(!arena.attach_child(live, husk), "stale child");
        assert!(!arena.attach_child(husk, live), "stale parent");
        assert!(!arena.attach_child(live, live), "self-parenting");
        assert!(!arena.detach_child(live, husk), "stale detach");
        assert!(arena.get(live).unwrap().children().is_empty());
        assert_eq!(arena.get(live).unwrap().parent(), None);
    }
}
