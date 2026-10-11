# User guide

## Starting and stopping

Run a release executable, a launcher script, or `cargo run --release --locked` from the source directory. Wait for the printed address, then open it in a modern browser with JavaScript enabled. Stop the server with Ctrl+C.

The server listens on `127.0.0.1`, so it is available on your own machine. It does not automatically open a browser. Opening the page loads history and waits for you to start a simulation.

## Parameters

| Parameter | Meaning |
| --- | --- |
| Runs | Number of independent sessions, from 1 to 1,000,000,000. Start with a small batch to estimate runtime. |
| Starting bankroll | Money available at the beginning of every session. |
| Minimum bet | Initial stake and the stake used after each win. |
| Table limit | Maximum stake per spin. Must be at least the minimum bet. |
| Unlimited | Removes the table limit. Bets still cannot exceed the current bankroll. |
| Spins per session | Maximum spins in fixed or combined mode, from 1 to 1,000,000. |
| Net profit target | Desired gain above the starting bankroll, used in target or combined mode. |
| Roulette rules | European: 18/37 win probability. American: 18/38 win probability. |

Money inputs must be positive and at most `1e12`. Decimal values are supported; calculation uses floating point without rounding each stake to cents. The € label is a display convention; the model works in arbitrary consistent units.

A bankroll smaller than the minimum bet is valid and produces a session with zero spins. A session does not stop when the requested doubled stake exceeds the table limit: the actual stake is capped and play continues.

## Stopping modes

| Mode | Stops when |
| --- | --- |
| Fixed spins | The configured spin limit is reached, or the bankroll falls below the minimum bet. |
| Profit target | Bankroll reaches starting bankroll + target, bankroll falls below the minimum bet, or a 100,000-spin safety cap is reached. |
| Combined | The target is reached, the configured spin limit is reached, or bankroll falls below the minimum bet. |
| Until bust | Bankroll falls below the minimum bet, or a 1,000,000-spin safety cap is reached. |

Safety-capped sessions are shown separately. They have not gone bust. The target wins when it is reached on the final permitted spin.

## Progress and cancellation

Fixed and combined batches of up to 2,000,000 runs return results when finished. Larger batches and all profit-target/until-bust simulations return a background job with progress, speed, ETA and a **Cancel simulation** button.

Progress is updated in small groups of completed sessions. It may remain at zero while a long first session is still running. ETA is an estimate from observed throughput. Cancellation is cooperative: workers check it between sessions and during long sessions. Cancelled batches do not save a partial result.

Jobs are held in memory. Restarting the server removes them. Up to 64 jobs/results are retained; older finished results can be evicted, and finished jobs older than 30 minutes are pruned when submitting another job. Saved CSV summaries remain available.

## Reading results

- **Sessions ending at a loss:** final bankroll is below starting bankroll.
- **Bust:** bankroll is below the minimum bet, even if some money remains.
- **Average final bankroll / net profit:** arithmetic mean over every completed session.
- **Median final bankroll / survival time:** calculated from a bounded sample.
- **Peak bankroll:** largest balance reached at any point in any session.
- **Streaks:** longest consecutive win or loss sequence across all sessions.
- **Target reached:** met the configured target.
- **Spin limit reached:** exited at the configured fixed/combined limit.
- **Safety cap reached:** exited at a protective cap in target/until-bust mode.

Loss and bust are different measures. A session can lose money without going bust; a session starting below the minimum bet can be classified as bust without losing money.

The chart subtitle shows the sample sizes. Histograms estimate counts from sample proportions; individually rounded bucket counts may not sum exactly to the total. Timeline shaded bands show the middle 50% and 80% of sampled outcomes. They are percentile ranges, not confidence intervals for a mean.

The [model reference](SIMULATION_MODEL.md) describes sampling and numeric limits in more detail.

## Charts

- **Bankroll distribution:** estimated final balances, with a separate zero-bankroll bucket.
- **Timeline:** sample mean, median and percentile ranges across spin checkpoints.
- **Spin distribution:** estimated number of spins played per session.
- **Sample paths:** up to 10 individual paths, with bust sessions in red.
- **History trends:** loss rate and average profit across saved batches. Check parameters before comparing different entries.

## History and CSV

Each completed simulation is appended to `martingale_simulations.csv` in the server's working directory. Histories show newest entries first and can be filtered by mode. Charts and individual trajectories are not persisted in CSV.

**CSV Export** downloads a chronological file using the current 37-column schema. The reader supports original 21-column and intermediate 27-column files, including files containing mixed row formats. Missing legacy statistics have zero/default values and should not be interpreted as newly measured values. Existing rows remain in place; the export normalizes their layout.

Unlimited limits are represented by `unlimited_max_bet=true` and numeric `max_bet=0`. Use the flag to interpret the limit. The history file and local release archives are ignored by Git.

**Clear history** asks for confirmation and removes the history file. Export a copy first if you want to keep it. Use one CSV file per running server process.

## Configuration

| Environment variable | Default | Purpose |
| --- | --- | --- |
| `MARTINGALE_PORT` | `3000` | Local HTTP port. `0` lets the operating system choose a port; use the printed URL. |
| `MARTINGALE_CSV_PATH` | `martingale_simulations.csv` | Absolute path, or path relative to the server's working directory. The parent directory must exist. |
| `RAYON_NUM_THREADS` | Rayon default | Worker thread count for simulation. |

PowerShell:

~~~powershell
$env:MARTINGALE_PORT = "3001"
$env:MARTINGALE_CSV_PATH = "C:\Data\roulette-history.csv"
cargo run --release --locked
~~~

Linux/macOS:

~~~sh
MARTINGALE_PORT=3001 MARTINGALE_CSV_PATH=./history.csv cargo run --release --locked
~~~

The launchers switch to their own directory before starting. When running the executable directly, its working directory determines relative CSV paths.

## Troubleshooting

| Symptom | What to do |
| --- | --- |
| `cargo` not found | Install Rust, then reopen the terminal; or use a prebuilt release. |
| Linker/build tools missing on Windows | Install Visual Studio Build Tools with Desktop development with C++. |
| Port already in use | Stop the other server, or set `MARTINGALE_PORT` to another port. |
| History cannot be saved | Use a writable folder and ensure the configured parent directory exists. The API reports the error. |
| Charts or styling missing | Check access to the external CDN domains and reload the page. |
| Old binary still shows old text | Rebuild after changes to `static/`; assets are embedded at compile time. |
| Very slow simulations | Reduce runs or spin limits, especially in until-bust mode. |
| Linux/macOS executable permission denied | Run `chmod +x martingale_sim`, then try again. |
| OS flags an unsigned download | Release builds are not code-signed or notarized. Building from source is another available route. |
