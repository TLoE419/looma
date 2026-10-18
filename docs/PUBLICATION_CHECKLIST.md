# First publication checklist

This checklist is a plan, not evidence that GitHub settings are already active.
The owner approved public upload as `TLoE419/looma` on 2026-10-17 after reviewing
the known risks. License choice: MIT, confirmed by the owner. This approval does
not mean the unresolved dependency warnings or desktop defect are fixed.

## Before the first upload

- [ ] Review the English and Traditional Chinese README against actual behavior.
- [ ] Review MIT ownership wording and confirm rights to all submitted source/assets.
- [ ] Review candidate upload paths, secret-scan results, and dependency limitations.
- [ ] Verify `target/`, `generated-images/`, environment files, local databases,
      credentials, and the private `publication-review/` directory are excluded.
- [ ] Keep `Cargo.lock`; do not commit ignored local artifacts with force-add.
- [ ] Run format, Clippy, and default tests using Rust 1.90.0.
- [ ] Complete and record the applicable desktop smoke tests.
- [ ] Confirm the exact GitHub owner/repository, public visibility, and commit identity.
      Use a GitHub no-reply email if personal-email privacy is desired.
- [ ] Obtain the owner's final upload approval.

## Unresolved dependency decisions before public exposure

The owner chose to keep the current stable framework and retain all advisory
warnings. No advisory suppression or third-party patch is approved by this
choice. The owner subsequently approved publishing this early source prototype
with the disclosed findings; they remain unresolved and must stay visible.

- [ ] Review the full-lockfile audit: 2 unsoundness warnings and 3 unmaintained
      dependency warnings remain; strict audit fails despite zero entries in
      the tool's vulnerability category.
- [ ] Decide how to handle `paste` 1.0.15, which is in the current macOS dependency
      graph through image codecs enabled by Dioxus Desktop.
- [ ] Record the Linux GTK dependency limitation: `glib` 0.18.5 has an unsoundness
      advisory, and `proc-macro-error` 1.0.4 is unmaintained. The patched glib
      series is outside the current GTK dependency constraint.
- [ ] Record the Android dependency limitation: `rand` 0.7.3 and `fxhash` 0.2.1
      are introduced through the old HTML parser in wry. The upstream parser
      replacement is outside the wry version accepted by this stable framework.
- [ ] Re-run the full audit immediately before publication and make an explicit
      decision on each unresolved finding. Platform scope alone does not mean
      warnings for other platforms are fixed.

These results are a 2026-10-17 snapshot, not a permanent security guarantee.
Keep the warnings visible. Do not mark this dependency review as fully passed.

## Repository setup after upload approval

The owner has approved repository creation and public upload. Configure the
available repository protections before publishing source where possible.

- [ ] Push the reviewed initial snapshot to `main`.
- [ ] Enable GitHub Actions and confirm the first `CI / macOS checks` run succeeds.
      Select the actual check reported by GitHub when configuring required checks.
- [ ] Protect `main`: require pull requests and the successful macOS checks;
      prohibit force pushes and branch deletion. Apply enforcement to maintainers
      where supported. Do not require an additional approving reviewer while the
      project has only one maintainer; PR authors cannot approve their own PRs.
- [ ] Enable available secret scanning, push protection, dependency alerts, and
      private vulnerability reporting. Confirm Security → Report a vulnerability
      is available before opening the repository publicly.
- [ ] Set workflow token defaults to read-only; do not enable write tokens or
      access to secrets for untrusted fork pull requests.
- [ ] Review Dependabot updates rather than automatically merging them.
- [ ] Prefer squash merging and automatically delete merged feature branches.
- [ ] Set a concise description and accurate repository topics; add a real demo
      only after recording and checking for private content.
- [ ] If created privately, switch to public only with the owner's explicit approval.

## Before announcing a release

- [ ] Retest the central interaction from a clean checkout.
- [ ] Record a truthful short demo, with known limitations clearly documented.
- [ ] Add release notes distinguishing implemented and tested behavior from plans.
- [ ] Fix the build-time image output path before distributing standalone binaries.
- [ ] Validate each advertised platform and installer; current CI covers macOS only.
- [ ] Recheck dependency advisories and third-party distribution obligations.

Sources: [GitHub repository practices](https://docs.github.com/en/repositories/creating-and-managing-repositories/best-practices-for-repositories),
[protected branches](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches),
[private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/working-with-repository-security-advisories/configuring-private-vulnerability-reporting-for-a-repository).
