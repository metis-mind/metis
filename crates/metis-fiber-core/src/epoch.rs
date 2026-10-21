//! Epoch fingerprints and the invalidation closure (fiber.md §5, ADR-0001
//! concretized): when a provider hot-reloads, its service's epoch bumps;
//! consumers whose recorded fingerprint is stale must restart, in dependency
//! order — unload consumers first, reload providers first.
//!
//! The module is pure functions over registry-shaped maps. The registry
//! itself (who bumps epochs, when injects are recorded) is a later station's
//! runtime concern (T10.7); what lives here is the comparison and the
//! closure, which are the parts the L1 property and exhaustive tests pin
//! down.
//!
//! **Registry invariant the caller owns** (fiber.md §5.3): re-registration
//! always increments the service's epoch, so a bumped service's current
//! epoch exceeds every fingerprint recorded against it. The closure still
//! compares with `!=` defensively, but the invariant is what makes the
//! restart rule below sound.
//!
//! **The restart rule** (a monotone fixpoint, hence order-independent):
//!
//! 1. Every provider of a bumped service restarts.
//! 2. Every consumer whose recorded fingerprint differs from the current
//!    epoch restarts — for bumped services this is all current consumers
//!    under the invariant; the same clause also catches fibers that missed
//!    an earlier bump (catch-up needs no new bump).
//! 3. A restarting fiber re-registers all its services when it reloads
//!    (fiber.md §5.3), so every consumer of those services restarts too —
//!    regardless of its recorded fingerprint, which is guaranteed stale the
//!    moment the provider finishes reloading.

use std::collections::BTreeSet;

use crate::id::FiberId;

/// The per-service epoch counter (fiber.md §5.1). Only ever increments:
/// equality, not ordering, is the freshness test.
pub type Epoch = u64;

/// Fingerprint staleness (fiber.md §5.4): the recorded epoch differs from
/// the provider's current one. Epochs only increment, so `!=` and `<`
/// coincide on consistent registries; `!=` also catches the inconsistent
/// ones loudly rather than silently treating them as fresh.
#[must_use]
pub fn is_stale(recorded: Epoch, current: Epoch) -> bool {
    recorded != current
}

/// What the closure computed: which fibers restart, in which order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestartPlan {
    /// Unload order: consumers before their providers (`unload` reversed is
    /// `reload`).
    pub unload: Vec<FiberId>,
    /// Reload order: providers before their consumers.
    pub reload: Vec<FiberId>,
}

/// Indexed views over the registry tables: the closure's working form.
/// Dense fiber ranks (rank order == id order) and service-indexed joins in
/// flat CSR form (count → offsets → fill), so every join below is a linear
/// merge and every lookup a binary search — no hashing on a cold path.
struct IndexedRegistry<'a, K> {
    providers: &'a [(K, (FiberId, Epoch))],
    dependents: &'a [(K, (FiberId, Epoch))],
    ranks: Vec<FiberId>,
    /// CSR: service indices grouped by provider rank —
    /// `provides_flat[provides_off[rank]..provides_off[rank + 1]]`.
    provides_off: Vec<usize>,
    provides_flat: Vec<usize>,
    /// `edge_lo[service]..edge_hi[service]` = the service's edge range.
    edge_lo: Vec<usize>,
    edge_hi: Vec<usize>,
    is_bumped: Vec<bool>,
}

impl<'a, K: Ord> IndexedRegistry<'a, K> {
    fn new(
        providers: &'a [(K, (FiberId, Epoch))],
        bumped: &'a [K],
        dependents: &'a [(K, (FiberId, Epoch))],
    ) -> Self {
        // The sortedness contract (fn docs), enforced loudly in debug.
        debug_assert!(
            providers.is_sorted_by(|a, b| a.0 < b.0),
            "providers must be sorted by unique service key"
        );
        debug_assert!(
            bumped.is_sorted_by(|a, b| a < b),
            "bumped must be sorted and unique"
        );
        debug_assert!(
            dependents.is_sorted_by(|a, b| a.0 <= b.0),
            "dependents must be sorted by service key"
        );

        // Dense fiber ranks: rank order == id order, and rank-indexed
        // tables replace every fiber-keyed map.
        let mut ranks: Vec<FiberId> = Vec::with_capacity(providers.len() + dependents.len());
        ranks.extend(providers.iter().map(|(_, (fiber, _))| *fiber));
        ranks.extend(dependents.iter().map(|(_, (fiber, _))| *fiber));
        ranks.sort_unstable();
        ranks.dedup();

        // Service-indexed joins as linear merges over the sorted slices.
        // Edges or bumps naming an unknown service are ignored (as before).
        let n = providers.len();
        let m = ranks.len();
        let mut provides_count = vec![0usize; m];
        let mut edge_lo = Vec::with_capacity(n);
        let mut edge_hi = Vec::with_capacity(n);
        let mut is_bumped = Vec::with_capacity(n);
        let mut edge_start = 0;
        let mut bump_start = 0;
        for (key, (provider, _)) in providers {
            provides_count[rank_in(&ranks, *provider)] += 1;
            while dependents.get(edge_start).is_some_and(|(k, _)| *k < *key) {
                edge_start += 1;
            }
            let mut edge_end = edge_start;
            while dependents.get(edge_end).is_some_and(|(k, _)| *k == *key) {
                edge_end += 1;
            }
            edge_lo.push(edge_start);
            edge_hi.push(edge_end);
            edge_start = edge_end;
            while bumped.get(bump_start).is_some_and(|k| *k < *key) {
                bump_start += 1;
            }
            is_bumped.push(bumped.get(bump_start).is_some_and(|k| *k == *key));
        }
        let mut provides_off = Vec::with_capacity(m + 1);
        provides_off.push(0);
        for rank in 0..m {
            provides_off.push(provides_off[rank] + provides_count[rank]);
        }
        let mut provides_flat = vec![0usize; n];
        // Fill: the cursor walk starts from a copy of the offsets.
        let mut cursor = vec![0usize; m];
        cursor.copy_from_slice(&provides_off[..m]);
        for (i, (_, (provider, _))) in providers.iter().enumerate() {
            let rank = rank_in(&ranks, *provider);
            provides_flat[cursor[rank]] = i;
            cursor[rank] += 1;
        }
        Self {
            providers,
            dependents,
            ranks,
            provides_off,
            provides_flat,
            edge_lo,
            edge_hi,
            is_bumped,
        }
    }

    fn dependents_of(&self, service: usize) -> &[(K, (FiberId, Epoch))] {
        &self.dependents[self.edge_lo[service]..self.edge_hi[service]]
    }

    /// Indices of the services the fiber with `rank` provides (CSR slice).
    fn provides_of(&self, rank: usize) -> &[usize] {
        &self.provides_flat[self.provides_off[rank]..self.provides_off[rank + 1]]
    }
}

/// Rank of a fiber in the sorted rank vec. Every fiber named by the input
/// is ranked by construction (the ranks are built from the same tables).
fn rank_in(ranks: &[FiberId], fiber: FiberId) -> usize {
    ranks
        .binary_search(&fiber)
        .expect("every fiber named by the input is ranked")
}

/// Provider → consumer adjacency in CSR form plus the per-dependent
/// affected in-degree countdown — `invalidation_closure` step 2, same
/// count → offsets → fill buffer discipline as `IndexedRegistry`.
struct Topo {
    in_degree: Vec<u32>,
    off: Vec<usize>,
    flat: Vec<usize>,
}

fn build_topo<K: Ord>(indexed: &IndexedRegistry<'_, K>, restarting: &[bool]) -> Topo {
    let m = indexed.ranks.len();
    let mut in_degree = vec![0u32; m];
    let mut consumers_count = vec![0usize; m];
    for (i, (_, (provider, _))) in indexed.providers.iter().enumerate() {
        let p = rank_in(&indexed.ranks, *provider);
        if !restarting[p] {
            continue;
        }
        for (_, (consumer, _)) in indexed.dependents_of(i) {
            let c = rank_in(&indexed.ranks, *consumer);
            // A fiber never waits on its own reload (self-edge exclusion);
            // edges to unaffected dependents are not part of this plan.
            if restarting[c] && c != p {
                in_degree[c] += 1;
                consumers_count[p] += 1;
            }
        }
    }
    let mut off = Vec::with_capacity(m + 1);
    off.push(0);
    for rank in 0..m {
        off.push(off[rank] + consumers_count[rank]);
    }
    let mut flat = vec![0usize; off[m]];
    let mut cursor = vec![0usize; m];
    cursor.copy_from_slice(&off[..m]);
    for (i, (_, (provider, _))) in indexed.providers.iter().enumerate() {
        let p = rank_in(&indexed.ranks, *provider);
        if !restarting[p] {
            continue;
        }
        for (_, (consumer, _)) in indexed.dependents_of(i) {
            let c = rank_in(&indexed.ranks, *consumer);
            if restarting[c] && c != p {
                flat[cursor[p]] = c;
                cursor[p] += 1;
            }
        }
    }
    Topo {
        in_degree,
        off,
        flat,
    }
}

/// Compute the invalidation closure over the registry tables.
///
/// - `providers` — the registry's service table, as key → (provider fiber,
///   current epoch) pairs, **sorted by key, keys unique** (registry
///   invariant: one provider per service).
/// - `bumped` — the services that just re-registered, **sorted, unique**;
///   their providers restart. Registry invariant (module docs): each bumped
///   service's current epoch exceeds every fingerprint recorded against it.
/// - `dependents` — the reverse index (fiber.md §5.5) as a flat edge list
///   sorted by service key: key → (consumer fiber, fingerprint recorded at
///   inject).
///
/// The sortedness contract makes the input canonical: the plan is
/// deterministic across processes by construction (journal/replay), every
/// join below is a linear merge, and every lookup a binary search — one
/// sort of the fiber set up front, no hashing anywhere. A `BTreeMap`-based
/// registry hands over its iteration order directly; a HashMap-based
/// caller collects and sorts once per call. This runs per re-registration
/// — a cold path — so constants beat hashing strings at registry scales.
///
/// Cyclic input cannot be fully ordered — the enable-time cycle ban
/// (wit-abi W9.7) keeps production graphs acyclic; should a cycle ever
/// appear anyway, the affected set is still complete and the unorderable
/// remainder is appended in id order rather than looping forever.
#[must_use]
pub fn invalidation_closure<K: Ord>(
    providers: &[(K, (FiberId, Epoch))],
    bumped: &[K],
    dependents: &[(K, (FiberId, Epoch))],
) -> RestartPlan {
    let indexed = IndexedRegistry::new(providers, bumped, dependents);
    let m = indexed.ranks.len();

    // 1. The restart set as a rank bitset, grown by the module-docs rule:
    //    bumped providers, then stale-fingerprint consumers, then
    //    propagation. Monotone = the worklist order cannot lose a member.
    let mut restarting = vec![false; m];
    for (i, (_, (provider, current))) in indexed.providers.iter().enumerate() {
        if indexed.is_bumped[i] {
            restarting[rank_in(&indexed.ranks, *provider)] = true;
        }
        for (_, (consumer, recorded)) in indexed.dependents_of(i) {
            if is_stale(*recorded, *current) {
                restarting[rank_in(&indexed.ranks, *consumer)] = true;
            }
        }
    }
    let mut worklist: Vec<usize> = restarting
        .iter()
        .enumerate()
        .filter(|(_, restarting)| **restarting)
        .map(|(rank, _)| rank)
        .collect();
    while let Some(fiber) = worklist.pop() {
        for service in indexed.provides_of(fiber) {
            for (_, (consumer, _)) in indexed.dependents_of(*service) {
                let rank = rank_in(&indexed.ranks, *consumer);
                if !restarting[rank] {
                    restarting[rank] = true;
                    worklist.push(rank);
                }
            }
        }
    }

    // 2. Topo-sort the affected set with provider → consumer edges: that
    //    order is the reload order (providers first); unload is its reverse.
    //    Kahn's algorithm in countdown form — each dependent counts its
    //    affected in-edges, a provider's pop decrements, zero means ready.
    //    (Countdown, not per-dependent prerequisite sets: termination must
    //    not hinge on one short-circuit — a disturbed empty-set test
    //    re-queues fibers forever.)
    let Topo {
        mut in_degree,
        off: consumers_off,
        flat: consumers_flat,
    } = build_topo(&indexed, &restarting);
    // Ready-queue: smallest rank first (rank order == id order), so the
    // plan is deterministic.
    let mut ready: BTreeSet<usize> = (0..m)
        .filter(|rank| restarting[*rank] && in_degree[*rank] == 0)
        .collect();
    let mut emitted = vec![false; m];
    // The plan accumulates rank indices, mapped to ids at the end.
    let mut plan: Vec<usize> = Vec::new();
    while let Some(fiber) = ready.pop_first() {
        plan.push(fiber);
        emitted[fiber] = true;
        for consumer in &consumers_flat[consumers_off[fiber]..consumers_off[fiber + 1]] {
            let count = &mut in_degree[*consumer];
            *count -= 1;
            if *count == 0 {
                ready.insert(*consumer);
            }
        }
    }
    // Cyclic remainder (unreachable in production; module docs): complete
    // the set deterministically in rank order rather than dropping fibers.
    for fiber in 0..m {
        if restarting[fiber] && !emitted[fiber] {
            plan.push(fiber);
        }
    }

    let reload: Vec<FiberId> = plan.iter().map(|rank| indexed.ranks[*rank]).collect();
    let unload: Vec<FiberId> = reload.iter().rev().copied().collect();
    RestartPlan { unload, reload }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;
    use std::collections::HashSet;

    /// Mint a batch of fiber ids.
    fn ids(n: usize) -> Vec<FiberId> {
        let mut arena: SlotMap<FiberId, ()> = SlotMap::with_key();
        (0..n).map(|_| arena.insert(())).collect()
    }

    /// Registry tables for a tiny two-service chain:
    /// root provides "a"; mid consumes "a", provides "b"; leaf consumes "b".
    #[allow(clippy::type_complexity)]
    fn chain() -> (
        Vec<FiberId>,
        Vec<(&'static str, (FiberId, Epoch))>,
        Vec<(&'static str, (FiberId, Epoch))>,
    ) {
        let ids = ids(3);
        let (root, mid, leaf) = (ids[0], ids[1], ids[2]);
        let providers = vec![("a", (root, 1)), ("b", (mid, 1))];
        let dependents = vec![("a", (mid, 1)), ("b", (leaf, 1))];
        (vec![root, mid, leaf], providers, dependents)
    }

    #[test]
    fn bumped_provider_restarts_its_stale_chain_in_order() {
        let (chain, _providers, dependents) = chain();
        let (root, mid, leaf) = (chain[0], chain[1], chain[2]);
        // "a" re-registers: epoch 1 -> 2. mid's fingerprint (1) is stale.
        let providers = vec![("a", (root, 2)), ("b", (mid, 1))];
        let bumped = vec!["a"];

        let plan = invalidation_closure(&providers, &bumped, &dependents);
        // mid restarts (stale); mid's restart bumps "b", so leaf restarts
        // too (propagation marks all of "b"'s dependents).
        assert_eq!(plan.unload, vec![leaf, mid, root]);
        assert_eq!(plan.reload, vec![root, mid, leaf]);
    }

    #[test]
    fn secondary_restart_marks_dependents_fresh_against_current_epochs() {
        // The review-caught case: a fiber fresh against its service's
        // CURRENT epoch must still restart when its provider restarts for
        // another reason — the provider's reload will bump the epoch past
        // any recorded fingerprint (module docs, restart rule 3).
        let ids = ids(3);
        let (f1, f2, f3) = (ids[0], ids[1], ids[2]);
        let providers = vec![("s0", (f2, 1)), ("s1", (f1, 3))];
        let dependents = vec![("s0", (f3, 1)), ("s1", (f2, 1))];
        let bumped = vec!["s0", "s1"];

        let plan = invalidation_closure(&providers, &bumped, &dependents);
        // f2 restarts (stale vs s1's current epoch); f2's restart propagates
        // to s0's consumers regardless of their fingerprints -> f3 restarts.
        assert_eq!(plan.reload, vec![f1, f2, f3]);
        assert_eq!(plan.unload, vec![f3, f2, f1]);
    }

    #[test]
    fn unaffected_branches_are_left_alone() {
        let ids = ids(4);
        let (a, b, c, d) = (ids[0], ids[1], ids[2], ids[3]);
        let providers = vec![("x", (a, 2)), ("y", (b, 1)), ("z", (c, 1))];
        let dependents = vec![("x", (b, 1)), ("y", (c, 1)), ("y", (d, 1))];
        // "x" bumped: b is stale (recorded 1 < 2). b's restart propagates to
        // "y" -> c and d become stale. "z" is untouched.
        let bumped = vec!["x"];
        let plan = invalidation_closure(&providers, &bumped, &dependents);
        let reload: HashSet<FiberId> = plan.reload.iter().copied().collect();
        assert_eq!(reload, HashSet::from([a, b, c, d]));
        // Ordering: b before a (consumer first on unload), c and d before b.
        let pos = |f: &FiberId| plan.unload.iter().position(|x| x == f).unwrap();
        assert!(pos(&b) < pos(&a));
        assert!(pos(&c) < pos(&b));
        assert!(pos(&d) < pos(&b));
        let mut reversed = plan.reload.clone();
        reversed.reverse();
        assert_eq!(plan.unload, reversed);
    }

    #[test]
    fn reload_order_is_topological_not_id_order() {
        // The consumer was created BEFORE its provider (lower id): an
        // id-sorted impostor reload would put the consumer first; the plan
        // must be topological regardless of id assignment.
        let ids = ids(2);
        let (consumer, provider) = (ids[0], ids[1]);
        let providers = vec![("s", (provider, 2))];
        let dependents = vec![("s", (consumer, 1))];
        let bumped = vec!["s"];
        let plan = invalidation_closure(&providers, &bumped, &dependents);
        assert_eq!(plan.reload, vec![provider, consumer]);
        assert_eq!(plan.unload, vec![consumer, provider]);
    }

    #[test]
    fn a_self_edge_does_not_block_the_fibers_own_reload() {
        // A fiber that consumes the service it provides never waits on
        // itself (the `dependent != provider` exclusion); its dependent
        // still waits on it. The dependent carries the LOWER id, so an
        // id-order fallback would show.
        let ids = ids(2);
        let (dependent, provider) = (ids[0], ids[1]);
        let providers = vec![("down", (provider, 2)), ("own", (provider, 2))];
        let dependents = vec![("down", (dependent, 1)), ("own", (provider, 1))];
        let plan = invalidation_closure(&providers, &Vec::new(), &dependents);
        assert_eq!(plan.reload, vec![provider, dependent]);
        assert_eq!(plan.unload, vec![dependent, provider]);
    }

    #[test]
    #[should_panic(expected = "providers must be sorted")]
    fn unsorted_providers_are_rejected_loudly() {
        let ids = ids(2);
        let providers = vec![("b", (ids[0], 1)), ("a", (ids[1], 1))];
        let _ = invalidation_closure(&providers, &Vec::new(), &Vec::new());
    }

    #[test]
    #[should_panic(expected = "bumped must be sorted and unique")]
    fn unsorted_bumped_is_rejected_loudly() {
        let ids = ids(1);
        let providers = vec![("a", (ids[0], 1)), ("b", (ids[0], 1))];
        let _ = invalidation_closure(&providers, &["b", "a"], &Vec::new());
    }

    #[test]
    #[should_panic(expected = "dependents must be sorted")]
    fn unsorted_dependents_are_rejected_loudly() {
        let ids = ids(2);
        let providers = vec![("a", (ids[0], 1)), ("b", (ids[1], 1))];
        let dependents = vec![("b", (ids[1], 1)), ("a", (ids[1], 1))];
        let _ = invalidation_closure(&providers, &Vec::new(), &dependents);
    }

    #[test]
    fn edges_and_bumps_naming_unknown_services_are_ignored() {
        let ids = ids(1);
        let providers = vec![("b", (ids[0], 1))];
        // "a" sorts before every provided key: the merge walk must skip it.
        let dependents = vec![("a", (ids[0], 9))];
        let bumped = vec!["a"];
        let plan = invalidation_closure(&providers, &bumped, &dependents);
        assert!(plan.reload.is_empty(), "unknown services restart no one");
        assert!(plan.unload.is_empty());
    }

    #[test]
    fn cycles_terminate_with_the_full_set() {
        let ids = ids(2);
        let (a, b) = (ids[0], ids[1]);
        let providers = vec![("s1", (a, 2)), ("s2", (b, 2))];
        let dependents = vec![("s1", (b, 1)), ("s2", (a, 1))];
        let bumped = vec!["s1"];
        let plan = invalidation_closure(&providers, &bumped, &dependents);
        let set: HashSet<FiberId> = plan.reload.iter().copied().collect();
        assert_eq!(
            set,
            HashSet::from([a, b]),
            "cycle still yields the full set"
        );
        assert_eq!(plan.reload.len(), 2, "no duplicates");
    }

    #[test]
    fn is_stale_compares_unequal() {
        assert!(is_stale(1, 2));
        assert!(!is_stale(2, 2));
        assert!(
            is_stale(2, 1),
            "inconsistent registries read as stale, loudly"
        );
    }

    mod properties {
        use super::*;
        use proptest::prelude::*;
        use slotmap::SlotMap;

        /// A random registry case. Consumers always outrank their provider
        /// in fiber index (the topological rank), so the dependency graph is
        /// acyclic by construction (cycles are an L0 unit-test concern).
        /// Fiber ids are shuffled against the rank: slotmap hands ids out in
        /// insertion order, so without a shuffle id order ≡ topo order and
        /// an id-sorted impostor would pass the ordering assertions.
        #[derive(Debug, Clone)]
        struct Case {
            providers: Vec<(u32, (FiberId, Epoch))>,
            dependents: Vec<(u32, (FiberId, Epoch))>,
            bumped: Vec<u32>,
        }

        fn case() -> impl Strategy<Value = Case> {
            (
                1usize..=8,
                prop::collection::vec((0u32..8, 1u64..=5), 0..=6),
                prop::collection::vec(any::<bool>(), 1..=8),
            )
                .prop_flat_map(|(n_fibers, service_specs, bump_flags)| {
                    let services: Vec<(u32, Epoch)> = {
                        let mut seen = HashSet::new();
                        service_specs
                            .into_iter()
                            .filter(|(key, _)| seen.insert(*key))
                            .collect()
                    };
                    let bumped: Vec<u32> = services
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| {
                            bump_flags
                                .get(i % bump_flags.len())
                                .copied()
                                .unwrap_or(false)
                        })
                        .map(|(_, (key, _))| *key)
                        .collect();
                    (
                        Just(n_fibers),
                        Just(services),
                        Just(bumped),
                        prop::collection::vec((0u32..8, 0..n_fibers, 1u64..=5), 0..=16),
                        prop::collection::vec(any::<u8>(), n_fibers),
                    )
                })
                .prop_map(|(n_fibers, services, bumped, dep_specs, shuffle)| {
                    let mut arena: SlotMap<FiberId, ()> = SlotMap::with_key();
                    let mut fibers: Vec<FiberId> =
                        (0..n_fibers).map(|_| arena.insert(())).collect();
                    // Deterministic Fisher–Yates from the drawn seed: the
                    // vec index stays the topological rank (consumers
                    // outrank providers below), the id order becomes
                    // arbitrary (see the Case docs).
                    for (i, &seed) in shuffle.iter().enumerate() {
                        let j = usize::from(seed) % (i + 1);
                        fibers.swap(i, j);
                    }
                    let mut providers: Vec<(u32, (FiberId, Epoch))> = services
                        .iter()
                        .enumerate()
                        .map(|(i, (key, epoch))| (*key, (fibers[i % n_fibers], *epoch)))
                        .collect();
                    providers.sort_unstable_by_key(|(key, _)| *key);
                    let mut seen_pairs = HashSet::new();
                    let mut dependents: Vec<(u32, (FiberId, Epoch))> = Vec::new();
                    for (key, consumer, recorded) in dep_specs {
                        let Some((_, (provider, current))) =
                            providers.iter().find(|(k, _)| *k == key)
                        else {
                            continue;
                        };
                        let provider_idx = fibers.iter().position(|f| f == provider).unwrap();
                        // DAG by construction: consumers outrank providers.
                        if consumer <= provider_idx || !seen_pairs.insert((key, consumer)) {
                            continue;
                        }
                        dependents.push((key, (fibers[consumer], recorded.min(*current))));
                    }
                    dependents.sort_unstable_by_key(|(key, _)| *key);
                    let mut bumped = bumped;
                    bumped.sort_unstable();
                    Case {
                        providers,
                        dependents,
                        bumped,
                    }
                })
        }

        /// Independent oracle: a naive fixpoint over the restart rule
        /// (structurally different from the seed+propagate implementation,
        /// so an agreement is evidence, not a tautology).
        fn reference_affected(case: &Case) -> HashSet<FiberId> {
            let mut affected: HashSet<FiberId> = case
                .bumped
                .iter()
                .filter_map(|service| {
                    case.providers
                        .iter()
                        .find(|(k, _)| k == service)
                        .map(|(_, v)| v)
                })
                .map(|(provider, _)| *provider)
                .collect();
            loop {
                let mut changed = false;
                for (service, (provider, epoch)) in &case.providers {
                    let provider_restarts = affected.contains(provider);
                    for (k, (dependent, recorded)) in &case.dependents {
                        if k == service
                            && (is_stale(*recorded, *epoch) || provider_restarts)
                            && affected.insert(*dependent)
                        {
                            changed = true;
                        }
                    }
                }
                if !changed {
                    break;
                }
            }
            affected
        }

        proptest! {
            #[test]
            fn closure_matches_the_oracle_and_orders_correctly(case in case()) {
                // No determinism arm: the sorted-input contract makes the
                // input canonical, so order-independence holds by
                // construction (the contract itself is pinned by an L0
                // should_panic test).
                let plan = invalidation_closure(&case.providers, &case.bumped, &case.dependents);

                // Shape: no duplicates; unload is reload reversed.
                let reload_set: HashSet<FiberId> = plan.reload.iter().copied().collect();
                prop_assert_eq!(reload_set.len(), plan.reload.len(), "no duplicates in reload");
                let mut reversed = plan.reload.clone();
                reversed.reverse();
                prop_assert_eq!(&plan.unload, &reversed);

                // Completeness and minimality: the affected set is exactly the oracle's.
                let expected = reference_affected(&case);
                prop_assert_eq!(&reload_set, &expected);

                // Ordering: a provider reloads before its consumers (and
                // therefore unloads after them).
                let position = |fiber: &FiberId| plan.reload.iter().position(|f| f == fiber);
                for (service, (provider, _)) in &case.providers {
                    for (k, (dependent, _)) in &case.dependents {
                        if k == service
                            && let (Some(p), Some(d)) = (position(provider), position(dependent))
                        {
                            prop_assert!(p < d, "provider must reload before consumer");
                        }
                    }
                }

                // Every bumped provider restarts.
                for service in &case.bumped {
                    if let Some((_, (provider, _))) =
                        case.providers.iter().find(|(k, _)| k == service)
                    {
                        prop_assert!(reload_set.contains(provider));
                    }
                }
            }
        }

        /// The bounded space the retired Kani harness quantified over
        /// (issue #5; testing-strategy §2.1 2026-10-21 修订注), enumerated
        /// exhaustively: 3 fibers × 3 services × every edge and bump
        /// combination — 26³ × 8 = 140,608 cases. Two argued domain
        /// reductions (per the ruler; the arguments live here):
        ///
        /// - Epochs drop from {1, 2, 3} to {1, 2}: epochs are only ever
        ///   compared with `!=` (`is_stale`), so two distinct values
        ///   represent every freshness relation.
        /// - A service absent from the retired harness's registry coincides
        ///   with a present-but-inert row here (no bump, both edge slots
        ///   dummies — a dummy is provider → itself at the current epoch:
        ///   never stale, excluded from the topo, and propagation through
        ///   it re-adds only the provider itself): such a row restarts no
        ///   one and adds no restart path. The adjacent unknown-service
        ///   ignore path is pinned by an L0 test.
        ///
        /// Discrimination note: the ids here are minted in rank order, so
        /// the ordering arm is vacuous in this space — ordering teeth live
        /// in the shuffled proptest above and the L0 ordering tests; this
        /// space pins the fixpoint set. Oracle = the same independent
        /// naive fixpoint the proptest samples against.
        #[test]
        fn closure_matches_the_oracle_on_every_bounded_case() {
            /// One edge slot: `Some(recorded)` = a real edge to the
            /// consumer, `None` = the inert dummy (provider → itself at
            /// the current epoch — never stale, excluded from the topo).
            type Slot = Option<Epoch>;
            /// (provider index, provider epoch, [slot0, slot1]).
            type KeyOptions = (usize, Epoch, [Slot; 2]);

            fn slot_options(provider: usize, slot: usize) -> Vec<Slot> {
                if provider + 1 + slot < 3 {
                    vec![None, Some(1), Some(2)]
                } else {
                    vec![None]
                }
            }

            let mut options: Vec<KeyOptions> = Vec::new();
            for provider in 0..3usize {
                for epoch in 1..=2 {
                    for s0 in slot_options(provider, 0) {
                        for s1 in slot_options(provider, 1) {
                            options.push((provider, epoch, [s0, s1]));
                        }
                    }
                }
            }
            assert_eq!(options.len(), 26, "per-key option count changed");

            let f = ids(3);
            let mut cases = 0u32;
            for k0 in &options {
                for k1 in &options {
                    for k2 in &options {
                        for bump_mask in 0..8u8 {
                            let mut providers = Vec::new();
                            let mut dependents = Vec::new();
                            let mut bumped = Vec::new();
                            for (key, &(provider, epoch, slots)) in
                                [k0, k1, k2].into_iter().enumerate()
                            {
                                let key = u32::try_from(key).expect("three keys");
                                providers.push((key, (f[provider], epoch)));
                                for (j, slot) in slots.into_iter().enumerate() {
                                    let edge = match slot {
                                        Some(recorded) => (f[provider + 1 + j], recorded),
                                        None => (f[provider], epoch),
                                    };
                                    dependents.push((key, edge));
                                }
                                if bump_mask & (1 << key) != 0 {
                                    bumped.push(key);
                                }
                            }
                            let case = Case {
                                providers,
                                dependents,
                                bumped,
                            };

                            let plan = invalidation_closure(
                                &case.providers,
                                &case.bumped,
                                &case.dependents,
                            );

                            let reload_set: HashSet<FiberId> =
                                plan.reload.iter().copied().collect();
                            assert_eq!(
                                reload_set.len(),
                                plan.reload.len(),
                                "duplicates in reload; case: {case:?}"
                            );
                            let mut reversed = plan.reload.clone();
                            reversed.reverse();
                            assert_eq!(plan.unload, reversed, "case: {case:?}");
                            let expected = reference_affected(&case);
                            assert_eq!(
                                reload_set, expected,
                                "closure != naive fixpoint; case: {case:?}"
                            );
                            let position =
                                |fiber: &FiberId| plan.reload.iter().position(|x| x == fiber);
                            for (service, (provider, _)) in &case.providers {
                                for (k, (consumer, _)) in &case.dependents {
                                    if k == service
                                        && consumer != provider
                                        && let (Some(p), Some(d)) =
                                            (position(provider), position(consumer))
                                    {
                                        assert!(
                                            p < d,
                                            "provider reloads after consumer; case: {case:?}"
                                        );
                                    }
                                }
                            }
                            cases += 1;
                        }
                    }
                }
            }
            assert_eq!(cases, 140_608, "bounded space size changed");
        }
    }
}
