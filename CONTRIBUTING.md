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

## Set up once per clone

```bash
tools/install_git_hooks.sh
```

This enables the versioned hooks in `.githooks/`:

| Hook | Checks |
|---|---|
| `pre-commit` | no commits on `main`, no conflict markers, no private keys, no files > 1 MiB, `cargo fmt` |
| `commit-msg` | commit standard below |
| `pre-push` | no pushes to `main`; runs `tools/ci_local.sh` (skip with `AINDB_SKIP_PUSH_CHECKS=1`; CI still runs) |

`main` is protected on GitHub: changes land only through pull requests with
all checks green.

## Commit standard

[Conventional Commits](https://www.conventionalcommits.org/) plus a
[DCO](https://developercertificate.org/) sign-off, enforced by
`tools/validate_commit_msg.sh` locally and in CI:

```text
type(scope): short imperative summary

Body explaining why (required for feat, fix, refactor).

Signed-off-by: Your Name <you@example.com>
```

- `type`: `feat` `fix` `docs` `style` `refactor` `perf` `test` `build` `ci` `chore` `revert`
- `scope`: required, lowercase kebab-case (`engine`, `audit`, `protocol`, `ci`, `docs`, …)
- Header 15–72 characters, starts lowercase, no trailing period
- No `WIP`, `tmp`, `fixup!` or `squash!` commits
- `!` after the scope marks a breaking change
- The PR title follows the same format; PRs are squash-merged with it

## Workflow

1. Fork and create a topic branch from `main` (`feat/*`, `fix/*`, `docs/*`,
   `chore/*`).
2. For non-trivial changes, open an issue first and align on approach.
   Design changes need an ADR in `docs/adr/`.
3. Run local validation, then open a PR with problem statement, change
   summary and test notes.

## Local validation

```bash
tools/ci_local.sh
```

## Pull-request checks

| Check | What it enforces |
|---|---|
| `fmt` | `cargo fmt` clean |
| `clippy` | no clippy warnings, all targets and features |
| `test` | unit + acceptance tests, CLI builds without CDK, lockfile current |
| `docs` | rustdoc builds without warnings |
| `msrv` | builds on the `rust-version` in `Cargo.toml` |
| `shellcheck` | hook and tool scripts |
| `commit-messages` | every commit: Conventional Commits + `Signed-off-by` |
| `pr-title` | PR title in commit format |
| `cargo-deny` | advisories, permissive licenses only, trusted sources (also weekly) |

## Rules for changes

- Respect crate boundaries (SPEC §10); only `aindb-engine` writes storage.
- No `unsafe` outside storage backends.
- Cryptography uses reviewed crates, stays hybrid (classical + PQC, both must
  verify) and records an algorithm id in every format.
- When implementing an acceptance test, remove its `#[ignore]` in the same PR.
- Every new failure path gets a negative test (tampered, truncated,
  unauthorized, oversized input).
