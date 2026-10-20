# GitHub automated checks

Pull requests, pushes, and manual dispatch run independently visible jobs:

| GitHub result | Local command | Scope |
| --- | --- | --- |
| Code quality | `bash scripts/check.sh quality` | Rust formatting and warnings-denied Clippy on macOS |
| Functional tests | `bash scripts/check.sh test` | Local Rust tests on macOS; ignored live-service tests stay ignored |
| Security checks | `bash scripts/check.sh security` | Publication paths, official-source patch reconstruction and strict full-lockfile RustSec audit |
| Dependency patch tests | `python3 scripts/check-dependency-patches.py --test` | Optimized GLib and macro tests on Linux with GTK development libraries |

The existing required name **macOS checks** is retained as an aggregate. It runs
with `always()` and passes only if all four jobs succeeded. A failed, cancelled,
or skipped dependency fails the aggregate. No branch-protection settings need
to change, and none are changed by this PR. The legacy name is retained for
compatibility even though security runs on Linux.

## Local prerequisites

Use the pinned Rust 1.90.0 toolchain, with rustfmt and Clippy. Security additionally
requires:

```sh
cargo install cargo-audit --version 0.22.2 --locked
```

`bash scripts/check.sh` runs all three groups and returns a failure if any fails.
Quality/tests can use `CARGO_NET_OFFLINE=true` when dependencies are cached.
Security needs network access to refresh the RustSec database and inspect yanked
packages; do not claim a cached advisory snapshot is a fresh security result.
The dependency remediation changes the application lockfile and adds four source overrides; see the patch guide for the exact scope.

## Dependency remediation and ongoing security requirements

The five original advisories are addressed by source-tracked parent updates and
backports described in [DEPENDENCY_PATCHES.md](DEPENDENCY_PATCHES.md). Local path
sources are not covered like registry packages by RustSec: the security job
therefore verifies the exact source inventory and reconstructs each patch from
its checksum-verified official archive. A separate required Linux job tests
patched libraries and diagnostic regressions with a locked verification workspace.

Security failures still block the existing required aggregate. No ignore IDs,
target filters, `continue-on-error`, branch-protection changes or audit bypass
are introduced. A passing audit alone is insufficient for accepting local patches.

## Publication-path protection

The guard inspects paths tracked in the Git index, including forced additions.
It rejects generated images, build output, private publication evidence,
credential directories, environment files (except sanitized `.env.example`
outside forbidden directories), key containers, and database files. Normal
image fixtures remain allowed. The self-test checks 16 forbidden cases plus
valid sample files, including nested paths and newline-containing filenames.

This is path-based protection, not content-based secret detection: arbitrary
renamed secrets, untracked files, and commit history are outside its scope.
GitHub secret scanning/push protection and human review remain necessary.
Review workflow/script changes themselves: a PR can edit its own checks.

## Execution and evidence limits

No service credentials or image-generation requests are used by CI. This does
not exercise the desktop UI, certify generated-image semantics, or validate
Linux/Windows application support. The app remains macOS-validated only.
Jobs keep read-only permissions and credential-free checkouts, use pinned action
SHAs, and report real nonzero exit codes. No hooks, secrets, deployments, or
repository settings are installed or altered.

Local failure probes verify that formatting errors, unused variables, and a
failing Rust test fail their respective groups. The aggregate step is exercised
against all 256 success/failure/skipped/cancelled combinations. Publication guard
probes use disposable temporary repositories; no actual credentials are used.

References: [GitHub workflow syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax)
and [cargo-audit](https://github.com/RustSec/rustsec/tree/main/cargo-audit).
