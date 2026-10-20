//! The generational fiber id (fiber.md §4): the arena owns every fiber; the
//! outside world only ever holds ids.
//!
//! Slot reuse after hot reload makes bare indices unsafe — a retired fiber's
//! id floats around in dependency edges, subscription tables and parent/child
//! links. The generation counter invalidates stale ids automatically (ABA
//! protection): a reused slot answers lookups only for its new generation.

slotmap::new_key_type! {
    /// Handle to a fiber in the arena. Copy-cheap; meaningless off the arena
    /// that issued it.
    pub struct FiberId;
}
