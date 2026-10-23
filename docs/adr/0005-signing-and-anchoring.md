# ADR-0005: Checkpoint signing keys and CDK anchoring

- Status: Proposed
- Date: 2026-10-23

## Context

Signed checkpoints are the root of tamper evidence (SPEC A4/A5). CDK already
uses hybrid Ed25519 + ML-DSA-65 signatures and hash-chained audit with
signed checkpoints.

## Decision (proposed)

- Checkpoint key: hybrid Ed25519 + ML-DSA-65; a signature is valid only if
  **both** verify. Crates: `ed25519-dalek`, `ml-dsa` (same as CDK).
- Checkpoint body: `AINDB-CKPT-v1 ‖ node_id ‖ seq ‖ head_hash ‖ alg ids`.
- Interval: every 1024 events and on clean shutdown (configurable).
- Standalone: key in a file with 0600 permissions; optional export of
  checkpoints to a witness file/directory.
- CDK mode: checkpoints anchored into the CDK audit log
  (`AINDB-ANCHOR-v1`); node key optionally certified by the CDK registry.
- Hardware key custody is not part of the public project.

## Consequences

- CDK tooling can verify AINDB checkpoints with no new primitives.
- ML-DSA signing cost (~ms) is amortized over 1024 events.
