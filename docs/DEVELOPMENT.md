# Development and release guide

## Prerequisites

- Stable Rust with Cargo, rustfmt and Clippy. `rust-toolchain.toml` declares these components.
- Node.js 24 for dependency-free JavaScript tests.
- Python 3.9+ only when creating release archives.
- On Windows, an MSVC-compatible C++ linker/build toolchain.

Download/extract or clone the repository, then work in the directory containing `Cargo.toml`. Commit `Cargo.lock`: this is an application and its dependency resolution belongs in version control.

## Run locally

~~~sh
cargo run --locked
~~~

Debug builds are convenient for development. Use `cargo run --release --locked` for performance measurements. Assets are embedded using `include_str!`, so restart/rebuild after changing HTML or JavaScript.

Environment variables are documented in the [user guide](USAGE.md). Tests use unique temporary folders and never read, clear or append to the user's history file.

## Structure

| Path | Responsibility |
| --- | --- |
| `src/main.rs` | Configuration, loopback listener and graceful Ctrl+C shutdown. |
| `src/lib.rs` | Library modules shared by the binary and tests. |
| `src/simulation.rs` | Validation, single-session rules, Rayon batches, sampled statistics. |
| `src/api.rs` | Embedded assets, direct/background requests, progress and cancellation. |
| `src/storage.rs` | CSV serialization, legacy readers, locking and normalized exports. |
| `src/test_support.rs` | Isolated temporary-directory fixtures for Rust tests. |
| `static/index.html` | English form, cards, chart containers and history controls. |
| `static/app.js` | Form submission, polling and Chart.js rendering. |
| `tests/frontend.test.cjs` | Node VM tests of the actual application script. |
| `examples/` | Request payloads for manual API use. |
| `scripts/package_release.py` | Native archive packaging and dependency license collection. |
| `.github/workflows/` | CI checks and draft release builds. |

CPU-heavy simulation runs in `tokio::task::spawn_blocking`, then uses Rayon's worker pool. Atomic counters publish progress without locking every spin. An atomic cancellation flag is checked between sessions and at spin checkpoints. Job-map locks protect metadata and completed results.

CSV access is serialized by a process-local mutex. It does not lock against other server processes. Give each server its own file. The server has no authentication or multi-user account system; it is designed for local use.

## Checks

~~~sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
node --check static/app.js
node --test tests/frontend.test.cjs
cargo build --release --locked
~~~

Use `cargo fmt` to apply Rust formatting. There is no frontend dependency installation or bundler.

Once dependencies are cached, the Rust checks can also use `--offline`. To regenerate the lockfile after intentional dependency changes, run `cargo check` without `--locked`, inspect the lockfile diff, then rerun checks.

### Test design

Single-session tests use scripted win/loss sequences rather than hoping a random batch meets a statistical threshold. They cover stake progression, stake caps, all-in losses, early target exits, final-spin precedence, safety caps and cancellation.

Aggregation tests check counters, sample sizes and histogram/timeline behavior. Storage tests cover precision, quoted fields, empty files, mixed legacy schemas and concurrent writes. Router tests send real HTTP requests through Axum without opening a network port.

UI tests execute `static/app.js` in a Node VM with a small DOM/Chart adapter. They verify mode fields, parsing, invalid submissions, error/cancel states, result rendering, history output and operation when Chart.js is unavailable. They do not test browser layout or real CDN loading.

For UI changes, also open a browser, run each mode, change chart tabs, export CSV and inspect the page at a narrow screen width.

## Changing the model

Keep the [model reference](SIMULATION_MODEL.md), input validation, form controls and [API documentation](API.md) consistent.

Test rule changes with exact spin outcomes. Preserve independent stop-reason and profit classifications. Do not describe sampled metrics as exact population metrics. If the CSV schema changes, retain readers for existing row formats and update export compatibility tests.

Public simulations use fresh entropy. Performance or comparison scripts should record the roulette type, all parameters, worker count and machine details.

## Preparing a GitHub repository

The source tree includes an MIT license, documentation, ignore rules and workflows. Local simulation CSVs, `target/` and `dist/` are ignored.

Before the first push:

1. Run the checks above.
2. Review `git status --short` and confirm that private CSV history and build outputs are absent.
3. Create an empty repository on GitHub.
4. Commit and push the source using Git or GitHub Desktop.

If using Git from the terminal and no remote is configured:

~~~sh
git add .
git commit -m "Prepare English simulator for open source"
git branch -M main
git remote add origin https://github.com/YOUR_USERNAME/YOUR_REPOSITORY.git
git push -u origin main
~~~

Replace both placeholders with your own repository information. If a remote already exists, inspect it with `git remote -v` and use the existing configuration.

## GitHub Actions

`ci.yml` runs formatting, Clippy, Rust tests, JavaScript syntax/tests and an optimized build on Windows, Linux and macOS. It triggers on pushes and pull requests and needs read-only repository permissions.

The workflows use official [checkout](https://github.com/actions/checkout), [setup-node](https://github.com/actions/setup-node), [upload-artifact](https://github.com/actions/upload-artifact) and [download-artifact](https://github.com/actions/download-artifact) actions. Rust installation uses rustup already present on GitHub-hosted runners.

## Creating a release

1. Update `Cargo.toml`'s version and `CHANGELOG.md`.
2. Run `cargo check` if needed to refresh the root package entry in `Cargo.lock`.
3. Run checks and commit the release changes.
4. Create and push a matching tag, for example:

~~~sh
git tag v0.1.0
git push origin v0.1.0
~~~

The tag-triggered `release.yml` checks that the tag matches the package version, runs tests and creates native archives for Windows x86-64, Linux x86-64 and macOS arm64. A final job attaches archives and SHA-256 sidecars to a **draft GitHub release**. Review the draft and publish it from GitHub.

The macOS archive is for Apple Silicon. Linux builds use Ubuntu 22.04 runners and require a compatible glibc-based system. Archives are not code-signed/notarized. No Linux or macOS release is produced by a local Windows build.

### Package locally

~~~sh
cargo build --release --locked
python scripts/package_release.py
~~~

The script detects the native platform and writes the archive plus a SHA-256 file to `dist/`. Packages contain the executable, launcher, README, license, guides and dependency license texts collected from Cargo's cache. It never includes local history or the build directory.

The binary embeds the page and application script. External CDN assets are loaded by the browser and are not bundled in the executable.

## Documentation links

Use relative links between repository documents so they work in a clone and in release archives. Avoid repository-specific badges until the GitHub owner/name are known.
