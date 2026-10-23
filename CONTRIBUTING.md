# Contributing to AINDB

Thanks for helping build AINDB, an AI-native database with a tamper-evident,
human-readable audit trail. Read [ROADMAP.md](ROADMAP.md) for open milestones
and [docs/SPEC.md](docs/SPEC.md) for the binding design.

## Before your first contribution

1. **Sign the CLA.** All contributions are accepted under the
   [AINDB Contributor License Agreement](CLA.md). In your first pull request,
   comment:

   ```text
   I have read the AINDB Contributor License Agreement (CLA.md, version 1.0) and I hereby sign it.
   ```

   You keep your copyright. The CLA lets the project also ship your code in
   AINDB Enterprise, and guarantees it stays available here under Apache-2.0.
2. **Sign off every commit** with `git commit -s` (Developer Certificate of
   Origin).
3. **Report vulnerabilities privately** to the maintainers. Never open a
   public issue for a security bug.
4. **Never submit confidential or export-restricted material.**

## Workflow

1. Fork and create a topic branch from `main` (`feat/*`, `fix/*`, `docs/*`,
   `chore/*`).
2. For non-trivial changes, open an issue first and align on approach.
   Design changes need an ADR in `docs/adr/`.
3. Run local validation, then open a PR with problem statement, change
   summary and test notes.

## Local validation

These are the CI steps:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build -p aindb-cli --no-default-features
```

## Rules for changes

- Respect crate boundaries (SPEC §10); only `aindb-engine` writes storage.
- No `unsafe` outside storage backends.
- Cryptography uses reviewed crates, stays hybrid (classical + PQC, both must
  verify) and records an algorithm id in every format.
- When implementing an acceptance test, remove its `#[ignore]` in the same PR.
- Every new failure path gets a negative test (tampered, truncated,
  unauthorized, oversized input).
