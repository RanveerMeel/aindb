# AINDB — AI-Native Database

AINDB is a transactional database whose primary clients are AI agents, not
human-written applications. Agents talk to it through **typed operations**
(with an optional compact binary encoding). Humans inspect it through a
**human-readable audit trail** generated from the same authoritative,
tamper-evident history.

The database never trusts an agent because the agent says it succeeded. It
enforces its own permissions, constraints, transaction invariants and audit
guarantees, and it exposes a contract for an **independent verifier** to
check operations before they commit.

AINDB is a separate project that runs standalone or integrates with
[CDK](https://github.com/RanveerMeel/cdk), the quantum-safe trust kernel for
AI agents. CDK decides *which agent may do what*; AINDB guarantees *what
actually happened to the data* and can prove it.

> Status: **v0.1 design / pre-implementation.** The specification and crate
> boundaries are in place; the engine is not implemented yet. See
> [ROADMAP.md](ROADMAP.md).

## Core ideas

| Principle | Meaning |
|---|---|
| One source of truth | Current state and the committed history that explains it live in one engine, committed atomically together. |
| Machine-native interaction | Agents submit typed, validated operations, not SQL strings or generated programs. |
| Human-readable history | Every committed change answers: who, what, why, which rules, outcome, can it be trusted. |
| Verify before commit | Policy, invariants and an optional external verifier run before the commit, against the same snapshot. |
| Storage independence | Logical architecture is our own; v0.1 prototypes on an existing embedded store. |

## Repository layout

```
aindb/
├── docs/
│   ├── SPEC.md            Architecture & requirements specification (v0.1)
│   ├── ACCEPTANCE.md      Acceptance tests (AT-xxx), mapped to milestones
│   ├── THREAT_MODEL.md    Assets, adversaries, controls
│   ├── CDK_INTEGRATION.md Adapter contract with CDK
│   ├── VISION.md          Long-term direction (verifier, quantum, OS-less)
│   └── adr/               Architecture Decision Records
├── crates/
│   ├── aindb-model/       Objects, schemas, values, events, evidence, ids
│   ├── aindb-protocol/    Typed operations, requests/results, binary codec
│   ├── aindb-policy/      Principals, authorization, standalone policy
│   ├── aindb-verify/      Independent-verifier contract
│   ├── aindb-audit/       Hash-chained history, checkpoints, explanations
│   ├── aindb-storage/     Storage backend trait (atomic write batches)
│   ├── aindb-engine/      Propose → validate → verify → commit → audit
│   ├── aindb-cdk/         CDK adapter (capabilities, signed checkpoints)
│   └── aindb-cli/         `aindb` binary: run, inspect, explain, verify
├── tests/acceptance/      Executable acceptance tests (stubs until implemented)
└── ROADMAP.md             Milestones M0–M6
```

## Build

```bash
cargo test --workspace
```

Contributors: run `tools/install_git_hooks.sh` once, and see
[CONTRIBUTING.md](CONTRIBUTING.md) for the commit standard and PR checks.

## Editions

AINDB is **open core**. This repository (the engine, replication, protocol,
audit format and verifier, CDK adapter and all cryptography) is Apache-2.0 and fully usable
on its own. **AINDB Enterprise** is a separate, paid edition for regulated and
large deployments. See [ROADMAP.md § Editions](ROADMAP.md#editions).

## License

The open-source edition of AINDB is licensed under the **Apache License 2.0**,
the same license as [CDK](https://github.com/RanveerMeel/cdk).

- Full license text: [`LICENSE`](LICENSE)
- Attribution and trademark notes: [`NOTICE`](NOTICE)
- Contributions are accepted under the [Contributor License Agreement](CLA.md).
  You keep your copyright, and your code stays available here under Apache-2.0.
- How to contribute: [`CONTRIBUTING.md`](CONTRIBUTING.md)
- AINDB Enterprise is commercially licensed and is not part of this repository.
