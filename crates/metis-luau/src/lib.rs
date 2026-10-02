//! The Luau FFI boundary (ADR-0005): mlua wrapper, per-plugin VM provisioning,
//! ctx userdata, `Value` <-> Lua conversion, and the require wrapper that
//! resolves `@lib/` aliases, records dependency edges, and rejects
//! cross-plugin escapes loudly (ADR-0003). All Lua FFI interaction goes
//! through mlua's safe API and is isolated here — this crate is the Miri
//! target for the boundary.
