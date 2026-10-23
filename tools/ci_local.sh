#!/usr/bin/env bash
# Runs the build checks from .github/workflows/ci.yml locally.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
set -x
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build -p aindb-cli --no-default-features
if command -v cargo-deny >/dev/null; then cargo deny --all-features check advisories bans licenses sources; fi
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
