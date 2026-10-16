//! Component runtime (ADR-0034): the wasmtime embedding layer — one
//! Store+Instance per plugin, disposable instantiation for hot swap, fuel and
//! epoch budget enforcement, and the registration point for all host imports.
//! Wasm traps are translated into the fiber error-containment path here.
//!
//! The four containment constraints (ADR-0005) are wasmtime-native at this
//! layer, not self-built. Wasmtime enters the dependency graph via the
//! adding-dependencies procedure; the async/reentrancy semantics this crate
//! implements are owned by wit-abi chapter W4, not invented here.
