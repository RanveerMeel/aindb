#!/usr/bin/env bash
# Point git at the versioned hooks in .githooks/ (run once per clone).
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
chmod +x "${repo_root}"/.githooks/* "${repo_root}"/tools/*.sh
git -C "${repo_root}" config core.hooksPath .githooks
echo "Git hooks enabled from .githooks/ (commit-msg, pre-commit, pre-push)."
