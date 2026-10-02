//! The `Value` data model shared by event payloads, service calls, and config
//! (ADR-0015): `Null / Bool / Float(f64) / String / Array / Map<String, _>` —
//! the JSON data model aligned with both Luau types and the YAML 1.2 core
//! schema. Pure data and zero-dependency by design (ADR-0019).
