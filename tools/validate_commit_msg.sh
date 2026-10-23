#!/usr/bin/env bash
# Validates a commit message against the AINDB commit standard:
#   * Conventional Commits header: type(scope): summary
#   * Developer Certificate of Origin: a Signed-off-by trailer
# Used by .githooks/commit-msg and by CI on every pull-request commit.
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <commit-msg-file>" >&2
  exit 2
fi

msg_file="$1"
# Drop comment lines git adds to the editor template.
msg="$(grep -v '^#' "$msg_file" | tr -d '\r')"
header="$(printf '%s\n' "$msg" | sed -n '1p')"
body="$(printf '%s\n' "$msg" | sed -n '2,$p')"

allowed_types="feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert"
header_regex="^(${allowed_types})\\(([a-z0-9]+(-[a-z0-9]+)*)\\)(!)?: [a-z][[:print:]]+$"

fail() {
  echo "ERROR: $*" >&2
  echo "See CONTRIBUTING.md#commit-standard" >&2
  exit 1
}

[[ -n "$header" ]] || fail "empty commit message header."

# Merge commits created by git are exempt from the header format.
if [[ "$header" =~ ^Merge\  ]]; then
  exit 0
fi

[[ ! "$header" =~ ^(fixup!|squash!|amend!) ]] || fail "fixup/squash commits are not allowed; squash them before pushing."
[[ ! "$header" =~ (^|[^[:alpha:]])([Ww][Ii][Pp]|[Tt][Mm][Pp])([^[:alpha:]]|$) ]] || fail "WIP/tmp commits are not allowed."

if ! [[ "$header" =~ $header_regex ]]; then
  fail "commit header must match 'type(scope): short imperative summary'
  allowed types: ${allowed_types//|/ }
  scope: required, lowercase kebab-case (e.g. engine, audit, ci, docs)"
fi

header_len="${#header}"
(( header_len >= 15 && header_len <= 72 )) || fail "commit header must be 15-72 characters (is ${header_len})."
[[ ! "$header" =~ \.$ ]] || fail "commit header must not end with a period."

if [[ -n "$(printf '%s\n' "$body" | sed -n '1p')" ]]; then
  fail "second line must be blank (separates header from body)."
fi

type="${header%%(*}"
if [[ "$type" == "feat" || "$type" == "fix" || "$type" == "refactor" ]]; then
  context="$(printf '%s\n' "$body" | grep -v -E '^(Signed-off-by|Co-Authored-By|Co-authored-by):' | sed '/^[[:space:]]*$/d')"
  [[ -n "$context" ]] || fail "${type} commits must include a body explaining why."
fi

if ! printf '%s\n' "$msg" | grep -q -E '^Signed-off-by: .+ <[^>]+@[^>]+>$'; then
  fail "missing 'Signed-off-by: Name <email>' trailer (Developer Certificate of Origin). Use 'git commit -s'."
fi
