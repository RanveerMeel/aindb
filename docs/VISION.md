# AINDB Long-Term Vision

This file records where AINDB came from and where it may go. It is context,
not a requirement. The binding document is [SPEC.md](SPEC.md).

## Origin

The starting question was whether AI will still need programming languages,
or whether it could emit machine code directly. The conclusion: AI can emit
binary, but the hard part was never the representation. It is specifying
intent precisely and **verifying** that the result does what was intended.
The bottleneck moves from writing instructions to defining intent, managing
constraints and checking outcomes.

Pushed further — one standard hardware interface, AI owning the whole
software lifecycle (specify → construct → verify → deploy → observe →
improve), humans seeing only outcomes — high-level languages and even the
traditional OS become implementation details. Their responsibilities
(isolation, scheduling, recovery, authorization) do not disappear; they move
into hardware, a minimal control layer (CDK), or AI-managed components.

Two gaps make outcome-only oversight unsafe:

1. **Not every failure shows up in the outcome.** A system can look correct
   while silently violating a rule. Humans need a trustworthy record of
   what happened, not the AI's recollection of it.
2. **An AI cannot be the final judge of its own work.** A second component
   must check, and for mechanically checkable properties that component must
   be deterministic, not another model.

AINDB addresses (1) directly and provides the hook for (2).

## Layers of the larger architecture

```
Human intent & constraints
        │
AI planner  +  independent verifier
        │            │
CDK — capabilities, approvals, attestation (trust kernel)
        │
AINDB — authoritative state, transactions, tamper-evident history   ◄ this repo
        │
Classical compute  │  (future) quantum subsystem
        │
Unified audit, evidence and recovery
```

## Independent verifier (next project)

Layered, in order of authority:

1. Deterministic rules — permissions, invariants, limits (in AINDB/CDK).
2. Formal verification — where properties can be proven.
3. Independent AI reviewer — logic errors, missing requirements, suspicious
   patterns. Advisory or required, never the sole authority.
4. Runtime monitoring — what actually happened after commit.
5. Cryptographic evidence — every verdict signed and bound to the event.

AINDB's `aindb-verify` trait is the integration point. A reference verifier
project will be specified after AINDB M4.

## Quantum computing

- The authoritative database stays **classical**: transactions, durable
  storage and precise records are not quantum strengths, and quantum states
  cannot be copied or read without disturbance.
- Future components: a quantum execution service and a **quantum provenance
  ledger** — algorithm, input references, hardware and calibration metadata,
  shot counts, measurement results, verification evidence — stored as
  ordinary AINDB objects/events.
- Verification of quantum results is statistical or protocol-based; the
  verifier records a confidence level, not a binary truth.
- The real near-term quantum impact is on **cryptography**. AINDB uses
  hybrid Ed25519 + ML-DSA-65 signatures (FIPS 204) and records algorithm ids
  for crypto agility, matching CDK.

## Beyond typed operations

Agent interface evolution, each step gated on evidence from the previous:

1. Typed operations (v0.1).
2. Compact binary encoding of the same operations (v0.1, M4).
3. Declarative intent compiled by AINDB into validated typed operations.
4. Engine-generated specialized execution plans (sandboxed, verified) —
   never agent-supplied code.

## Positioning

Major AI labs work on agents with tool access, data agents and multi-agent
safety. Publicly, none has stated a plan to rebuild the database around AI
as the primary client with verification inside the transaction lifecycle.
AINDB does not compete on models; it is model-independent infrastructure.
Novelty is **not** claimed until a literature and patent review is done
(ROADMAP M0 task).
