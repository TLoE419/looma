# Maintained dependency backports

Looma's maintainers own these four narrow source overrides until compatible
official releases replace them. This is an explicit maintenance obligation,
not a claim that the upstream packages are maintained again.

| Source | Change | Finding addressed |
| --- | --- | --- |
| dioxus-desktop 0.7.10 | Enable only PNG/JPEG/WebP; update both wry requirements to the compatible, tested 0.54.4 series | Remove unused EXR/AVIF → paste and old Android HTML → fxhash/rand 0.7 chains |
| glib 0.18.5 | Backport the mutable out-pointer fix from [upstream #1343](https://github.com/gtk-rs/gtk-rs-core/pull/1343) | RUSTSEC-2024-0429 |
| glib-macros 0.18.5 | Adapt [upstream #1288](https://github.com/gtk-rs/gtk-rs-core/pull/1288): return syn diagnostics rather than use proc-macro-error | RUSTSEC-2024-0370 |
| gtk3-macros 0.18.2 | Apply the same syn diagnostic approach to template derivation, including child-attribute errors | Remove the second proc-macro-error parent |

Package names and versions are unchanged. No advisory IDs, target filters or
warning suppression are used. `proc-macro-error2` was rejected because it also
has an unmaintained advisory (RUSTSEC-2026-0173).

## Provenance and licensing

`vendor/manifest.json` records each official registry archive's SHA-256 from the
original lockfile, the published VCS metadata (including upstream dirty flags),
the exact source inventory, changed files and patch digest. Registry archives,
not a possibly different Git checkout, are the reproducible baseline.
`vendor/patches/` contains the reviewable deltas. Registry bookkeeping and nested
lockfiles are omitted; upstream tests, copyright notices and licenses remain.
The Dioxus license files were copied from the recorded upstream revision.
Original `Cargo.toml.orig` files are retained for context; the patched normalized
`Cargo.toml` files are authoritative for builds.

```sh
python3 scripts/check-dependency-patches.py --verify-upstream
```

This checks active Cargo overrides and source hashes, downloads checksum-verified
official archives, applies the recorded patches in temporary folders, and
requires every reconstructed file to match the committed source.

## Required verification

RustSec matching does not cover local-path overrides in the same way as registry
packages. A zero-warning `cargo audit` result alone therefore cannot approve
these patches. The security job also requires source reconstruction; a separate
required dependency-test job exercises the real patched code on Linux.

```sh
# Python 3.11+, pinned Rust 1.90.0, GTK3 development libraries required.
python3 scripts/check-dependency-patches.py --test
```

The test runner copies the three GTK-family crates into a disposable workspace
and uses the separate committed `tests/dependency-patches/Cargo.lock`. Test-only
dependencies are not added to the application lockfile. It runs the optimized
GLib library suite, GLib macro library/integration tests (including compile-fail
diagnostics), and GTK3 macro unit regressions. No display or service credentials
are needed. Linux tests of these dependencies do not certify a Linux Looma app.

The string-iterator regression covers next/nth/last/next_back/nth_back and collection.
The original optimized consumer crashed; the real mutable-pointer fix passes.
Five upstream test-only reads of a NULL terminator were also corrected: the
terminator is in the backing allocation but beyond the logical slice length,
so the test uses its raw backing pointer instead of out-of-bounds get_unchecked.
The test failed identically in the unmodified upstream control; it is not skipped.
Macro regressions verify invalid input still produces diagnostics and valid
template input remains accepted.

## Maintenance and removal

Before changing an override, review upstream changes and these deltas, regenerate
the archive/patch inventory deliberately, and run reconstruction, dependency
tests, strict audit, quality, application tests and affected desktop checks.
Refresh the separate test lock only deliberately:
`python3 scripts/check-dependency-patches.py --update-test-lock`.

Track upstream security reports for all four sources; RustSec cannot do that
work for our local copies. Do not relabel a package version to escape a warning.
When compatible official versions contain the fixes, remove each override and
its source/inventory entry, regenerate the application lockfile and repeat the
same checks before removing its dedicated coverage. Reverting these overrides
restores the known security failures; it is not a safe warning-free rollback.
