//! Epoch fingerprints and the invalidation closure (fiber.md §5, ADR-0001
//! concretized): when a provider hot-reloads, its service's epoch bumps;
//! consumers whose recorded fingerprint is stale must restart, in dependency
//! order — unload consumers first, reload providers first.
//!
//! The module is pure functions over registry-shaped maps. The registry
//! itself (who bumps epochs, when injects are recorded) is a later station's
//! runtime concern (T10.7); what lives here is the comparison and the
//! closure, which are the parts the L1 property tests and the L4 targets
//! pin down.
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

use std::collections::{BTreeSet, HashMap, HashSet};

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

/// Compute the invalidation closure over the registry tables.
///
/// - `providers` — the registry's service table: service key → (provider
///   fiber, current epoch).
/// - `bumped` — the services that just re-registered; their providers
///   restart. Registry invariant (module docs): each bumped service's
///   current epoch exceeds every fingerprint recorded against it.
/// - `dependents_of` — the reverse index (fiber.md §5.5): service key →
///   (consumer fiber, fingerprint recorded at inject).
///
/// The result is deterministic (ids are visited in sorted order), so
/// journal/replay comparisons hold. Cyclic input cannot be fully ordered —
/// the enable-time cycle ban (wit-abi W9.7) keeps production graphs acyclic;
/// should a cycle ever appear anyway, the affected set is still complete
/// and the unorderable remainder is appended in id order rather than
/// looping forever.
#[must_use]
pub fn invalidation_closure<K: Eq + std::hash::Hash, S: std::hash::BuildHasher>(
    providers: &HashMap<K, (FiberId, Epoch), S>,
    bumped: &HashSet<K, S>,
    dependents_of: &HashMap<K, Vec<(FiberId, Epoch)>, S>,
) -> RestartPlan {
    // Fiber -> services it provides (inverted provider table), for restart
    // propagation.
    let mut provides_of: HashMap<FiberId, Vec<&K>> = HashMap::new();
    for (service, (provider, _)) in providers {
        provides_of.entry(*provider).or_default().push(service);
    }

    // 1. The restart set, as a monotone fixpoint over the rule in the
    //    module docs. Monotone = order-independent: no dedup order can lose
    //    a member (the pre-fix worklist version could; the L1 suite caught
    //    it).
    let mut restarting: HashSet<FiberId> = HashSet::new();
    for service in bumped {
        if let Some((provider, _)) = providers.get(service) {
            restarting.insert(*provider);
        }
    }
    for (service, (_, current)) in providers {
        for (dependent, recorded) in dependents_of.get(service).into_iter().flatten() {
            if is_stale(*recorded, *current) {
                restarting.insert(*dependent);
            }
        }
    }
    let mut worklist: Vec<FiberId> = restarting.iter().copied().collect();
    while let Some(fiber) = worklist.pop() {
        for service in provides_of.get(&fiber).into_iter().flatten() {
            for (dependent, _) in dependents_of.get(*service).into_iter().flatten() {
                if restarting.insert(*dependent) {
                    worklist.push(*dependent);
                }
            }
        }
    }

    // 2. Affected set = the restart set (bumped providers were seeded in).
    let affected: BTreeSet<FiberId> = restarting.into_iter().collect();

    // 3. Topo-sort the affected set with provider → consumer edges: that
    //    order is the reload order (providers first); unload is its reverse.
    //    Kahn's algorithm in countdown form — each dependent counts its
    //    affected in-edges, a provider's pop decrements, zero means ready.
    //    The BTreeSet ready-queue keeps the result deterministic.
    //    (Countdown, not per-dependent prerequisite sets: termination must
    //    not hinge on one short-circuit — a disturbed empty-set test
    //    re-queues fibers forever.)
    let mut remaining: HashMap<FiberId, usize> = HashMap::new();
    let mut consumers_of: HashMap<FiberId, Vec<FiberId>> = HashMap::new();
    for (service, (provider, _)) in providers {
        if !affected.contains(provider) {
            continue;
        }
        for (dependent, _) in dependents_of.get(service).into_iter().flatten() {
            // A fiber never waits on its own reload (self-edge exclusion);
            // edges to unaffected dependents are not part of this plan.
            if affected.contains(dependent) && dependent != provider {
                *remaining.entry(*dependent).or_insert(0) += 1;
                consumers_of.entry(*provider).or_default().push(*dependent);
            }
        }
    }
    let mut ready: BTreeSet<FiberId> = affected
        .iter()
        .copied()
        .filter(|fiber| !remaining.contains_key(fiber))
        .collect();
    let mut reload: Vec<FiberId> = Vec::with_capacity(affected.len());
    while let Some(fiber) = ready.pop_first() {
        reload.push(fiber);
        for dependent in consumers_of.get(&fiber).into_iter().flatten() {
            let count = remaining
                .get_mut(dependent)
                .expect("every recorded edge also counted its dependent");
            *count -= 1;
            if *count == 0 {
                ready.insert(*dependent);
            }
        }
    }
    // Cyclic remainder (unreachable in production; module docs): complete the
    // set deterministically rather than dropping fibers.
    for fiber in &affected {
        if !reload.contains(fiber) {
            reload.push(*fiber);
        }
    }

    let mut unload = reload.clone();
    unload.reverse();
    RestartPlan { unload, reload }
}

#[cfg(kani)]
mod proofs {
    use super::*;
    use slotmap::SlotMap;

    /// Bounded DAGs (≤3 fibers, ≤3 services, epochs 1..=3): the closure
    /// computes exactly the naive fixpoint, without duplicates, and every
    /// provider reloads before its consumers. Universally quantified where
    /// the L1 property tests sample.
    #[kani::proof]
    #[kani::unwind(16)]
    fn closure_computes_the_fixpoint_and_respects_edges() {
        let mut arena: SlotMap<FiberId, ()> = SlotMap::with_key();
        let f: Vec<FiberId> = (0..3).map(|_| arena.insert(())).collect();

        // Symbolic registry; DAG by construction (consumers outrank their
        // provider's index).
        let mut providers: HashMap<u8, (FiberId, Epoch)> = HashMap::new();
        let mut dependents_of: HashMap<u8, Vec<(FiberId, Epoch)>> = HashMap::new();
        let mut bumped: HashSet<u8> = HashSet::new();
        for key in 0u8..3 {
            if kani::any() {
                let provider_idx: usize = kani::any();
                kani::assume(provider_idx < 3);
                let epoch: Epoch = kani::any();
                kani::assume((1..=3).contains(&epoch));
                providers.insert(key, (f[provider_idx], epoch));
                for consumer_idx in (provider_idx + 1)..3 {
                    if kani::any() {
                        let recorded: Epoch = kani::any();
                        kani::assume((1..=3).contains(&recorded));
                        dependents_of
                            .entry(key)
                            .or_default()
                            .push((f[consumer_idx], recorded));
                    }
                }
                if kani::any() {
                    bumped.insert(key);
                }
            }
        }

        let plan = invalidation_closure(&providers, &bumped, &dependents_of);

        // Oracle: a naive fixpoint; 4 passes always suffice for 3 fibers
        // (each pass settles at least one more fiber, or is stable).
        let mut expected: HashSet<FiberId> = bumped
            .iter()
            .filter_map(|service| providers.get(service))
            .map(|(provider, _)| *provider)
            .collect();
        for _ in 0..4 {
            for (service, (provider, epoch)) in &providers {
                for (dependent, recorded) in dependents_of.get(service).into_iter().flatten() {
                    if is_stale(*recorded, *epoch) || expected.contains(provider) {
                        expected.insert(*dependent);
                    }
                }
            }
        }

        let reload_set: HashSet<FiberId> = plan.reload.iter().copied().collect();
        assert_eq!(reload_set, expected, "the closure is exactly the fixpoint");
        assert_eq!(plan.reload.len(), reload_set.len(), "no duplicates");
        for (service, (provider, _)) in &providers {
            for (dependent, _) in dependents_of.get(service).into_iter().flatten() {
                if let (Some(p), Some(d)) = (
                    plan.reload.iter().position(|x| x == provider),
                    plan.reload.iter().position(|x| x == dependent),
                ) {
                    assert!(p < d, "provider reloads before consumer");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;

    /// Mint a batch of fiber ids.
    fn ids(n: usize) -> Vec<FiberId> {
        let mut arena: SlotMap<FiberId, ()> = SlotMap::with_key();
        (0..n).map(|_| arena.insert(())).collect()
    }

    type Providers = HashMap<&'static str, (FiberId, Epoch)>;
    type Dependents = HashMap<&'static str, Vec<(FiberId, Epoch)>>;

    /// Registry tables for a tiny two-service chain:
    /// root provides "a"; mid consumes "a", provides "b"; leaf consumes "b".
    fn chain() -> (Vec<FiberId>, Providers, Dependents) {
        let ids = ids(3);
        let (root, mid, leaf) = (ids[0], ids[1], ids[2]);
        let providers = HashMap::from([("a", (root, 1)), ("b", (mid, 1))]);
        let dependents = HashMap::from([("a", vec![(mid, 1)]), ("b", vec![(leaf, 1)])]);
        (vec![root, mid, leaf], providers, dependents)
    }

    #[test]
    fn bumped_provider_restarts_its_stale_chain_in_order() {
        let (chain, _providers, dependents) = chain();
        let (root, mid, leaf) = (chain[0], chain[1], chain[2]);
        // "a" re-registers: epoch 1 -> 2. mid's fingerprint (1) is stale.
        let providers = HashMap::from([("a", (root, 2)), ("b", (mid, 1))]);
        let bumped = HashSet::from(["a"]);

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
        let providers = HashMap::from([("s0", (f2, 1)), ("s1", (f1, 3))]);
        let dependents = HashMap::from([("s0", vec![(f3, 1)]), ("s1", vec![(f2, 1)])]);
        let bumped = HashSet::from(["s0", "s1"]);

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
        let providers = HashMap::from([("x", (a, 2)), ("y", (b, 1)), ("z", (c, 1))]);
        let dependents = HashMap::from([
            ("x", vec![(b, 1)]),
            ("y", vec![(c, 1), (d, 1)]),
            ("z", vec![]),
        ]);
        // "x" bumped: b is stale (recorded 1 < 2). b's restart propagates to
        // "y" -> c and d become stale. "z" is untouched.
        let bumped = HashSet::from(["x"]);
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
        let providers = HashMap::from([("s", (provider, 2))]);
        let dependents = HashMap::from([("s", vec![(consumer, 1)])]);
        let bumped = HashSet::from(["s"]);
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
        let providers = HashMap::from([("own", (provider, 2)), ("down", (provider, 2))]);
        let dependents =
            HashMap::from([("own", vec![(provider, 1)]), ("down", vec![(dependent, 1)])]);
        let plan = invalidation_closure(&providers, &HashSet::new(), &dependents);
        assert_eq!(plan.reload, vec![provider, dependent]);
        assert_eq!(plan.unload, vec![dependent, provider]);
    }

    #[test]
    fn cycles_terminate_with_the_full_set() {
        let ids = ids(2);
        let (a, b) = (ids[0], ids[1]);
        let providers = HashMap::from([("s1", (a, 2)), ("s2", (b, 2))]);
        let dependents = HashMap::from([("s1", vec![(b, 1)]), ("s2", vec![(a, 1)])]);
        let bumped = HashSet::from(["s1"]);
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
            providers: HashMap<u32, (FiberId, Epoch)>,
            dependents_of: HashMap<u32, Vec<(FiberId, Epoch)>>,
            bumped: HashSet<u32>,
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
                    let bumped: HashSet<u32> = services
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
                    let providers: HashMap<u32, (FiberId, Epoch)> = services
                        .iter()
                        .enumerate()
                        .map(|(i, (key, epoch))| (*key, (fibers[i % n_fibers], *epoch)))
                        .collect();
                    let mut seen_pairs = HashSet::new();
                    let mut dependents_of: HashMap<u32, Vec<(FiberId, Epoch)>> = HashMap::new();
                    for (key, consumer, recorded) in dep_specs {
                        let Some((provider, current)) = providers.get(&key) else {
                            continue;
                        };
                        let provider_idx = fibers.iter().position(|f| f == provider).unwrap();
                        // DAG by construction: consumers outrank providers.
                        if consumer <= provider_idx || !seen_pairs.insert((key, consumer)) {
                            continue;
                        }
                        dependents_of
                            .entry(key)
                            .or_default()
                            .push((fibers[consumer], recorded.min(*current)));
                    }
                    Case {
                        providers,
                        dependents_of,
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
                .filter_map(|service| case.providers.get(service))
                .map(|(provider, _)| *provider)
                .collect();
            loop {
                let mut changed = false;
                for (service, (provider, epoch)) in &case.providers {
                    let provider_restarts = affected.contains(provider);
                    for (dependent, recorded) in
                        case.dependents_of.get(service).into_iter().flatten()
                    {
                        if (is_stale(*recorded, *epoch) || provider_restarts)
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
                let plan = invalidation_closure(&case.providers, &case.bumped, &case.dependents_of);

                // Determinism: the same logical registry, rebuilt into
                // fresh maps (fresh hasher state, hence a different internal
                // iteration order), must yield the same plan — the fixpoint
                // is order-independent by construction.
                let rebuilt_providers: HashMap<u32, (FiberId, Epoch)> =
                    case.providers.iter().map(|(k, v)| (*k, *v)).collect();
                let rebuilt_bumped: HashSet<u32> = case.bumped.iter().copied().collect();
                let rebuilt_dependents: HashMap<u32, Vec<(FiberId, Epoch)>> = case
                    .dependents_of
                    .iter()
                    .map(|(k, v)| (*k, v.clone()))
                    .collect();
                let again = invalidation_closure(
                    &rebuilt_providers,
                    &rebuilt_bumped,
                    &rebuilt_dependents,
                );
                prop_assert_eq!(&plan, &again);

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
                    for (dependent, _) in case.dependents_of.get(service).into_iter().flatten() {
                        if let (Some(p), Some(d)) = (position(provider), position(dependent)) {
                            prop_assert!(p < d, "provider must reload before consumer");
                        }
                    }
                }

                // Every bumped provider restarts.
                for service in &case.bumped {
                    if let Some((provider, _)) = case.providers.get(service) {
                        prop_assert!(reload_set.contains(provider));
                    }
                }
            }
        }
    }
}
