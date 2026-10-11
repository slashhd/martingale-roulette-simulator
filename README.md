# Martingale Roulette Simulator

A local web application for exploring Martingale roulette sessions with Monte Carlo simulations. A Rust engine runs sessions in parallel; the browser shows outcomes, sample paths, percentile timelines and saved history.

## Features

- Four stopping modes: fixed spins, profit target, combined and until bust.
- European or American even-money roulette probabilities.
- Configurable starting bankroll, minimum bet and table limit, including unlimited table limits.
- Loss and bust rates, mean and median results, survival times, streaks and peak bankrolls.
- Five chart views and history filters for each mode.
- CSV history saved locally, with download and clear controls.
- Progress and cancellation for large batches and potentially long sessions.

The [simulation model](docs/SIMULATION_MODEL.md) explains the betting rules and which statistics use bounded samples.

## Download and run

### Prebuilt releases

Once a release has been published, open this repository's **Releases** page and download the archive for your platform:

| Archive | Platform |
| --- | --- |
| `martingale-windows-x86_64.zip` | 64-bit Windows |
| `martingale-linux-x86_64.tar.gz` | 64-bit Linux |
| `martingale-macos-arm64.tar.gz` | macOS on Apple Silicon |

Extract the archive into a writable folder. On Windows, double-click `start_martingale.bat`. On Linux or macOS, open a terminal in the extracted folder and run:

~~~sh
sh start_martingale.sh
~~~

Open **http://127.0.0.1:3000** after the terminal reports the server is ready. Keep the terminal open while using the app. Press **Ctrl+C** to stop it.

Prebuilt downloads do not require Rust or Node.js. The server and CSV storage run locally. Styling, fonts and charts load from external CDNs, so the full interface requires internet access. Without Chart.js, simulation results and CSV history still work.

### Run from source

1. Install the stable Rust toolchain from [rustup.rs](https://rustup.rs). On Windows, use the MSVC toolchain and install the C++ build tools if prompted.
2. Download the repository using **Code → Download ZIP**, then extract it; or clone it with Git.
3. Open a terminal in the directory containing `Cargo.toml`.
4. Run:

~~~sh
cargo run --release --locked
~~~

Then open **http://127.0.0.1:3000**. The first build downloads dependencies and takes longer. Future builds reuse Cargo's cache. You can also use the included launcher scripts.

## First simulation

Choose **Fixed spins** and try:

- Runs: `10000`
- Starting bankroll: `1000`
- Minimum bet: `5`
- Table limit: `500`
- Spins per session: `100`
- Roulette rules: **European**

Click **Start Monte Carlo simulation**. Compare the loss rate, average final bankroll and median. Use the chart tabs to inspect the distributions and example paths.

See the [user guide](docs/USAGE.md) for all modes, interpreting results, saving history and troubleshooting.

## Documentation

- [User guide](docs/USAGE.md)
- [Simulation model and statistical limits](docs/SIMULATION_MODEL.md)
- [Development and release guide](docs/DEVELOPMENT.md)
- [HTTP API](docs/API.md)
- [Contributing](CONTRIBUTING.md)
- [Changelog](CHANGELOG.md)

## Development checks

Use stable Rust and Node.js 24 (Node is needed only for the UI tests):

~~~sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
node --check static/app.js
node --test tests/frontend.test.cjs
cargo build --release --locked
~~~

No npm install or frontend build is required. GitHub Actions runs these checks on Windows, Linux and macOS.

## License

[MIT](LICENSE). See [third-party notices](THIRD_PARTY_NOTICES.md) for runtime dependencies.
