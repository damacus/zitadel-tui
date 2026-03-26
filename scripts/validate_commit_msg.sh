#!/usr/bin/env bash

set -euo pipefail

message_file="${1:?commit message file is required}"
message="$(sed -e '/^#/d' -e '/^$/d' "$message_file" | head -n 1)"

if [[ -z "$message" ]]; then
  echo "empty commit messages are not allowed" >&2
  exit 1
fi

pattern='^(build|chore|ci|docs|feat|fix|perf|refactor|revert|style|test)(\([a-z0-9._/-]+\))?!?: .+$'

if [[ "$message" =~ $pattern ]]; then
  exit 0
fi

cat >&2 <<'EOF'
commit message must use the Conventional Commits format:
  <type>(optional-scope): <description>

Allowed types:
  build, chore, ci, docs, feat, fix, perf, refactor, revert, style, test

Examples:
  feat: add user quick setup flow
  fix(auth): handle missing access token
  chore(ci): enforce conventional commits
EOF

exit 1
