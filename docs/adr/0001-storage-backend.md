# ADR-0001: Storage backend for the v0.1 prototype

- Status: Proposed
- Date: 2026-10-23

## Context

AINDB defines its own logical architecture but prototypes on an existing
store (SPEC §1, §10). The central guarantee — state and history committed in
one atomic, durable step — must be provided by the backend's write batch.
The backend sits behind the byte-oriented `aindb_storage::Backend` trait.

Requirements: atomic multi-key write batch, durable on return (fsync),
consistent read snapshots concurrent with the writer, ordered range scans,
crash safety, pure-Rust or well-maintained bindings, permissive license.

## Options

| Option | Pros | Cons |
|---|---|---|
| **redb** | Pure Rust, ACID, MVCC read snapshots, single file, simple | Younger project, single writer (fine for v0.1) |
| RocksDB (`rust-rocksdb`) | Battle-tested, high write throughput, WriteBatch + snapshots | C++ build, large dependency, tuning burden |
| SQLite (`rusqlite`) as KV | Extremely well tested, transactions | Tempts SQL coupling; C dependency |
| LMDB (`heed`) | Mature, MVCC, fast reads | Map size management, C dependency |
| Custom | Full control | Out of scope for v0.1 |

## Decision (proposed)

Use **redb** for v0.1, with RocksDB as the second backend used in M4
benchmarks to prove the trait is backend-neutral (every acceptance test runs
on in-memory, redb and, from M4, RocksDB).

To be accepted after a spike: 2-object transfer commit latency with fsync,
snapshot read under concurrent writes, crash-injection on 1 000 commits.

## Consequences

- Single-writer commit path matches ADR-0002.
- Torn-write handling is inherited from the backend; AT-REC-02 verifies it.
