#!/usr/bin/env bash
set -euo pipefail
guard="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/check-publication.sh"
probe_dir=$(mktemp -d "${TMPDIR:-/tmp}/looma-publication-guard.XXXXXX")
trap 'rm -rf "$probe_dir"' EXIT
mkdir -p "$probe_dir/scripts" "$probe_dir/fixtures"
cp "$guard" "$probe_dir/scripts/"
cd "$probe_dir"
git init --quiet
printf '# No credentials here\n' > .env.example
printf '# No credentials here\n' > fixtures/.env.example
printf 'Synthetic fixture\n' > fixtures/example.png
git add -- scripts .env.example fixtures/.env.example fixtures/example.png
bash scripts/check-publication.sh

for path in generated-images/example.png publication-review/report.md target/output \
    .codex/auth.json .aws/credentials .env .env.local nested/.env.production \
    nested/.aws/credentials private.key client.pem state.sqlite state.db-wal \
    .aws/.env.example publication-review/.env.example \
    $'generated-images/line\nbreak.png'; do
    mkdir -p "$(dirname "$path")"
    printf 'Synthetic probe, no real credentials\n' > "$path"
    git add -f -- "$path"
    if bash scripts/check-publication.sh > "$probe_dir/result.log" 2>&1; then
        printf 'Guard incorrectly accepted: %q\n' "$path" >&2
        exit 1
    fi
    if [[ $(< "$probe_dir/result.log") != *"Blocked publication path:"* ]]; then
        cat "$probe_dir/result.log" >&2
        exit 1
    fi
    git rm --quiet --cached -- "$path"
done
bash scripts/check-publication.sh
printf 'Publication guard: valid paths accepted; 16 forbidden-path probes rejected.\n'
