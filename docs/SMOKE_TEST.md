# Desktop smoke test

Run on macOS from the source checkout. Record the date, OS, Rust version, Codex
CLI version, and exact source snapshot. Mark each item pass, fail, or not run;
automated model tests do not complete this checklist.

## Local interaction (no service requests)

- Launch with `cargo run --locked`; check the empty workspace and readable controls.
- Import one PNG, JPEG, and WebP, then import multiple files together.
- Cancel the file picker; the existing workspace should remain usable.
- Select overlapping blocks; the selected block should move to the front.
- Drag a block at different zoom levels and pan the background.
- Zoom in/out and fit all, including blocks far outside the original view.
- Resize the window to the minimum size and verify controls remain usable.
- Open an image through `cargo run --locked -- /absolute/path/to/image.png`.
- Close and reopen; confirm the documented absence of saved layout/history.

## Optional real-service checks (credentials/network/quota required)

Only run deliberately with non-sensitive prompts. Do not automate these in CI.

- Ask for one text reply; verify that the selected image was not submitted.
- Generate one image; check that it appears as a new block and remains on disk.
- Confirm pan/zoom remains usable while generation is pending, and duplicate
  submissions are disabled.
- With an unavailable Codex executable or failed request, confirm a useful error
  and preservation of existing blocks. Restore the original environment afterward.

## Evidence record

- Date / source snapshot:
- OS / Rust / Codex CLI:
- Steps completed and results:
- Steps not run and why:
- Redacted screenshots / recordings:
- Remaining failures:
