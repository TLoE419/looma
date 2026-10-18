# Contributing to Looma

Looma is an early desktop prototype. English and Traditional Chinese reports are
welcome. Be respectful, address the work rather than the person, and keep private
information out of public discussions. Harassment and discriminatory conduct are
not welcome; maintainers may moderate disruptive participation.

## Start small

For a bug, include reproduction steps, expected/actual behavior, OS version, Rust
version, and Codex CLI version if relevant. Remove secrets and private paths from
logs and screenshots. See [SECURITY.md](SECURITY.md) for vulnerabilities.

Discuss substantial features or architectural changes in an issue before coding.
An issue is useful for deferred bugs and work that needs a durable specification;
it is not required for every small fix. Voice and persistence are planned, not
existing integrations that contributors can assume are available.

## Local setup and checks

Follow [README.md](README.md). Use the pinned Rust toolchain and commit
`Cargo.lock` for this desktop application. Do not regenerate dependency versions
incidentally. Format and check the change before opening a pull request:

```sh
cargo fmt -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

The ignored real-service tests require separate, deliberate execution and may
consume quota. Never provide authentication to CI or a pull request from a fork.
Do not run unreviewed contributor code with personal credentials.

For UI changes, run the relevant steps in [SMOKE_TEST.md](docs/SMOKE_TEST.md),
record the OS and tool versions, and attach a redacted screenshot or recording
when useful. Passing unit tests does not prove desktop interactions work.

## Change workflow

1. Create a short-lived branch, for example `feat/image-history` or `fix/zoom-anchor`.
2. Keep one meaningful task per branch and pull request. Avoid unrelated cleanup.
3. Describe the problem, resulting behavior, actual verification, and remaining risks.
4. Add a regression test when changing nontrivial behavior; do not add tests that
   merely repeat implementation details.
5. Wait for checks and maintainer review before merging. Maintainers manually
   exercise the affected main behavior before accepting a user-facing change.

Use English pull request titles. English documentation is the entry point;
keep the Traditional Chinese README consistent when behavior or setup changes.
Do not claim a feature or platform is verified without naming the evidence.

Never commit generated user images, private data, authentication files, or keys.
If a credential is exposed, revoke/rotate it; deleting a file is insufficient.

## Dependencies and licensing

Explain why a new dependency is needed and check its license. MIT licensing of
Looma does not relicense third-party libraries, models, or generated content.
Unless explicitly stated otherwise, contributions are provided under Looma's
MIT License. Contributors must have the right to submit their changes.
