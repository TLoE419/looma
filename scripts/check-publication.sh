#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

# Inspect the index, since ignore rules alone cannot stop a forced add.
git ls-files --cached -z | {
    failed=0
    while IFS= read -r -d '' path; do
        case "$path" in
            target|target/*|generated-images|generated-images/*|publication-review|publication-review/*|\
            .codex|.codex/*|*/.codex|*/.codex/*|\
            .aws|.aws/*|*/.aws|*/.aws/*) ;;
            .env.example|*/.env.example) continue ;;
            .env|.env.*|*/.env|*/.env.*|\
            *.pem|*.key|*.p12|*.pfx|*.sqlite|*.sqlite3|*.db|*.db-wal|*.db-shm) ;;
            *) continue ;;
        esac
        printf 'Blocked publication path: %q\n' "$path" >&2
        failed=1
    done
    exit "$failed"
}
