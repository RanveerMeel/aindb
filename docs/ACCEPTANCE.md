# AINDB Acceptance Tests

Each test has an ID, the milestone where it must pass, and an executable test
in `tests/acceptance/` (marked `#[ignore]` until its milestone).

Status: **M1 complete** — AT-MOD-01..05 and AT-PROT-01, 02, 04, 05 pass. A milestone
is complete only when all its tests pass on the in-memory **and** the durable
backend (from M3).

Fixture used throughout: schema `account@1 {owner: Text, balance: Decimal(2),
currency: Text}` with invariant `balance >= 0`, named op `transfer@1{from, to,
amount}` with invariant "sum of balances over touched accounts unchanged".

## Model & protocol (M1)

| ID | Test | Pass condition |
|---|---|---|
| AT-MOD-01 | Create object with valid schema | State validates, defaults filled (version 1 assigned by the engine in AT-TX-01) |
| AT-MOD-02 | Missing required field | `SchemaViolation{field_path}` |
| AT-MOD-03 | Wrong field type | `SchemaViolation{field_path}` |
| AT-MOD-04 | Decimal overflow in arithmetic | Checked arithmetic and parsing report overflow (engine abort without state change: AT-TX-03) |
| AT-MOD-05 | Object larger than limit | `LimitExceeded` |
| AT-PROT-01 | Typed JSON round trip | decode(encode(r)) == r for generated requests |
| AT-PROT-02 | Reject unknown field / trailing bytes / duplicate key | `Malformed`, nothing executed |
| AT-PROT-03 | Same request typed vs binary | Identical results and identical history events (except `time`) |
| AT-PROT-04 | Unknown protocol version | `Malformed{unsupported_version}` |
| AT-PROT-05 | 257 ops in one request | `LimitExceeded` |

## Transactions (M2)

| ID | Test | Pass condition |
|---|---|---|
| AT-TX-01 | Transfer A→B | Both balances updated, one event, one seq |
| AT-TX-02 | Transfer exceeding balance | `InvariantViolated{balance>=0}`, no state change, rejection event |
| AT-TX-03 | Transaction with failing 3rd op | First two ops not applied |
| AT-TX-04 | `expect_version` stale | `PreconditionFailed` |
| AT-TX-05 | 64 concurrent random transfers, 1000 rounds | Total money constant; result equals some serial order (checked by history replay) |
| AT-TX-06 | Write skew scenario (two txs each read both, write one) | One aborts with `Conflict` |
| AT-TX-07 | Resubmit committed `tx_id`, same body | Original result returned, no new event |
| AT-TX-08 | Reuse `tx_id`, different body | `Malformed{tx_id_reuse}` |
| AT-TX-09 | `dry_run` | `DryRunOk`, no state change, no commit event |
| AT-TX-10 | Named op that would create money (bad registration) | Registration rejected or invariant fails at commit |
| AT-TX-11 | Delete then get | Tombstone; `get(at=prev seq)` returns old state |
| AT-TX-12 | Unique index violation | `SchemaViolation{unique}` |

## Authorization (M2)

| ID | Test | Pass condition |
|---|---|---|
| AT-AUTH-01 | Unknown credential | `Unauthorized`, rejection event with no write set |
| AT-AUTH-02 | Agent allowed `get` only attempts `update` | `Unauthorized` |
| AT-AUTH-03 | Agent restricted to namespace `acct:` writes `hr:` | `Unauthorized` |
| AT-AUTH-04 | Principal field in `intent` spoofed | Event records authenticated principal, not claimed one |
| AT-AUTH-05 | Policy file changed | Event records new policy version; old events keep old version |

## Audit & explanation (M3)

| ID | Test | Pass condition |
|---|---|---|
| AT-AUD-01 | Every committed tx has exactly one event | Count(events committed) == count(Committed results); no orphan state |
| AT-AUD-02 | `explain tx` | Answers the six questions (SPEC §7.3); secret fields redacted |
| AT-AUD-03 | Flip one byte in an event | `verify` reports first bad seq |
| AT-AUD-04 | Delete a middle event | `verify` reports gap |
| AT-AUD-05 | Reorder two events | `verify` reports chain break |
| AT-AUD-06 | Edit state without event | `verify --state` reports state/history mismatch |
| AT-AUD-07 | Truncate history after last anchored checkpoint | `verify --anchors` reports truncation |
| AT-AUD-08 | Checkpoint signatures | Both Ed25519 and ML-DSA-65 verify; either failing ⇒ invalid |
| AT-AUD-09 | Explanation labels intent as claimed | Output separates "claimed" from "verified" |
| AT-AUD-10 | Rejected attempts recorded | Rejection events present, rate limit honoured |

## Recovery (M3)

| ID | Test | Pass condition |
|---|---|---|
| AT-REC-01 | Kill process at random points during 10 000 commits (100 runs) | After restart: every acknowledged commit present, no partial tx, chain verifies |
| AT-REC-02 | Torn write at end of storage | Recovery discards incomplete batch, verify passes |
| AT-REC-03 | Restart after clean shutdown | Final checkpoint present and valid |
| AT-REC-04 | Recovery time for 10 GB store | ≤ 60 s (NFR-4) |

## Verifier (M4)

| ID | Test | Pass condition |
|---|---|---|
| AT-VER-01 | Required verifier rejects | Abort, `VerifierRejected{id}`, evidence recorded |
| AT-VER-02 | Advisory verifier rejects | Commit proceeds, verdict recorded |
| AT-VER-03 | Verifier timeout | Treated as reject for required verifiers |
| AT-VER-04 | State changes between verify and commit | Commit aborts with `Conflict` (no TOCTOU) |
| AT-VER-05 | Verifier attempts to mutate proposal | Impossible by API (compile-time) and write set hash unchanged |
| AT-VER-06 | Verifier accepts invariant-violating tx | Deterministic invariant still rejects |
| AT-VER-07 | Seeded-fault corpus (100 wrong ops) | Report detection rate for reference verifier (measured) |

## CDK integration (M5)

| ID | Test | Pass condition |
|---|---|---|
| AT-CDK-01 | Valid CDK capability token for `aindb:acct/*` write | Authorized; event records CDK issuer id and token hash |
| AT-CDK-02 | Expired / revoked token | `Unauthorized` |
| AT-CDK-03 | Token with only Ed25519 or only ML-DSA valid | `Unauthorized` (hybrid requires both) |
| AT-CDK-04 | Checkpoint anchored to CDK audit log | CDK record references AINDB seq + head hash; AT-AUD-07 detects truncation via it |
| AT-CDK-05 | Engine built without `cdk` feature | All non-CDK tests pass |

## Evaluation (M6, measured not pass/fail)

| ID | Measurement |
|---|---|
| EV-01 | Concurrent correctness vs PostgreSQL + event table |
| EV-02 | Audit completeness and tamper detection vs baseline |
| EV-03 | Unauthorized agent actions blocked vs tool-wrapped baseline |
| EV-04 | Latency / throughput of typed vs binary vs SQL-via-tool |
| EV-05 | Verifier detection of seeded errors |

Total: 53 pass/fail tests + 5 evaluation measurements.
