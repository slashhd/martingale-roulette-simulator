# HTTP API

Base URL: `http://127.0.0.1:3000`, or the configured/printed local address. Requests are JSON, except CSV downloads and plain-text error responses. The API is for a local, single-user application.

## Routes

| Method | Route | Result |
| --- | --- | --- |
| GET | `/` | Embedded English HTML. |
| GET | `/app.js` | Embedded application JavaScript. |
| POST | `/api/simulate` | A summary or background-job descriptor. |
| GET | `/api/job/{id}` | Progress and, once finished, a result or error. |
| POST | `/api/job/{id}/cancel` | Request cooperative cancellation. |
| GET | `/api/history` | Flat saved summaries, newest first. |
| DELETE | `/api/history` | Remove saved CSV history. |
| GET | `/api/export_csv` | Chronological CSV attachment with the current schema. |

Both direct and asynchronous submissions return HTTP 200. Validation failures return 400; missing/wrong JSON fields generally return 422. Unknown job IDs return 404. Cancelling a finished job returns 409. Storage/task errors return 500. A full job map with no completed entries to evict returns 503.

## Submit a simulation

~~~json
{
  "runs": 10000,
  "min_bet": 5,
  "max_bet": 500,
  "unlimited_max_bet": false,
  "start_capital": 1000,
  "mode": "fixed_rounds",
  "rounds_per_run": 100,
  "target_profit": null,
  "roulette_type": "european"
}
~~~

Defaults for omitted optional fields: `mode=fixed_rounds`, `rounds_per_run=100`, `target_profit=null`, `roulette_type=european`. Other fields are required.

Allowed modes: `fixed_rounds`, `target_profit`, `combined`, `till_bust`. Allowed roulette types: `european` and `american`. All names are case-sensitive.

Runs range from 1 to 1 billion. Fixed/combined spin counts range from 1 to 1 million. Target/until-bust requests may send `rounds_per_run=0` because those modes use built-in safety caps. Positive amounts and optional profit targets must be finite and at most `1e12`. Target/combined require a positive target.

With an unlimited limit, `max_bet` must still be a finite JSON number, but the engine ignores it. Use `0` when calling the API directly. The browser supplies a finite placeholder.

From the repository root:

~~~sh
curl -H "Content-Type: application/json" --data-binary @examples/fixed-rounds.json http://127.0.0.1:3000/api/simulate
~~~

On Windows PowerShell, use `curl.exe` for these commands.

## Direct response

Fixed/combined requests with up to 2,000,000 runs return a `SimulationSummary` containing:

| Fields | Meaning |
| --- | --- |
| `id`, `timestamp`, `params`, `mode` | Identifier, timestamp including local UTC offset, and input parameters. |
| `total_runs` | Number of completed sessions. |
| `failed_*`, `bankrupt_*`, `successful_*` | Counters and percentages for financial classifications. |
| `target_reached_*`, `round_limit_*`, `hit_safety_cap_*` | Stop-reason counters and percentages. |
| `avg_rounds_played`, `median_rounds_played`, `min_rounds_played`, `max_rounds_played` | Session lengths. |
| `p10_rounds`, `p25_rounds`, `p75_rounds`, `p90_rounds` | Sampled spin-count percentiles. |
| `avg_result`, `avg_profit`, `median_result`, `median_profit` | Final balance and profit metrics. |
| `min_result`, `max_result`, `max_capital_overall` | Extremes; peak includes intermediate balances. |
| `max_consecutive_losses`, `max_consecutive_wins` | Longest streaks in any session. |
| `duration_ms` | Engine elapsed time, excluding CSV persistence. |
| `distribution`, `rounds_distribution` | Estimated histogram buckets. |
| `sample_trajectories` | Up to 10 sample sessions with capital checkpoints and exit flags. |
| `timeline_percentiles` | Sampled mean, median, p10, p25, p75, p90 and bust percentage per checkpoint. |
| `statistics_sample_size`, `timeline_sample_size` | Actual sizes of the bounded samples. |

A histogram bucket has `label`, `min_val`, `max_val`, `count` and `percentage`. A trajectory point has `round` and `capital`.

A successful result is persisted before the direct response is returned. If persistence fails, the request returns an error rather than claiming success.

## Background jobs

Large fixed/combined requests and every target/until-bust request return:

~~~json
{"job_id":"job_0","status":"running"}
~~~

Poll `GET /api/job/job_0`. Job progress contains:

~~~json
{
  "job_id": "job_0",
  "total_runs": 10000,
  "completed_runs": 512,
  "percentage": 5.12,
  "speed_runs_per_sec": 1200,
  "elapsed_ms": 426.67,
  "estimated_remaining_ms": 7906.67,
  "is_finished": false,
  "is_cancelled": false,
  "result": null,
  "error": null
}
~~~

A finished successful job includes a full summary in `result`. A finished failed job has `error`. Cooperative cancellation discards partial results; inspect the finished state if completion races with cancellation. Job IDs last only for the server process lifetime.

Request cancellation:

~~~sh
curl -X POST http://127.0.0.1:3000/api/job/job_0/cancel
~~~

Job retention is limited to 64 entries and completed jobs expire after 30 minutes when the next submission prunes the map.

## History and export

History entries use the flat `HistoricalRecord` schema corresponding to the 37 CSV columns. They contain no trajectory arrays, histograms or sample-size fields. `max_capital` in history maps to `max_capital_overall` in a live summary.

For unlimited limits, history/export use `max_bet=0` together with `unlimited_max_bet=true`. Legacy rows fill unavailable statistics with defaults.

~~~sh
curl http://127.0.0.1:3000/api/history
curl -o results.csv http://127.0.0.1:3000/api/export_csv
~~~

The export response has `Content-Type: text/csv; charset=utf-8` and an attachment filename of `martingale_simulations.csv`. An empty history still exports a header.

`DELETE /api/history` deletes the file and returns `{"status":"success","message":"History cleared"}`. It does not cancel active jobs, which may save a new record afterwards.
