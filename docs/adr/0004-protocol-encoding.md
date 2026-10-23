# ADR-0004: Agent protocol encodings

- Status: Proposed
- Date: 2026-10-23

## Context

Agents use typed operations first, with an optional compact binary encoding
of identical semantics (SPEC §5).

## Options

- Typed form: JSON with strict schema vs Protobuf text vs custom text.
- Binary: Protobuf vs FlatBuffers vs CBOR vs custom deterministic TLV.

## Decision (proposed)

- Typed form: **strict JSON** (LLMs emit it reliably; unknown fields, duplicate
  keys and trailing data rejected; decimals as strings).
- Binary form: **custom deterministic TLV** (`AINDB-OP-v1` magic + version
  byte), shared with the event encoder (ADR-0003). Protobuf rejected because
  it is not canonical by default and allows unknown fields.
- Both decode into `aindb_protocol::Request`; there is no engine path that
  sees the wire format.
- Publish a JSON Schema for the typed form so agent frameworks can generate
  tool definitions automatically.

## Consequences

- We own a binary format: it needs a spec document and fuzzing (M4).
- AT-PROT-03 enforces semantic equivalence.
