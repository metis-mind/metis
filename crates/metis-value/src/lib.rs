//! The `Value` data model shared by event payloads, service calls, and
//! config (ADR-0015): a pure-data tree of
//! `Null / Bool / Float(f64) / Int(i64) / String / Bytes / Array / Map`.
//!
//! - Values are pure data: functions, handles, and other live references
//!   never enter the tree, so crossing a plugin boundary is always a deep
//!   copy with no shared mutable state — the actor-isolation foundation.
//! - `Int(i64)` carries exact 64-bit integers (ADR-0022); `Bytes` carries
//!   in-model binary (ADR-0042). Both are first-class variants, never
//!   encodings layered onto `String` (which stays strictly UTF-8).
//! - `Map` is a `BTreeMap`: iteration order is key-sorted, so canonical
//!   serialization and replay comparison are deterministic for free
//!   (luau-abi A6.3, semantics carried by wit-abi W6.10).
//! - `Int` and `Float` are strict, separate variants: nothing here coerces
//!   between them, and boundary codecs compare tags exactly (wit-abi W6
//!   D157). IEEE semantics apply inside `Float` — `NaN != NaN` under the
//!   derived `PartialEq`.
//! - Timestamps inside values are integer milliseconds or fractional
//!   seconds, never nanoseconds (ADR-0015's timestamp discipline).
//!
//! `Bytes` is the data-model binary lane (event / service / business
//! payloads) and stays subject to the wire budgets; large binaries belong
//! to the blob lane (wit-abi W6.5), not to `Bytes`.
//!
//! Out of scope by design: the TLV wire encoding (metis-codec), journal's
//! JSONL rendering (metis-runtime), and schema validation (metis-loader).
//! The tree itself is unbounded in memory; entry-count / depth / total-byte
//! budgets are wire-boundary concerns owned by the codec.
//! Zero-dependency pure data (ADR-0019).
//!
//! # Testing provenance (D11)
//!
//! Composition: agent-written unit tests (in-file `mod tests`); maintainer
//! review happens at station acceptance per the operating protocol.
//! Mutation audit (testing-strategy §3.C, on-demand run): cargo-mutants
//! 27.1.0, 2026-10-20 — 44 mutants, 31 caught, 13 unviable, 0 missed.
//! Repro: `cargo mutants -p metis-value --test-tool nextest`.

use std::collections::BTreeMap;
use std::sync::Arc;

/// One node of the `Value` tree.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// Explicit absence of a value.
    Null,
    /// Boolean.
    Bool(bool),
    /// IEEE-754 double. Distinct from `Int` even for integral values.
    Float(f64),
    /// Exact 64-bit signed integer (ADR-0022).
    Int(i64),
    /// UTF-8 text.
    String(String),
    /// In-model binary. `Arc`-backed from day one so host-side sharing
    /// never copies (ADR-0042); cloning a `Value` shares the buffer.
    Bytes(Arc<[u8]>),
    /// Ordered list of values.
    Array(Vec<Value>),
    /// String-keyed map; iteration order is key-sorted (deterministic).
    Map(BTreeMap<String, Value>),
}

// `Value` must stay `Send + Sync`: the executor builds and dispatches
// values in parallel (luau-abi A6.3, carried by wit-abi W6.10). A plain
// tree satisfies this naturally — the assertion is the tripwire against a
// future variant smuggling in a non-Send/Sync payload.
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Value>();
};

impl Value {
    /// `true` only for the `Null` variant.
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// The payload if this is `Bool`, else `None`.
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// The payload if this is `Float`, else `None`. Never coerces `Int`.
    #[must_use]
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// The payload if this is `Int`, else `None`. Never coerces `Float`.
    #[must_use]
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// The payload if this is `String`, else `None`.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// The payload if this is `Bytes`, else `None`.
    #[must_use]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Bytes(b) => Some(b.as_ref()),
            _ => None,
        }
    }

    /// The payload if this is `Array`, else `None`.
    #[must_use]
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(a) => Some(a.as_slice()),
            _ => None,
        }
    }

    /// The payload if this is `Map`, else `None`.
    #[must_use]
    pub fn as_map(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Value::Map(m) => Some(m),
            _ => None,
        }
    }
}

// The `From` set is deliberately minimal: integers enter only as `i64`,
// so an untyped literal never silently picks a slot and out-of-range
// values fail loudly at the call site instead of coercing (strictness,
// wit-abi W6 D157). No `Default`: `Null` must be chosen, not happened.
impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Float(v)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Int(v)
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::String(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::String(v.to_owned())
    }
}

impl From<Arc<[u8]>> for Value {
    fn from(v: Arc<[u8]>) -> Self {
        Value::Bytes(v)
    }
}

impl From<Vec<u8>> for Value {
    fn from(v: Vec<u8>) -> Self {
        Value::Bytes(v.into())
    }
}

impl From<&[u8]> for Value {
    fn from(v: &[u8]) -> Self {
        Value::Bytes(v.into())
    }
}

impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Self {
        Value::Array(v)
    }
}

impl From<BTreeMap<String, Value>> for Value {
    fn from(v: BTreeMap<String, Value>) -> Self {
        Value::Map(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples() -> Vec<Value> {
        vec![
            Value::Null,
            Value::Bool(true),
            Value::Float(1.5),
            Value::Int(-7),
            Value::String("hello".to_owned()),
            Value::Bytes(Arc::from(&[1_u8, 2, 3][..])),
            Value::Array(vec![Value::Int(1)]),
            Value::Map(BTreeMap::from([("k".to_owned(), Value::Null)])),
        ]
    }

    #[test]
    fn accessors_pick_exactly_their_own_variant() {
        for value in samples() {
            let profile = (
                value.is_null(),
                value.as_bool().is_some(),
                value.as_float().is_some(),
                value.as_int().is_some(),
                value.as_str().is_some(),
                value.as_bytes().is_some(),
                value.as_array().is_some(),
                value.as_map().is_some(),
            );
            let expected = match &value {
                Value::Null => (true, false, false, false, false, false, false, false),
                Value::Bool(_) => (false, true, false, false, false, false, false, false),
                Value::Float(_) => (false, false, true, false, false, false, false, false),
                Value::Int(_) => (false, false, false, true, false, false, false, false),
                Value::String(_) => (false, false, false, false, true, false, false, false),
                Value::Bytes(_) => (false, false, false, false, false, true, false, false),
                Value::Array(_) => (false, false, false, false, false, false, true, false),
                Value::Map(_) => (false, false, false, false, false, false, false, true),
            };
            assert_eq!(profile, expected, "wrong accessor profile for {value:?}");
        }
    }

    #[test]
    fn accessors_return_the_payload() {
        assert_eq!(Value::Bool(true).as_bool(), Some(true));
        assert_eq!(Value::Float(1.5).as_float(), Some(1.5));
        assert_eq!(Value::Int(-7).as_int(), Some(-7));
        assert_eq!(Value::from("hello").as_str(), Some("hello"));
        assert_eq!(
            Value::from(vec![1_u8, 2, 3]).as_bytes(),
            Some(&[1_u8, 2, 3][..])
        );
        assert_eq!(
            Value::from(vec![Value::Int(1)]).as_array(),
            Some(&[Value::Int(1)][..])
        );
        let map = Value::from(BTreeMap::from([("k".to_owned(), Value::Null)]));
        assert_eq!(map.as_map().and_then(|m| m.get("k")), Some(&Value::Null));
    }

    #[test]
    fn from_conversions_pick_the_right_variant() {
        // Payload-pinning (not just variant-matching): the mutation tool's
        // body-replacement operator can't build a `Value`, so these
        // assertions are the only teeth over the `From` bodies.
        assert_eq!(Value::from(true), Value::Bool(true));
        assert_eq!(Value::from(1.5_f64), Value::Float(1.5));
        assert_eq!(Value::from(-7_i64), Value::Int(-7));
        assert_eq!(Value::from("s"), Value::String("s".to_owned()));
        assert_eq!(Value::from("s".to_owned()), Value::String("s".to_owned()));
        assert_eq!(Value::from(vec![1_u8]), Value::Bytes((&[1_u8][..]).into()));
        assert_eq!(Value::from(&[1_u8][..]), Value::Bytes((&[1_u8][..]).into()));
        let arc: Arc<[u8]> = Arc::from(&[1_u8][..]);
        assert_eq!(Value::from(arc), Value::Bytes((&[1_u8][..]).into()));
        assert_eq!(
            Value::from(vec![Value::Null]),
            Value::Array(vec![Value::Null])
        );
        assert_eq!(
            Value::from(BTreeMap::from([("k".to_owned(), Value::Null)])),
            Value::Map(BTreeMap::from([("k".to_owned(), Value::Null)]))
        );
    }

    #[test]
    fn equality_is_structural_and_tag_strict() {
        let tree = || {
            Value::from(BTreeMap::from([(
                "k".to_owned(),
                Value::from(vec![Value::Int(1), Value::from("s")]),
            )]))
        };
        assert_eq!(tree(), tree());
        assert_eq!(Value::Null, Value::Null);
        // Tag strictness: numeric closeness never crosses the Int/Float
        // variant boundary (wit-abi W6 D157).
        assert_ne!(Value::Int(1), Value::Float(1.0));
        assert_ne!(Value::Bool(true), Value::Int(1));
        // IEEE semantics inside Float: documented, not hidden.
        assert_ne!(Value::Float(f64::NAN), Value::Float(f64::NAN));
        // Bytes compare by content, not by Arc identity.
        let a = Value::from(vec![1_u8, 2]);
        let b = Value::from(&[1_u8, 2][..]);
        assert_eq!(a, b);
    }

    #[test]
    fn map_iteration_is_key_sorted() {
        // Canonical serialization and replay comparison lean on this order
        // (luau-abi A6.3); the test pins it against a silent container swap.
        let map = Value::from(BTreeMap::from([
            ("b".to_owned(), Value::Null),
            ("a".to_owned(), Value::Null),
            ("c".to_owned(), Value::Null),
        ]));
        let keys: Vec<&str> = map.as_map().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, ["a", "b", "c"]);
    }

    #[test]
    fn clone_shares_bytes_but_owns_containers() {
        // Bytes is Arc-backed (ADR-0042): cloning the Value shares the
        // immutable buffer instead of copying it.
        let bytes = Value::from(vec![1_u8, 2, 3]);
        let cloned = bytes.clone();
        match (&bytes, &cloned) {
            (Value::Bytes(a), Value::Bytes(b)) => assert!(Arc::ptr_eq(a, b)),
            _ => unreachable!(),
        }

        // Containers are owned: mutating a cloned array never leaks back.
        let original = Value::from(vec![Value::Int(1)]);
        let mut cloned = original.clone();
        if let Value::Array(items) = &mut cloned {
            items.push(Value::Int(2));
        }
        assert_eq!(original.as_array().map(<[Value]>::len), Some(1));
        assert_eq!(cloned.as_array().map(<[Value]>::len), Some(2));
    }
}
