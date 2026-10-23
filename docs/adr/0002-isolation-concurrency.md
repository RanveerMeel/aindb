# ADR-0002: Isolation level and concurrency control

- Status: Proposed
- Date: 2026-10-23

## Context

Verifiers and invariants must see the exact state the transaction commits on
(SPEC P4). Agents generate bursts of small transactions. v0.1 is single node.

## Options

1. Single-writer, optimistic validation of read set at commit (serializable).
2. Snapshot isolation (MVCC) — allows write skew; would need explicit locks
   for invariants spanning objects.
3. Strict two-phase locking — simpler reasoning, deadlocks, verifier calls
   would hold locks for up to the verifier timeout.

## Decision (proposed)

Option 1. Transactions execute against a snapshot, record their read set
(object versions and scanned key ranges), and are validated under one writer
lock immediately before the atomic batch. Verifiers run before validation,
outside the lock, so slow verifiers never block other commits; a stale
snapshot aborts with retryable `Conflict`.

## Consequences

- Serializable without deadlocks; throughput bounded by single writer
  (acceptable for v0.1, measured in M4).
- Range reads must be tracked to prevent phantoms (AT-TX-06).
- Long verifier runs raise conflict rates on hot objects.
