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
- Enter a prompt and press 規劃圖片. Verify assumptions, block details, and the
  expandable JSON match the submitted prompt; no images should be generated.
- Submit another plan while one is pending; controls must prevent duplicate
  requests. On a failed plan, preserve the prompt and previous successful plan.
- Scroll the plan panel, expand its data, and close it. Check it remains usable
  at the minimum window size and does not start dragging the workspace.
- Generate one image; check that it appears as a new block and remains on disk.
- Press 生成這份計畫 on a validated plan; verify a separate preview for each
  generated block, progress updates, and the plan count before requests start.
- Stop during an in-flight image; it should finish or time out, then leave later
  items pending. Resume and confirm successful outputs are not regenerated.
- Fail an upstream image; independent blocks should continue, dependents should
  wait, and retry should restore the dependency chain without replacing successes.
- Check planned reference images are actually attached, and ratio/transparency
  warnings do not imply that generated pixels were repaired or visually accepted.
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

## Image planning verification — 2026-10-18

- Source: base `0dbeab379f7a1a1fae4fda650216763fdb041b09`, local feature changes
  on `feat/image-plan` (not committed). A local source-hash manifest and screenshots
  were saved with the verification artifacts, outside the repository.
- Environment: macOS 26.5, Homebrew Rust 1.90.0, Codex CLI 0.162.1.
- Passed: formatting, Clippy with warnings denied, build, and 16 local tests.
  The three external-service tests remain opt-in by default.
- Passed: explicitly ran `real_plan`; a real request returned two validated
  blocks for a park and a transparent cat, at the requested 1000 × 800 size.
- Passed: in the compiled desktop app, entered a food-ordering prompt and pressed
  規劃圖片. All request buttons became disabled while waiting. A validated
  nine-block plan appeared, with assumptions, placement, layering, text, and
  references; buttons became available again and the image count stayed zero.
- Passed: expanded the original prompt and complete JSON; keyboard focus scrolled
  the panel to the JSON view, where the original prompt and visual rules were visible.
- Not verified in this run: minimum-size native window layout and pointer-wheel
  scrolling. The desktop automation tool returned `noWindowsAvailable` for native
  resize/scroll gestures, while accessibility reads, clicks, and keyboard input worked.
- Not run: desktop failure/retry preservation, close/reopen behavior, or unrelated
  image-import and image-generation checks. Parser rejection cases were covered locally.
- No image-generation request was issued by these planning checks.

## Batch generation verification

- Source: the local `feat/image-plan` changes following the planning verification
  above. A separate source-hash manifest and real-request receipts were saved
  outside the repository; no generated images are committed.
- Passed: 20 local tests, formatting, warnings-denied Clippy, and a desktop build.
- Passed: local tests for partial failure, independent work continuing, blocked
  descendants, explicit retry, duplicate submission/completion guards, stopping
  between images, and resuming without regenerating successful outputs.
- Passed: local inspection tests for unchanged pixels, aspect/alpha warnings,
  nonzero-alpha bounds, and fully transparent output rejection. These checks do
  not establish correct text, clean cutout edges, or assembly readiness.
- Passed: explicit `real_batch` service test generated a search bar and two meal
  cards in 150.99 seconds. All three outputs were distinct retained PNGs. The
  burger request's retained `reference-1.png` matched the chicken output byte
  for byte. Paths and hashes are in the local verification receipt.
- Passed: desktop planning produced ten validated ordering blocks at 390 × 844.
  Pressing generate disabled close, replan, and duplicate generation controls.
  Stopping during the background request allowed that image to finish, left
  nine blocks pending, and restored controls. Resuming started at the header;
  the background was not generated again.
- Passed: the resumed desktop batch completed all ten blocks with zero failed
  or blocked assets. All ten preview paths resolved to distinct decodable PNGs;
  the burger's staged reference matched the chicken card byte for byte. Captured
  the completed progress panel, an actual card preview, and the accessible output
  paths in local verification artifacts. No image was automatically added to or
  assembled on the canvas.
- Not run: native minimum-window-size and pointer-wheel checks, or forced
  desktop service-failure/retry. Failure and retry transitions were tested locally.
- Automatic assembly and persistence across restarts are outside this change.
