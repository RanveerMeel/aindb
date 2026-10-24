# AINDB Architecture & Requirements Specification

Version 0.1 (draft) · 2026-10-23

Decisions fixed for v0.1:

- **Implementation language:** Rust (stable).
- **Storage:** independent logical architecture; the v0.1 prototype runs on an
  existing embedded key-value store behind the `aindb-storage` trait
  ([ADR-0001](adr/0001-storage-backend.md)).
- **Agent interface:** typed operations first; an optional compact binary
  encoding with identical semantics ([ADR-0004](adr/0004-protocol-encoding.md)).
- **Project boundary:** separate project; CDK integration through an adapter
  crate, never through CDK internals ([CDK_INTEGRATION.md](CDK_INTEGRATION.md)).

Requirement keywords MUST, SHOULD and MAY follow RFC 2119.

---

## 1. Purpose and scope

### 1.1 Problem

Agents today reach databases through tools that wrap SQL or REST APIs built
for human-written applications. Three things are missing:

1. A native operation interface designed for machine clients, with typed
   inputs, declared preconditions and structured results.
2. A history that a human can read and trust, generated from authoritative
   committed records rather than from an agent's account of what it did.
3. A transaction lifecycle in which an independent component can check an
   operation before it takes effect, without a check-then-act race.

### 1.2 Research question

Can a database designed around AI-native operations deliver better
verifiability, auditability and agent interaction than a conventional
database exposed to agents through tools or APIs?

v0.1 answers it with a prototype and a comparative evaluation against
PostgreSQL plus an event table plus a conventional tool interface (M6).

### 1.3 In scope (v0.1)

- Typed, schema-governed objects with versioned state.
- Multi-object atomic transactions with a single isolation level.
- Append-only, hash-chained history of committed changes and rejections,
  with signed checkpoints.
- Human-readable explanations generated from history.
- Pluggable authorization: standalone policy file or CDK capabilities.
- Pre-commit verifier hook (contract only; reference verifier is
  deterministic).
- Single node, embedded and local-socket server modes.
- Crash recovery.

### 1.4 Out of scope (v0.1)

- Replication, sharding, distributed transactions.
- A custom storage engine (designed for, not built).
- SQL or a general query language. v0.1 offers get, scan-by-index and
  history queries.
- An LLM-based verifier (the contract allows one; none ships).
- Quantum execution or quantum provenance (see [VISION.md](VISION.md)).
- Generated executable operations or agent-supplied code.
- Encryption at rest beyond what the backend provides (ADR later).

### 1.5 Terms

| Term | Meaning |
|---|---|
| Agent | Any client that submits operations. Usually an AI agent; may be a human tool. |
| Principal | The authenticated identity an operation runs as (agent id plus delegation chain). |
| Operation | A typed request: a named, versioned operation type plus typed inputs. |
| Transaction | A group of operations that commit or abort together. |
| Commit | The atomic, durable write of new state **and** its history event. |
| Event | An immutable record in the history: committed change, rejection or system action. |
| Evidence | Data attached to an event that justifies it: authorization decision, checks run, verifier verdict. |
| Checkpoint | A signed statement of the history head at sequence *n*. |
| Explanation | Human-readable rendering of events, derived only from stored records. |

---

## 2. Design principles

- **P1 One authoritative source.** State and history are written in the same
  atomic storage batch. No secondary log can disagree with state.
- **P2 The engine is the authority, not the agent.** Every guarantee in this
  document holds even if the agent is wrong, malicious or compromised.
- **P3 Semantics independent of encoding.** Typed and binary forms of an
  operation decode to the same internal value and are processed identically.
- **P4 Verify against the snapshot you commit on.** Checks run against a
  fixed read snapshot; the commit is rejected if that snapshot is no longer
  current for anything the checks read.
- **P5 Explanations are views.** The human trail is computed from events;
  it is never a separately stored narrative.
- **P6 Small trusted core.** Authorization, verification and explanation are
  behind traits. The engine core does not depend on CDK, an LLM or a network.
- **P7 Crypto agility.** Hash and signature algorithms are identified in
  every record that uses them so they can be replaced (ADR-0003).

---

## 3. Architecture

```
          AI agents (any model, any provider)
                     │ typed ops / binary ops
┌────────────────────▼─────────────────────────────────────┐
│ aindb-protocol   decode · validate shape · size limits   │
├──────────────────────────────────────────────────────────┤
│ aindb-engine                                             │
│   1 authenticate ─► aindb-policy (standalone | aindb-cdk)│
│   2 snapshot + validate (schema, preconditions)          │
│   3 invariants (schema-declared, deterministic)          │
│   4 verify ─────► aindb-verify (optional, external)      │
│   5 commit: state + event in ONE storage batch           │
│   6 result + receipt                                     │
├─────────────────────────────┬────────────────────────────┤
│ state (objects, indexes)    │ aindb-audit (chain, ckpts, │
│                             │   explain)                 │
├─────────────────────────────┴────────────────────────────┤
│ aindb-storage  trait: snapshot read, atomic write batch  │
│   v0.1 backend: existing embedded KV store (ADR-0001)    │
└──────────────────────────────────────────────────────────┘
          humans ◄── aindb-cli explain / verify / history
```

### 3.1 Logical subsystems

| Subsystem | Responsibility | Crate |
|---|---|---|
| Machine-native interface | Decode, size-limit and shape-check operations; encode results | `aindb-protocol` |
| Transaction & policy engine | Authentication, authorization, validation, isolation, commit protocol | `aindb-engine`, `aindb-policy` |
| State engine | Current object versions and secondary indexes | `aindb-engine` (on `aindb-storage`) |
| Event engine | Append-only history, sequence numbers, hash chain | `aindb-audit` |
| Durable storage & recovery | Atomic batches, fsync policy, recovery | `aindb-storage` |
| Audit & explanation | Checkpoints, integrity verification, readable output | `aindb-audit`, `aindb-cli` |
| Verification interface | Contract for independent checkers | `aindb-verify` |

---

## 4. Data model

### 4.1 Primitives

| Primitive | Definition | Mutable? |
|---|---|---|
| **Object** | Durable entity with an `ObjectId`, a `SchemaId` and a version counter | identity fixed |
| **State** | Attribute values of an object at a version | replaced per commit |
| **Event** | Immutable history record with sequence number and chain hash | never |
| **Transaction** | Ordered set of operations committed atomically under one `TxId` | n/a |
| **Evidence** | Authorization decision, checks run, verifier verdicts, request metadata | never (part of event) |

Example: a bank account is an object; its balance is state; a committed
deposit is an event; a transfer touching two accounts is a transaction; the
authorization and the "no money created" check are evidence.

### 4.2 Identifiers

- `ObjectId`: `ns:key`, chosen by the creating agent. `ns` is
  `[a-z0-9_-]{1,32}` and is the unit of authorization; `key` is
  `[A-Za-z0-9._-]+`; at most 255 bytes in total.
- `SchemaId`: `name@major` (e.g. `account@1`); named operations use the
  same form (`transfer@1`).
- `TxId`: `[A-Za-z0-9._:-]{1,64}`, caller-supplied idempotency key.
- `Seq`: u64, dense, strictly increasing history sequence number.
- `Version`: u64 per object, starts at 1, +1 per committed change.

### 4.3 Values

`Value` = `Null | Bool | I64 | U64 | Decimal(i128, scale u8) | Text(UTF-8) |
Bytes | Timestamp(UTC ns) | Ref(ObjectId) | List(Value) | Map(Text→Value)`.

- No floating point in v0.1 (non-deterministic comparisons, money hazards).
- `Decimal` carries an explicit scale; arithmetic MUST be checked
  (overflow aborts the transaction).
- Max encoded object state: 1 MiB (configurable lower).

### 4.4 Schemas

A schema declares: fields (name, type, required, default), indexes
(unique / non-unique, single or composite field), field sensitivity
(`public | internal | secret`, used by explanations), and **invariants**.

Invariants are declarative, deterministic expressions over the
post-transaction state of objects the transaction touched, for example
`balance >= 0` (per object) and `sum(balance) unchanged over op transfer`
(per operation type). No user code, no I/O, bounded evaluation cost.

Schemas are themselves objects (`schema@1`), so schema changes are
transactions with history. Only additive changes are allowed within a major
version; a new major requires an explicit migration operation (post-v0.1).

### 4.5 Operations

Built-in operation types for v0.1:

| Op | Effect |
|---|---|
| `create(schema, id, state)` | New object at version 1 |
| `update(id, expect_version?, patch)` | Field-level patch |
| `delete(id, expect_version?)` | Tombstone (history kept) |
| `get(id, at?)` | Read current or historical version |
| `scan(index, range, limit)` | Bounded index range read |
| `history(id \| tx \| principal, range)` | Read events |
| `define_schema(schema)` | Create schema / additive change |
| `named(op_type@v, inputs)` | Schema-registered composite operation (e.g. `transfer@1`) |

**Named operations** are how domain actions such as `transfer` are expressed.
A named operation is registered with: typed input signature, the
reads/writes it performs expressed with the built-in ops, preconditions and
invariants. Agents call `transfer@1{from, to, amount}`; the engine expands it
deterministically. Agents never submit code.

### 4.6 Storage of state vs history

Not pure event sourcing. The engine keeps **current state + the append-only
committed history**, written together, plus checkpoints for fast integrity
verification. Historical versions of objects are reconstructible from history
(`get(id, at=seq)`); v0.1 MAY store full prior states in events rather than
diffs (ADR-0003).

---

## 5. Agent interface

### 5.1 Request

```text
Request {
  protocol:      "AINDB-OP-v1"
  tx_id:         TxId                 // idempotency key
  principal:     PrincipalClaim       // credential or CDK token
  ops:           [Operation]          // 1..=256
  preconditions: [Precondition]       // e.g. version(id)==7, exists(id)
  intent:        { reason: Text, request_ref: Text? }   // recorded, not trusted
  options:       { dry_run: bool, want_explanation: bool }
}
```

Illustrative typed form:

```json
{
  "protocol": "AINDB-OP-v1",
  "tx_id": "tx-1042",
  "ops": [{ "named": "transfer@1",
            "inputs": { "from": "acct:A-100", "to": "acct:A-200",
                        "amount": { "value": "10000.00", "currency": "INR" } } }],
  "preconditions": [],
  "intent": { "reason": "customer_requested_transfer", "request_ref": "ticket-88" }
}
```

### 5.2 Result

```text
Result {
  tx_id, status: Committed | Rejected | Aborted | DryRunOk,
  seq: Seq?,                     // history position when committed
  reads/writes: [(ObjectId, Version)],
  error: { code: ErrorCode, field_path: Text?, detail: Text }?,
  receipt: { seq, event_hash, checkpoint_ref? },
  explanation: Text?             // only if requested
}
```

Errors are typed codes (`Unauthorized`, `SchemaViolation`,
`PreconditionFailed`, `InvariantViolated{name}`, `Conflict`,
`VerifierRejected{verifier}`, `LimitExceeded`, `Malformed`, `Internal`) so
agents can react without parsing prose.

### 5.3 Encodings

- **Typed (canonical text):** JSON with a strict schema. For debugging,
  humans and simple clients. Reference: [PROTOCOL.md](PROTOCOL.md).
- **Binary (`AINDB-OP-v1` binary):** length-prefixed, deterministic,
  versioned encoding of the same structure (ADR-0004).
- Both decode to the same `aindb_protocol::Request`. Acceptance test
  AT-PROT-03 asserts identical results and identical history for both.
- Decoders MUST reject unknown fields, trailing bytes, duplicate map keys,
  non-canonical integers, floating-point numbers and inputs over size
  limits. Object size limits are measured on the canonical typed encoding,
  so they do not depend on whitespace or wire format.

### 5.4 Transport

v0.1: in-process (embedded library) and a local Unix-domain socket with
length-prefixed frames. Network transport and CDK link transport are M5.

---

## 6. Transaction semantics

### 6.1 Isolation

v0.1 provides **serializable** isolation for writing transactions via a
single-writer commit path with optimistic validation (ADR-0002):

1. A transaction reads from snapshot `S` (history head at `seq_S`).
2. The engine records the read set (object ids + versions, index ranges).
3. At commit, under the writer lock, if any object in the read set or any
   key in a scanned range changed after `seq_S`, the transaction aborts with
   `Conflict` (retryable). Otherwise it commits at `seq_S'+1`.

Read-only requests are served from a consistent snapshot without the lock.

### 6.2 Lifecycle

```
received ─► decoded ─► authenticated ─► authorized
   ─► snapshot taken ─► preconditions ─► ops applied to private write set
   ─► schema checks ─► invariants ─► verifier(s) ─► commit-validate (P4)
   ─► atomic batch {state writes, index writes, event, chain head} ─► durable
   ─► result + receipt
```

Any failure before the batch is written produces no state change. Rejections
caused by authorization, invariants or verifiers are **themselves recorded**
as `Rejected` events (with evidence, without the rejected write set's secret
values), so that refused attempts are auditable (rate-limited, ADR-0003).

### 6.3 Guarantees

- **Atomicity:** all writes of a transaction and its event are in one storage
  batch; a crash leaves either all or none (AT-TX-01, AT-REC-01).
- **Consistency:** schema rules and invariants hold in every committed state.
- **Isolation:** serializable for read-write transactions (AT-TX-05).
- **Durability:** a `Committed` result is returned only after the batch is
  durable per the configured sync policy (default: fsync per commit).
- **Idempotency:** resubmitting a committed `tx_id` with an identical request
  returns the original result; a different request with the same `tx_id` is
  `Malformed`.
- **Determinism:** given the same prior state and request, validation,
  invariant evaluation and the resulting write set are identical.

### 6.4 Limits (defaults, configurable)

256 ops per transaction · 1 MiB per object · 8 MiB per transaction ·
10 000 objects scanned per request · 5 s verifier timeout (timeout ⇒ reject).

---

## 7. Audit guarantees

### 7.1 Event record

```text
Event {
  seq, prev_hash, hash,             // hash = H(domain ‖ seq ‖ prev_hash ‖ body)
  time (engine clock, informational),
  kind: TxCommitted | TxRejected | SchemaChanged | System(..),
  tx_id, principal (id + delegation chain + auth method),
  intent (as submitted, marked "claimed"),
  ops (canonical binary form),
  writes: [(ObjectId, old_version, new_version, before?, after?)],
  evidence: { authz decision + policy version,
              checks: [(name, version, result)],
              verifiers: [(verifier_id, verdict, signature?)] },
  hash_alg, sig_alg ids
}
```

Domain separator: `AINDB-EVENT-v1`. Checkpoint domain: `AINDB-CKPT-v1`.

### 7.2 Properties

- **A1 Completeness:** every committed transaction has exactly one event; no
  state change exists without an event (AT-AUD-01).
- **A2 Ordering:** `seq` is dense and strictly increasing; each event commits
  to its predecessor's hash.
- **A3 Tamper evidence:** modification, deletion, insertion or reordering of
  any event, or a state value that disagrees with history, is detected by
  `aindb verify` (AT-AUD-03..07).
- **A4 Signed checkpoints:** every N events (default 1024) and on clean
  shutdown the engine signs `(seq, head_hash, state_root?)` with its node key.
  Signatures are hybrid Ed25519 + ML-DSA-65 to match CDK (ADR-0005).
- **A5 External anchoring (optional):** checkpoints MAY be exported to CDK's
  audit log or another witness; truncation after the last anchored
  checkpoint is then detectable.
- **A6 Explanations from records only:** `explain` uses events, schemas and
  sensitivity labels. Secret fields render as `<redacted:hash-prefix>`.
- **A7 Claimed vs verified:** an explanation distinguishes what the agent
  claimed (`intent`) from what the engine verified (authz, checks).

Honest limit: a hash chain makes tampering *detectable*, not impossible.
An attacker with storage access and the signing key can rewrite history
after the last externally anchored checkpoint. Key custody and anchoring
frequency define the real guarantee (see threat model).

### 7.3 The six questions

For any committed transaction, `aindb explain tx <id>` MUST answer:

1. **Who** initiated it (principal, delegation, auth method).
2. **What** changed (object, field, before → after, redacted where secret).
3. **Why** it was requested (claimed intent, labelled as claimed).
4. **Which rules** were checked (policy version, invariants, verifiers).
5. **Outcome** (committed / rejected / aborted, seq).
6. **Can it be trusted** (chain position, last covering checkpoint, verify
   status).

---

## 8. Authorization and verification

### 8.1 Authorization (`aindb-policy`)

```rust
trait Authorizer {
    fn authenticate(&self, claim: &PrincipalClaim) -> Result<Principal, AuthError>;
    fn authorize(&self, p: &Principal, op: &OpSummary, snap: &dyn SnapshotView)
        -> Decision; // Allow{policy_ref} | Deny{reason} | NeedsApproval{..}
}
```

- **Standalone:** policy file of `(principal pattern, op type, namespace,
  schema) → allow/deny`, static API keys or Ed25519 client keys.
- **CDK:** `aindb-cdk` verifies CDK capability tokens; permissions map to
  op types and namespaces (CDK_INTEGRATION.md).
- `NeedsApproval` is surfaced as `Rejected{NeedsApproval}` in v0.1; human
  approval flows are delegated to CDK.

### 8.2 Verification (`aindb-verify`)

```rust
trait Verifier {
    fn id(&self) -> VerifierId;
    fn verify(&self, proposal: &Proposal) -> Verdict; // Accept | Reject{reason} | Abstain
}
```

- `Proposal` = request, principal, snapshot seq, read set, write set
  (before/after), invariants already passed.
- Verifiers run **after** deterministic checks and **before** commit
  validation; the snapshot they saw is the one committed against (P4).
- Configured per op type: `required` (Reject or timeout ⇒ abort) or
  `advisory` (verdict recorded only).
- A verifier MUST NOT be able to modify the write set.
- Deterministic invariants are never delegated to a verifier. An LLM
  verifier is an additional layer, never the authority for mechanically
  checkable properties.

---

## 9. Threat model (summary)

Full version: [THREAT_MODEL.md](THREAT_MODEL.md).

| Adversary | Can | Must not be able to |
|---|---|---|
| Faulty agent | Send malformed, wrong or contradictory ops | Corrupt state, partial-commit, bypass invariants |
| Malicious agent (valid creds) | Send any op allowed by its grants | Exceed grants, forge principal, hide actions, poison explanations as "verified" |
| Prompt-injected agent | Act as malicious agent within its grants | Same as above; blast radius = its grants |
| Network attacker (M5) | Observe/modify traffic | Forge/replay requests (auth + tx_id idempotency) |
| Storage-level attacker, no key | Modify/delete files | Avoid detection on `verify` |
| Storage attacker with node key | Rewrite unanchored history | Rewrite before last anchored checkpoint undetected |
| Compromised verifier | Reject or accept wrongly | Change write set; override deterministic checks |
| Malicious operator/root | Everything on host | Out of scope for v0.1 beyond anchoring |

---

## 10. Module boundaries

| Crate | Depends on | Must not depend on |
|---|---|---|
| `aindb-model` | — | anything else in workspace |
| `aindb-protocol` | model | storage, engine |
| `aindb-policy` | model, protocol | storage, engine, cdk |
| `aindb-verify` | model, protocol | storage, engine |
| `aindb-audit` | model, protocol | engine, cdk |
| `aindb-storage` | — | model semantics (bytes only) |
| `aindb-engine` | all above | cdk, cli, any network/LLM crate |
| `aindb-cdk` | policy, verify, audit, (cdk-link) | engine internals |
| `aindb-cli` | engine, audit, cdk (feature) | — |

Rules:
- Only `aindb-engine` writes to storage.
- `aindb-storage` is byte-oriented: `get`, `range`, `snapshot`,
  `write_batch(atomic)`. Swapping the backend must not change any
  acceptance-test outcome.
- `aindb-cdk` is behind a cargo feature in `aindb-cli`; the engine builds
  and passes all non-CDK tests without it.
- No `unsafe` outside `aindb-storage` backends (`#![forbid(unsafe_code)]`
  elsewhere).

---

## 11. Non-functional requirements

| ID | Requirement |
|---|---|
| NFR-1 | Linux x86_64 and aarch64; stable Rust. |
| NFR-2 | Target p50 commit latency for a 2-object transfer ≤ 2 ms on NVMe with fsync per commit (a target to measure in M4, not a promise). |
| NFR-3 | Full-chain verify ≥ 50 000 events/s single core. |
| NFR-4 | Recovery after crash of a 10 GB store ≤ 60 s. |
| NFR-5 | Decoders fuzzed (cargo-fuzz) with no crash in 10 M iterations before v0.1 release. |
| NFR-6 | Deterministic: same inputs ⇒ byte-identical events (except `time`). |
| NFR-7 | `#![forbid(unsafe_code)]` in all non-backend crates. |
| NFR-8 | All crypto via audited crates; algorithm ids recorded; no custom primitives. |

---

## 12. Milestones

See [ROADMAP.md](../ROADMAP.md). Summary:

| M | Deliverable | Exit criteria |
|---|---|---|
| M0 | Spec, ADRs, workspace skeleton | This repo; ADRs 0001–0005 accepted |
| M1 | Model + protocol + typed codec + in-memory backend | AT-PROT-*, AT-MOD-* pass |
| M2 | Engine: transactions, schemas, invariants, standalone policy | AT-TX-*, AT-AUTH-* pass |
| M3 | Durable backend, history, hash chain, checkpoints, explain, verify, recovery | AT-AUD-*, AT-REC-* pass |
| M4 | Binary encoding, verifier hook, fuzzing, benchmarks | AT-VER-*, AT-PROT-03, NFR-2..5 measured |
| M5 | CDK adapter, socket server, checkpoint anchoring to CDK | AT-CDK-* pass |
| M6 | Comparative evaluation vs PostgreSQL baseline; v0.1 release | Report published |

## 13. Acceptance tests

Listed in [ACCEPTANCE.md](ACCEPTANCE.md). Each has an ID, milestone and an
executable stub under `tests/acceptance/`.

## 14. Open decisions

Tracked as ADRs: storage backend (0001), isolation & concurrency (0002),
audit encoding & hashing (0003), protocol encoding (0004), signing keys &
CDK anchoring (0005). Later: encryption at rest, schema migration, history
compaction/retention vs. tamper evidence, multi-node.

## 15. v0.1 release criteria

- All AT-* tests for M1–M5 pass in CI.
- `aindb verify` detects every tamper case in AT-AUD-03..07.
- Decoder fuzzing per NFR-5.
- Evaluation report (M6) with reproducible scripts.
- Threat model reviewed against the implementation.
