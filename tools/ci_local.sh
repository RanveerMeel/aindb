#!/usr/bin/env bash
# Runs the build checks from .github/workflows/ci.yml locally.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
set -x
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build -p aindb-cli --no-default-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
