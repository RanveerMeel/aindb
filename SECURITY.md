# Security Policy

AINDB is pre-release (v0.x). Only the latest commit on `main` receives fixes.

## Reporting a vulnerability

Report privately through
[GitHub Security Advisories](https://github.com/RanveerMeel/aindb/security/advisories/new).
Do **not** open a public issue, pull request or discussion.

Include the affected commit, a description, reproduction steps and impact.
You will get an acknowledgement within 7 days. Fixes land in the open-source
edition first or at the same time as in AINDB Enterprise.

## Scope

In scope: anything that breaks a guarantee in [docs/SPEC.md](docs/SPEC.md)
or [docs/THREAT_MODEL.md](docs/THREAT_MODEL.md), e.g. partial commits,
authorization bypass, undetected history tampering, secret leakage in
explanations, decoder crashes on untrusted input.
