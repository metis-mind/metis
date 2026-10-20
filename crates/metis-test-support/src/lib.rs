//! Test-support utilities for the slow track (testing-strategy §2.4).
//!
//! Currently one export: [`CountingAllocator`], the measuring instrument of
//! the D6 memory-return legs (§3.E). Anything too sharp-edged for the
//! production crates but reusable across their test suites lives here.
//!
//! This crate is the workspace's unsafe quarantine: the workspace lints
//! forbid `unsafe_code` absolutely, and a `GlobalAlloc` impl cannot exist
//! without unsafe. The crate therefore opts out of `[lints] workspace =
//! true` (see Cargo.toml). The complete unsafe surface is in this file —
//! the allocator impl and its unit test; consumers only ever write the
//! safe `#[global_allocator]` attribute.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

/// A `GlobalAlloc` wrapper over the system allocator that counts live bytes
/// and live allocation blocks.
///
/// The counters track *requests*, not RSS: the system allocator's internal
/// caching stays invisible below, so a balanced workload returns the counts
/// exactly — an equality assertion has teeth where an RSS slack check would
/// not. `realloc` is deliberately not overridden: the trait's default routes
/// through our `alloc`/`dealloc`, keeping the counts exact.
///
/// The ordering is `Relaxed` throughout: the counters carry no
/// synchronization semantics — the legs read them in a quiescent
/// measurement window.
#[derive(Default)]
pub struct CountingAllocator {
    live_bytes: AtomicUsize,
    live_allocs: AtomicUsize,
}

impl CountingAllocator {
    /// A zeroed counter pair, usable in `static` context.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            live_bytes: AtomicUsize::new(0),
            live_allocs: AtomicUsize::new(0),
        }
    }

    /// Bytes currently allocated through this allocator.
    #[must_use]
    pub fn live_bytes(&self) -> usize {
        self.live_bytes.load(Ordering::Relaxed)
    }

    /// Allocation blocks currently live through this allocator.
    #[must_use]
    pub fn live_allocations(&self) -> usize {
        self.live_allocs.load(Ordering::Relaxed)
    }
}

// The workspace's unsafe surface, site 1 of 2 (the quarantine — see the
// crate docs). SAFETY: every method delegates to `System` with its
// arguments unchanged; the counters are plain atomics and never affect
// allocation behavior.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarded unchanged; the `GlobalAlloc::alloc` contract is
        // upheld by `System`.
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            self.live_bytes.fetch_add(layout.size(), Ordering::Relaxed);
            self.live_allocs.fetch_add(1, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // The caller guarantees `ptr` came from this allocator with this
        // `layout`, so the balance is exact before the forward.
        self.live_bytes.fetch_sub(layout.size(), Ordering::Relaxed);
        self.live_allocs.fetch_sub(1, Ordering::Relaxed);
        // SAFETY: forwarded unchanged; the `GlobalAlloc::dealloc` contract
        // is upheld by `System`.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The allocator is global-state by nature, so its own unit test runs
    /// against a process-local instance: balances measured through a local
    /// allocator installed via `#[global_allocator]` in the consumer's test
    /// binary are covered by the fiber-core memory leg.
    #[test]
    fn counters_balance_allocations_and_frees() {
        let allocator = CountingAllocator::new();
        let layout = Layout::new::<u64>();
        let base_bytes = allocator.live_bytes();
        let base_allocs = allocator.live_allocations();
        // SAFETY: the layout is valid and the block is used only as a raw
        // allocation; freed with the same layout below.
        let ptr = unsafe { allocator.alloc(layout) };
        assert!(!ptr.is_null());
        assert_eq!(allocator.live_bytes(), base_bytes + layout.size());
        assert_eq!(allocator.live_allocations(), base_allocs + 1);
        // SAFETY: `ptr` came from `allocator` with `layout` above.
        unsafe { allocator.dealloc(ptr, layout) };
        assert_eq!(allocator.live_bytes(), base_bytes);
        assert_eq!(allocator.live_allocations(), base_allocs);
    }
}
