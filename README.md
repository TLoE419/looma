# Looma · 織象

An experimental desktop image workspace for shaping ideas with AI-generated visuals.

[繁體中文](README.zh-TW.md) · [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md)

> Early prototype. The interface is currently in Traditional Chinese. This is a
> source-only project; no packaged desktop release is available yet.

## What works in the current source

- Import PNG, JPEG, and WebP images through a native file picker, including multiple files.
- Arrange image blocks, select and bring them forward, pan, zoom, and fit all images into view.
- Generate an image from text using a locally installed, authenticated Codex CLI.
- Request a one-shot text reply from the same input area.
- Keep successful generated images in separate request directories.

The long-term goal is to help software developers communicate ideas through
voice and visuals. Voice input/output, saved workspaces, image editing, image
history, and conversational memory are **not implemented**. SQLite is a planned
storage choice, not a current dependency; see [the storage decision](STORAGE.md).

## Run from source

The current development baseline is macOS with Rust 1.90.0 and Xcode Command Line
Tools. Linux and Windows have not been validated for this project. Use the
standard Rust toolchain manager, rustup; `rust-toolchain.toml` selects the version
and components. A Homebrew Rust installation can also work if it provides the
same compiler, rustfmt, and Clippy.

From the project directory:

```sh
cargo run --locked
```

Image import and arrangement do not require a Codex login. For text replies and
image generation, install Codex CLI separately, run `codex login`, and make sure
`codex` is available in the environment that launches Looma. The local CLI used
for preparation was 0.162.1; compatibility with other versions is not verified.
Your account must support the requested capabilities; model/tool access and
usage charges depend on your account. Looma does not include model credentials
or model access.

Optional: open existing images at launch:

```sh
cargo run --locked -- /absolute/path/to/image.png
```

## Data and current limitations

- Prompts are sent to Codex and its configured service. This is not an offline
  image-generation application. Never put credentials or confidential content
  into prompts or public bug reports.
- Selecting an image does **not** attach it to text or image requests. Requests
  currently contain only the text entered; each is independent.
- Looma does not save workspace layout or conversation history. Imported images
  remain references to their original files. Closing the window loses the layout.
- Generated files are retained under `generated-images/image-*/generated.png` in
  the **build-time source directory**, which must remain writable. This location
  needs to change before distributing standalone binaries.
- Text requests time out after two minutes; image requests after five minutes.
  Timeout handling requests termination of the direct CLI child, but does not
  guarantee cleanup of every descendant process.
- Calls use Codex ephemeral mode and ignore the user's main config file while
  reusing local authentication. This is not a guarantee that the external
  service keeps no data, or that no other local configuration applies.

## Known dependency advisories

The 2026-10-17 full-lockfile RustSec check reports two unsoundness warnings and
three unmaintained-dependency warnings. Strict audit does not pass. The current
macOS dependency graph includes the unmaintained `paste` crate; the other warned
paths concern Linux and Android. No advisories are suppressed. See the
[publication checklist](docs/PUBLICATION_CHECKLIST.md#unresolved-dependency-decisions-before-public-exposure)
for scope and upstream constraints. This prototype is not a security-audited release.

## Development checks

```sh
cargo fmt -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

The default tests are local. Two ignored tests make real service requests and
must be run explicitly; they are excluded from CI and may consume account quota:

```sh
cargo test --locked real_reply -- --ignored
cargo test --locked real_image -- --ignored --nocapture
```

Automated tests cover model geometry/state and generated-image validation, not
end-to-end desktop interaction. See [the manual smoke checklist](docs/SMOKE_TEST.md).

## Contribute

Small, focused changes are welcome. Read [the contribution guide](CONTRIBUTING.md)
before starting a larger feature. Planned features are directions, not delivery
commitments. No affiliation with OpenAI is implied.

## License

Looma source code is available under the [MIT License](LICENSE). Third-party
components and generated/imported content retain their own applicable terms.
