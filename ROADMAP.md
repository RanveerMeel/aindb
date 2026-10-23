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

## Editions

AINDB follows an **open-core** model, like CDK. The split below is a
proposal and will be finalized before v0.1.

| Open-source core (Apache-2.0, this repository) | AINDB Enterprise (paid, separate, proprietary) |
|---|---|
| Engine, data model, transactions, storage trait and backends | High availability: replication, failover, clustering |
| Agent protocol (typed and binary) and its specification | Hardware key custody (HSM / TPM) for checkpoint keys |
| Audit format, hash chain, signed checkpoints, `aindb verify` | Encryption at rest with external key management |
| Verifier contract and reference deterministic verifier | Compliance report packs (sector-specific audit exports) |
| CDK adapter and standalone authorization | Certified builds and evaluation evidence |
| All cryptography and protocol formats | Support, SLAs, on-premises deployment assistance |

Rules that keep this honest:

- The open core is a complete database, not a demo. Single-node correctness,
  auditability and tamper detection are never paywalled.
- Cryptographic algorithms and data/protocol formats are always public, so
  open-source tools can verify data written by AINDB Enterprise.
- Security fixes land in the open core first or at the same time.
- Enterprise code lives in a separate private repository and depends on the
  public crates; nothing enterprise-only is committed here.
- Contributions are accepted under the [CLA](CLA.md), which lets the project
  steward also ship contributed code in AINDB Enterprise while guaranteeing it
  stays available here under Apache-2.0.

## After v0.1 (not committed)

- Custom storage engine on the same trait.
- Declarative intent → validated operations.
- Independent-verifier project (layered, incl. AI reviewer).
- Externally witnessed checkpoints (replication is an Enterprise feature, see Editions).
- Quantum provenance ledger schemas.
