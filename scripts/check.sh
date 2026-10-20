#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

case "${1:-all}" in
    quality)
        cargo fmt -- --check
        cargo clippy --locked --all-targets -- -D warnings
        ;;
    test)
        cargo test --locked
        ;;
    security)
        bash scripts/test-publication-guard.sh
        bash scripts/check-publication.sh
        cargo audit --deny warnings
        ;;
    all)
        bash scripts/check.sh quality
        bash scripts/check.sh test
        bash scripts/check.sh security
        ;;
    *)
        printf 'Usage: bash scripts/check.sh [quality|test|security|all]\n' >&2
        exit 2
        ;;
esac
