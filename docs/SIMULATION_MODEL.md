# Simulation model

## One session

The engine simulates independent even-money spins, such as repeatedly betting on red:

1. Start with the configured bankroll and minimum bet.
2. Cap the requested stake at the table limit unless unlimited, and at the current bankroll.
3. On a win, add the actual stake to the bankroll and reset the next requested stake to the minimum.
4. On a loss, subtract the actual stake and double the requested stake.
5. Check the stopping condition and continue if appropriate.

For example, with a minimum bet of 5, two losses followed by a win stake 5, 10 and 20, producing net profit 5 if the table limit and bankroll permit those stakes. At a table limit of 10, the third stake is 10 and the same sequence produces a net loss of 5.

If less than a full doubled stake remains, the engine bets the available bankroll. This is a capped Martingale model; exceeding the ideal doubled stake does not stop a session.

## Roulette probabilities

European roulette uses `18 / 37`; American roulette uses `18 / 38`. Each win pays 1:1 and all other outcomes lose the stake, including zero/double-zero. The engine draws a 64-bit random value and compares it with a probability threshold. It does not construct individual wheel numbers.

Spins are independent. Workers use entropy-seeded `SmallRng` instances. There is no user-facing random seed; repeated requests need not reproduce the same output. Tests inject scripted outcomes to verify rules deterministically.

For an even-money stake, the expected net return per stake is `2p - 1`: `-1/37` for European and `-2/38` for American roulette. Changing stake size does not change those probabilities.

The model excludes wheel bias, casino-specific zero rules, fees, credit, time per spin and human decisions.

## Stopping conditions and classification

Fixed/combined spin limits are explicit. Target mode uses a 100,000-spin safety cap; until-bust uses a 1,000,000-spin cap. A target reached on the last spin takes precedence over the limit.

Stop reasons are target, bust, fixed spin limit or safety cap. Financial classifications are separate:

- `is_failed`: final bankroll < starting bankroll.
- `is_bankrupt`: final bankroll < minimum bet.
- `successful_runs`: total runs minus failed runs; includes break-even outcomes and non-losing capped sessions.

A bust need not have exactly zero money. The zero-bankroll histogram bucket contains balances at or below 0.01; it is not the same as the bust counter.

## Exact aggregates and bounded samples

Counts, means, minimum/maximum final balances, minimum/maximum spins, longest streaks and peak bankroll use every completed session.

To limit memory growth, each worker keeps at most 10,000 final-balance/spin samples; reduction retains at most 50,000 globally. These samples determine medians, spin percentiles and histograms. The sample is a bounded prefix of worker output, not reservoir sampling. Scheduling and thread count can change which independent runs it contains. It may miss rare tail events.

Timeline sampling captures bounded worker prefixes, using up to 250, 600 or 1,200 trajectories per worker depending on batch size. Reduction keeps at most 2,000 trajectories globally. Up to the first 10 session IDs are also retained as example paths.

The response reports `statistics_sample_size` and `timeline_sample_size`. For larger batches, medians and distributions are estimates, even though counters and means include all sessions.

Histogram proportions are extrapolated to the total batch size. Bucket counts are rounded independently. Extreme final-balance outliers may be grouped into an overflow bucket above the sample's 99th percentile; spin histograms can group unusually long sessions above the 98th percentile.

## Timeline approximation

Trajectories keep every early spin and progressively fewer checkpoints in long sessions. At a requested chart spin, the engine uses the last recorded balance at or before that spin. After a session exits, its final balance is carried forward.

The timeline shows around 100 checkpoints. Fixed/combined modes use the configured endpoint. Target/until-bust timelines use a sample-derived endpoint to keep outliers from making the chart unreadable; they may not extend to the longest session.

Timeline means are sample means at approximate checkpoints. They can differ from the exact all-session final mean. Shaded ranges are percentiles of outcomes, not statistical confidence intervals.

## Numeric and runtime limits

Amounts use `f64`; stakes are not rounded to cents after every spin. Results can contain floating-point rounding error. CSV serialization preserves the available float precision.

Input limits are 1 billion runs, 1 million configured spins, and `1e12` per money parameter. Unknown modes/rules and non-finite inputs are rejected. These are validation bounds, not a promise that maximum-size requests will finish quickly.

Cancellation is checked between sessions and every 256 spins during a session. Progress is published in groups of 256 completed sessions and flushed at chunk boundaries. No partial summary is persisted after cancellation. Summary calculation and CSV writes after the simulation loop may already be underway when a cancellation request arrives.

Performance depends on spin count, early exits, sample capture, CPU and worker count. Benchmarks must report those parameters.
