# AINDB Threat Model (v0.1)

## Assets

1. **Integrity of committed state** — balances, records, schemas.
2. **Integrity and completeness of history** — every change and rejection.
3. **Authorization boundary** — an agent acts only within its grants.
4. **Confidentiality of secret fields** — in state, events and explanations.
5. **Checkpoint signing key** — the root of tamper evidence.
6. **Availability** — secondary for v0.1; bounded by limits, not guaranteed.

## Trust boundaries

```
agent ──(1)── protocol decoder ──(2)── engine ──(3)── storage files
                                   │
                         (4) authorizer / CDK
                         (5) verifiers
                         (6) checkpoint key / anchor
```

(1) untrusted bytes · (2) typed but untrusted intent · (3) trusted only with
key-backed verification · (4) trusted for decisions, verified via signatures
in CDK mode · (5) trusted only to say no (when required) · (6) trusted.

## Adversaries and controls

| # | Threat | Control | Test |
|---|---|---|---|
| T1 | Malformed / oversized / adversarial encodings | Strict canonical decoders, limits, fuzzing | AT-PROT-02/05, NFR-5 |
| T2 | Agent exceeds its grants | Authorizer on every op, namespace-scoped grants, deny by default | AT-AUTH-02/03 |
| T3 | Agent spoofs identity in payload | Principal comes only from authentication; intent marked claimed | AT-AUTH-04, AT-AUD-09 |
| T4 | Agent submits harmful but allowed op (prompt injection) | Invariants, required verifiers, CDK approval gates; blast radius limited to grants; full audit | AT-TX-02, AT-VER-01 |
| T5 | Check-then-act race | Verify against snapshot; commit-time read-set validation | AT-VER-04, AT-TX-06 |
| T6 | Partial commit on crash | Single atomic batch for state + event | AT-REC-01/02 |
| T7 | History tampering on disk | Hash chain, state-vs-history check, signed checkpoints | AT-AUD-03..06 |
| T8 | Truncation / rollback of whole store | External anchoring of checkpoints (CDK or witness) | AT-AUD-07, AT-CDK-04 |
| T9 | Replay of a request | `tx_id` idempotency; CDK tokens carry expiry | AT-TX-07/08, AT-CDK-02 |
| T10 | Compromised verifier | Cannot mutate proposal; cannot override invariants; advisory vs required | AT-VER-05/06 |
| T11 | Secret leakage via explanations / errors | Sensitivity labels, redaction, errors without values | AT-AUD-02 |
| T12 | Quantum adversary forging checkpoint signatures | Hybrid Ed25519 + ML-DSA-65, algorithm ids recorded (crypto agility) | AT-AUD-08 |
| T13 | Resource exhaustion by agent | Per-request limits, scan limits, verifier timeouts; quotas post-v0.1 | AT-PROT-05 |
| T14 | Rejection-log flooding | Rate-limited rejection events with counters | AT-AUD-10 |

## Explicitly out of scope (v0.1)

- Malicious host root / operator with the signing key, beyond what
  anchoring detects.
- Side channels (timing, cache) on the host.
- Confidentiality against storage-level readers (no encryption at rest yet).
- Denial of service guarantees.
- Correctness of an agent's *intent* — AINDB proves what happened and that
  it was permitted, not that it was wise.

## Residual risks

- A hash chain is tamper-*evident*. Without frequent external anchoring, an
  attacker holding the node key can rewrite recent history.
- An authorized agent can still do authorized damage. Invariants and
  verifiers narrow this; they do not remove it.
- Explanations are only as precise as schemas and named-op registrations.
