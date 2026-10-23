# AINDB Roadmap

Milestones are sequential. A milestone is done when its acceptance tests
([docs/ACCEPTANCE.md](docs/ACCEPTANCE.md)) pass in CI.

## M0 — Specification and skeleton  ◄ current

- [x] Architecture & requirements spec ([docs/SPEC.md](docs/SPEC.md))
- [x] Threat model, CDK integration contract, acceptance-test list
- [x] Cargo workspace with crate boundaries and core traits
- [x] Acceptance-test stubs (`tests/acceptance`, all `#[ignore]`)
- [ ] Decide ADR-0001 storage backend (benchmark redb vs RocksDB vs SQLite)
- [ ] Decide ADR-0002 … ADR-0005
- [ ] Literature / prior-art review (event-sourced DBs, ledger DBs such as
      QLDB/immudb, Datomic, verifiable logs, agent-tool frameworks)

## M1 — Model and protocol

- `aindb-model`: `Value`, `Decimal` checked arithmetic, schemas, validation.
- `aindb-protocol`: `Request`/`Result`, strict typed (JSON) codec, limits.
- `aindb-storage`: in-memory backend implementing the full trait.
- Tests: AT-MOD-*, AT-PROT-01/02/04/05.

## M2 — Transaction engine

- Snapshot reads, private write sets, read-set tracking.
- Single-writer commit with optimistic validation (serializable).
- Schema-declared invariants, named operations (`transfer@1` fixture).
- Standalone `Authorizer` (policy file + Ed25519 client keys).
- Idempotent `tx_id`, dry run.
- Tests: AT-TX-*, AT-AUTH-*.

## M3 — Durable history and audit

- Durable backend chosen in ADR-0001; state + event in one atomic batch.
- Hash-chained events, rejection events (rate-limited).
- Hybrid signed checkpoints (Ed25519 + ML-DSA-65).
- `aindb verify` (chain, state-vs-history, anchors) and `aindb explain`.
- Crash-injection harness and recovery.
- Tests: AT-AUD-*, AT-REC-*.

## M4 — Machine-native protocol and verification

- Binary encoding (`AINDB-OP-v1` binary), equivalence with typed form.
- `aindb-verify` hook in lifecycle: required / advisory, timeouts.
- Deterministic reference verifier + seeded-fault corpus.
- cargo-fuzz targets for both decoders and event parser.
- Benchmarks for NFR-2..4.
- Tests: AT-PROT-03, AT-VER-*.

## M5 — CDK integration and server

- Unix-socket server mode.
- `aindb-cdk`: `CdkAuthorizer` (hybrid token verification, revocation),
  checkpoint anchoring into CDK audit log, `tool:aindb` gateway wiring.
- Tests: AT-CDK-*.

## M6 — Evaluation and v0.1 release

- Baseline: PostgreSQL + event table + conventional tool interface.
- Measurements EV-01..05 with reproducible scripts; written report.
- Release per SPEC §15.

## After v0.1 (not committed)

- Custom storage engine on the same trait.
- Declarative intent → validated operations.
- Independent-verifier project (layered, incl. AI reviewer).
- Replication and externally witnessed checkpoints.
- Quantum provenance ledger schemas.
