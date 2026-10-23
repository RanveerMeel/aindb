# ADR-0003: Event encoding, hashing and retention

- Status: Proposed
- Date: 2026-10-23

## Context

Events must be canonical (same content ⇒ same bytes ⇒ same hash), cheap to
verify (NFR-3) and crypto-agile (P7).

## Options

- Encoding: custom deterministic TLV (shared with ADR-0004) vs CBOR
  deterministic profile (RFC 8949 §4.2) vs bincode/postcard.
- Hash: SHA-256 (matches CDK, FIPS) vs BLAKE3 (faster).
- Write records: full before/after state vs field diff.
- Rejection events: always vs rate-limited.

## Decision (proposed)

- Same deterministic TLV as the binary protocol, domain `AINDB-EVENT-v1`.
- SHA-256 (`hash_alg = 1`), algorithm id stored per event; BLAKE3 can be
  added as id 2 without format change.
- v0.1 stores full before/after for changed fields only (field diff with
  both values), secret fields hashed in the event body.
- Rejection events rate-limited per principal (default 10/s, then one
  summary event with a counter).
- No history compaction in v0.1 (compaction vs tamper evidence needs its
  own ADR).

## Consequences

- One canonical encoder used for protocol and history reduces bug surface.
- Hashing secret fields lets `verify` work without revealing them, but
  low-entropy secrets need salting (keyed hash, key in node config).
