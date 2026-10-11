use rand::rngs::SmallRng;
use rand::{RngCore, SeedableRng};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub const MAX_RUNS: u64 = 1_000_000_000;
pub const MAX_ROUNDS: usize = 1_000_000;
pub const TARGET_SAFETY_CAP: usize = 100_000;
const MAX_MONEY: f64 = 1_000_000_000_000.0;

/// One even-money betting session per run. Amounts use abstract currency units.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationParams {
    pub runs: u64,
    pub min_bet: f64,
    pub max_bet: f64,
    pub unlimited_max_bet: bool,
    pub start_capital: f64,
    #[serde(default = "default_mode")]
    pub mode: String, // "fixed_rounds", "target_profit", "combined", "till_bust"
    #[serde(default = "default_rounds")]
    pub rounds_per_run: usize,
    #[serde(default)]
    pub target_profit: Option<f64>,
    #[serde(default = "default_roulette_type")]
    pub roulette_type: String, // "european" (18/37) or "american" (18/38)
}

impl SimulationParams {
    /// Reject unknown rules and malformed limits rather than silently changing the model.
    pub fn validate(&self) -> Result<(), String> {
        if self.runs == 0 || self.runs > MAX_RUNS {
            return Err(format!("Runs must be between 1 and {MAX_RUNS}"));
        }
        for (name, value) in [
            ("Minimum bet", self.min_bet),
            ("Starting bankroll", self.start_capital),
        ] {
            if !value.is_finite() || value <= 0.0 || value > MAX_MONEY {
                return Err(format!(
                    "{name} must be finite, positive and at most {MAX_MONEY}"
                ));
            }
        }
        if !self.max_bet.is_finite()
            || (!self.unlimited_max_bet
                && (self.max_bet < self.min_bet || self.max_bet > MAX_MONEY))
        {
            return Err("Table limit must be finite and between the minimum bet and 1e12".into());
        }
        if !["fixed_rounds", "target_profit", "combined", "till_bust"].contains(&self.mode.as_str())
        {
            return Err("Unknown simulation mode".into());
        }
        if !["european", "american"].contains(&self.roulette_type.as_str()) {
            return Err("Roulette type must be european or american".into());
        }
        if self.rounds_per_run > MAX_ROUNDS
            || (matches!(self.mode.as_str(), "fixed_rounds" | "combined")
                && self.rounds_per_run == 0)
        {
            return Err(format!(
                "Spin limit must be between 1 and {MAX_ROUNDS} in fixed or combined mode"
            ));
        }
        if let Some(target) = self.target_profit {
            if !target.is_finite() || target <= 0.0 || target > MAX_MONEY {
                return Err("Profit target must be finite, positive and at most 1e12".into());
            }
        }
        if matches!(self.mode.as_str(), "target_profit" | "combined")
            && self.target_profit.is_none()
        {
            return Err("This mode requires a positive profit target".into());
        }
        Ok(())
    }
}

fn default_mode() -> String {
    "fixed_rounds".to_string()
}

fn default_rounds() -> usize {
    100
}

fn default_roulette_type() -> String {
    "european".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryPoint {
    pub round: usize,
    pub capital: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleTrajectory {
    pub id: usize,
    pub final_capital: f64,
    pub rounds_played: usize,
    pub is_failed: bool,
    pub is_bankrupt: bool,
    pub reached_target: bool,
    pub hit_safety_cap: bool,
    pub points: Vec<TrajectoryPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PercentilePoint {
    pub round: usize,
    pub mean: f64,
    pub median: f64,
    pub p10: f64,
    pub p25: f64,
    pub p75: f64,
    pub p90: f64,
    pub bankrupt_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucket {
    pub label: String,
    pub min_val: f64,
    pub max_val: f64,
    pub count: u64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationSummary {
    pub id: String,
    pub timestamp: String,
    pub params: SimulationParams,
    pub mode: String,
    pub total_runs: u64,
    pub failed_runs: u64,
    pub failed_percentage: f64,
    pub bankrupt_runs: u64,
    pub bankrupt_percentage: f64,
    pub successful_runs: u64,
    pub successful_percentage: f64,
    pub target_reached_runs: u64,
    pub target_reached_percentage: f64,
    pub round_limit_runs: u64,
    pub round_limit_percentage: f64,
    pub hit_safety_cap_runs: u64,
    pub hit_safety_cap_percentage: f64,
    pub avg_rounds_played: f64,
    pub median_rounds_played: f64,
    pub min_rounds_played: usize,
    pub max_rounds_played: usize,
    pub p10_rounds: f64,
    pub p25_rounds: f64,
    pub p75_rounds: f64,
    pub p90_rounds: f64,
    pub avg_result: f64,
    pub avg_profit: f64,
    pub median_result: f64,
    pub median_profit: f64,
    pub min_result: f64,
    pub max_result: f64,
    pub max_capital_overall: f64,
    pub max_consecutive_losses: usize,
    pub max_consecutive_wins: usize,
    pub duration_ms: f64,
    pub distribution: Vec<HistogramBucket>,
    pub rounds_distribution: Vec<HistogramBucket>,
    pub sample_trajectories: Vec<SampleTrajectory>,
    pub timeline_percentiles: Vec<PercentilePoint>,
    /// Medians and histograms use this bounded sample, not necessarily all runs.
    pub statistics_sample_size: usize,
    pub timeline_sample_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProgress {
    pub job_id: String,
    pub total_runs: u64,
    pub completed_runs: u64,
    pub percentage: f64,
    pub speed_runs_per_sec: f64,
    pub elapsed_ms: f64,
    pub estimated_remaining_ms: f64,
    pub is_finished: bool,
    pub is_cancelled: bool,
    pub result: Option<SimulationSummary>,
    pub error: Option<String>,
}

#[derive(Clone)]
struct ThreadAccumulator {
    runs: u64,
    failed_runs: u64,
    bankrupt_runs: u64,
    target_reached_runs: u64,
    round_limit_runs: u64,
    hit_safety_cap_runs: u64,
    sum_capital: f64,
    min_capital: f64,
    max_capital: f64,
    max_capital_overall: f64,
    max_loss_streak: usize,
    max_win_streak: usize,
    sum_rounds: u64,
    min_rounds: usize,
    max_rounds: usize,
    sample_capitals: Vec<f64>,
    sample_rounds: Vec<usize>,
    trajectories: Vec<SampleTrajectory>,
    sample_runs_for_timeline: Vec<SampleTrajectory>,
}

impl ThreadAccumulator {
    fn new(start_capital: f64) -> Self {
        Self {
            runs: 0,
            failed_runs: 0,
            bankrupt_runs: 0,
            target_reached_runs: 0,
            round_limit_runs: 0,
            hit_safety_cap_runs: 0,
            sum_capital: 0.0,
            min_capital: f64::MAX,
            max_capital: 0.0,
            max_capital_overall: start_capital,
            max_loss_streak: 0,
            max_win_streak: 0,
            sum_rounds: 0,
            min_rounds: usize::MAX,
            max_rounds: 0,
            sample_capitals: Vec::with_capacity(5000),
            sample_rounds: Vec::with_capacity(5000),
            trajectories: Vec::new(),
            sample_runs_for_timeline: Vec::with_capacity(200),
        }
    }

    fn merge(&mut self, mut other: ThreadAccumulator) {
        self.runs += other.runs;
        self.failed_runs += other.failed_runs;
        self.bankrupt_runs += other.bankrupt_runs;
        self.target_reached_runs += other.target_reached_runs;
        self.round_limit_runs += other.round_limit_runs;
        self.hit_safety_cap_runs += other.hit_safety_cap_runs;
        self.sum_capital += other.sum_capital;
        self.min_capital = self.min_capital.min(other.min_capital);
        self.max_capital = self.max_capital.max(other.max_capital);
        self.max_capital_overall = self.max_capital_overall.max(other.max_capital_overall);
        self.max_loss_streak = self.max_loss_streak.max(other.max_loss_streak);
        self.max_win_streak = self.max_win_streak.max(other.max_win_streak);
        self.sum_rounds += other.sum_rounds;
        self.min_rounds = self.min_rounds.min(other.min_rounds);
        self.max_rounds = self.max_rounds.max(other.max_rounds);

        if self.sample_capitals.len() < 50_000 {
            let space = 50_000 - self.sample_capitals.len();
            let to_add = other.sample_capitals.len().min(space);
            self.sample_capitals
                .extend_from_slice(&other.sample_capitals[..to_add]);
        }

        if self.sample_rounds.len() < 50_000 {
            let space = 50_000 - self.sample_rounds.len();
            let to_add = other.sample_rounds.len().min(space);
            self.sample_rounds
                .extend_from_slice(&other.sample_rounds[..to_add]);
        }

        if self.trajectories.len() < 10 {
            let space = 10 - self.trajectories.len();
            let to_add = other.trajectories.len().min(space);
            self.trajectories.extend(other.trajectories.drain(..to_add));
        }

        if self.sample_runs_for_timeline.len() < 2000 {
            let space = 2000 - self.sample_runs_for_timeline.len();
            let to_add = other.sample_runs_for_timeline.len().min(space);
            self.sample_runs_for_timeline
                .extend(other.sample_runs_for_timeline.drain(..to_add));
        }
    }
}

/// Parallel simulation with bounded result samples. Invalid or cancelled runs return None.
pub fn run_monte_carlo_stream(
    params: &SimulationParams,
    progress_counter: Arc<AtomicU64>,
    cancel_flag: Arc<AtomicBool>,
) -> Option<SimulationSummary> {
    params.validate().ok()?;
    let start_time = Instant::now();
    let total_runs = params.runs.max(1);

    let threshold: u64 = if params.roulette_type.to_lowercase() == "american" {
        ((18.0 / 38.0) * (u64::MAX as f64)) as u64
    } else {
        ((18.0 / 37.0) * (u64::MAX as f64)) as u64
    };

    let num_cpus = rayon::current_num_threads().max(1) as u64;
    let chunk_size = (total_runs / (num_cpus * 40)).clamp(25_000, 1_000_000);
    let total_chunks = total_runs.div_ceil(chunk_size);

    let start_cap = params.start_capital;

    let final_acc = (0..total_chunks)
        .into_par_iter()
        .fold(
            || (ThreadAccumulator::new(start_cap), SmallRng::from_entropy()),
            |(mut acc, mut rng), chunk_idx| {
                if cancel_flag.load(Ordering::Relaxed) {
                    return (acc, rng);
                }

                let chunk_start = chunk_idx * chunk_size;
                let chunk_end = (chunk_start + chunk_size).min(total_runs);
                let current_chunk_len = chunk_end - chunk_start;

                let mut pending_progress = 0;
                for run_idx in 0..current_chunk_len {
                    if cancel_flag.load(Ordering::Relaxed) {
                        break;
                    }
                    let global_run_id = chunk_start + run_idx;
                    let timeline_sample_limit = if total_runs <= 10_000 {
                        250
                    } else if total_runs <= 1_000_000 {
                        600
                    } else {
                        1200
                    };
                    let capture_sample_traj = global_run_id < 10 && acc.trajectories.len() < 10;
                    let capture_timeline_traj =
                        acc.sample_runs_for_timeline.len() < timeline_sample_limit;
                    let capture_trajectory = capture_sample_traj || capture_timeline_traj;
                    let capture_sample = acc.sample_capitals.len() < 10000;

                    let Some(RunOutcome {
                        final_capital: final_cap,
                        peak_capital: peak_cap,
                        rounds_played,
                        loss_streak,
                        win_streak,
                        is_failed,
                        is_bankrupt,
                        reached_target,
                        hit_round_limit,
                        hit_safety_cap,
                        trajectory: traj,
                    }) = simulate_single_run_flexible(
                        params,
                        threshold,
                        &mut rng,
                        capture_trajectory,
                        &cancel_flag,
                    )
                    else {
                        break;
                    };

                    acc.runs += 1;
                    pending_progress += 1;
                    if pending_progress == 256 {
                        progress_counter.fetch_add(pending_progress, Ordering::Relaxed);
                        pending_progress = 0;
                    }
                    if is_failed {
                        acc.failed_runs += 1;
                    }
                    if is_bankrupt {
                        acc.bankrupt_runs += 1;
                    }
                    if reached_target {
                        acc.target_reached_runs += 1;
                    }
                    if hit_round_limit {
                        acc.round_limit_runs += 1;
                    }
                    if hit_safety_cap {
                        acc.hit_safety_cap_runs += 1;
                    }

                    acc.sum_capital += final_cap;
                    if final_cap < acc.min_capital {
                        acc.min_capital = final_cap;
                    }
                    if final_cap > acc.max_capital {
                        acc.max_capital = final_cap;
                    }
                    if peak_cap > acc.max_capital_overall {
                        acc.max_capital_overall = peak_cap;
                    }
                    if loss_streak > acc.max_loss_streak {
                        acc.max_loss_streak = loss_streak;
                    }
                    if win_streak > acc.max_win_streak {
                        acc.max_win_streak = win_streak;
                    }

                    acc.sum_rounds += rounds_played as u64;
                    if rounds_played < acc.min_rounds {
                        acc.min_rounds = rounds_played;
                    }
                    if rounds_played > acc.max_rounds {
                        acc.max_rounds = rounds_played;
                    }

                    if capture_sample {
                        acc.sample_capitals.push(final_cap);
                        acc.sample_rounds.push(rounds_played);
                    }

                    if let Some(t) = traj {
                        let sample_item = SampleTrajectory {
                            id: global_run_id as usize + 1,
                            final_capital: final_cap,
                            rounds_played,
                            is_failed,
                            is_bankrupt,
                            reached_target,
                            hit_safety_cap,
                            points: t,
                        };

                        if capture_sample_traj {
                            acc.trajectories.push(sample_item.clone());
                        }
                        if capture_timeline_traj {
                            acc.sample_runs_for_timeline.push(sample_item);
                        }
                    }
                }

                progress_counter.fetch_add(pending_progress, Ordering::Relaxed);
                (acc, rng)
            },
        )
        .map(|(acc, _)| acc)
        .reduce(
            || ThreadAccumulator::new(start_cap),
            |mut a, b| {
                a.merge(b);
                a
            },
        );

    if cancel_flag.load(Ordering::Relaxed) {
        return None;
    }

    let duration = start_time.elapsed();
    let duration_ms = duration.as_secs_f64() * 1000.0;

    let actual_runs = final_acc.runs.max(1);
    let failed_runs = final_acc.failed_runs;
    let bankrupt_runs = final_acc.bankrupt_runs;
    let successful_runs = actual_runs.saturating_sub(failed_runs);

    let failed_percentage = (failed_runs as f64 / actual_runs as f64) * 100.0;
    let bankrupt_percentage = (bankrupt_runs as f64 / actual_runs as f64) * 100.0;
    let successful_percentage = (successful_runs as f64 / actual_runs as f64) * 100.0;
    let target_reached_percentage =
        (final_acc.target_reached_runs as f64 / actual_runs as f64) * 100.0;
    let round_limit_percentage = (final_acc.round_limit_runs as f64 / actual_runs as f64) * 100.0;
    let hit_safety_cap_percentage =
        (final_acc.hit_safety_cap_runs as f64 / actual_runs as f64) * 100.0;

    let avg_result = final_acc.sum_capital / actual_runs as f64;
    let avg_profit = avg_result - params.start_capital;
    let avg_rounds_played = final_acc.sum_rounds as f64 / actual_runs as f64;

    let mut samples = final_acc.sample_capitals;
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let median_result = if samples.is_empty() {
        avg_result
    } else if samples.len() % 2 == 0 {
        (samples[samples.len() / 2 - 1] + samples[samples.len() / 2]) / 2.0
    } else {
        samples[samples.len() / 2]
    };
    let median_profit = median_result - params.start_capital;

    let mut round_samples = final_acc.sample_rounds;
    round_samples.sort_unstable();
    let median_rounds_played = if round_samples.is_empty() {
        avg_rounds_played
    } else if round_samples.len() % 2 == 0 {
        (round_samples[round_samples.len() / 2 - 1] + round_samples[round_samples.len() / 2]) as f64
            / 2.0
    } else {
        round_samples[round_samples.len() / 2] as f64
    };

    // Survival Percentiles for rounds (p10, p25, p75, p90)
    let (p10_rounds, p25_rounds, p75_rounds, p90_rounds) = if !round_samples.is_empty() {
        let n = round_samples.len() as f64;
        let p10 = round_samples[(n * 0.10).min(n - 1.0) as usize] as f64;
        let p25 = round_samples[(n * 0.25).min(n - 1.0) as usize] as f64;
        let p75 = round_samples[(n * 0.75).min(n - 1.0) as usize] as f64;
        let p90 = round_samples[(n * 0.90).min(n - 1.0) as usize] as f64;
        (p10, p25, p75, p90)
    } else {
        (0.0, 0.0, 0.0, 0.0)
    };

    let min_result = if final_acc.min_capital == f64::MAX {
        0.0
    } else {
        final_acc.min_capital
    };
    let max_result = final_acc.max_capital;

    let min_rounds_played = if final_acc.min_rounds == usize::MAX {
        0
    } else {
        final_acc.min_rounds
    };
    let max_rounds_played = final_acc.max_rounds;

    let distribution = calculate_histogram_from_samples(&samples, actual_runs);
    let rounds_distribution = calculate_rounds_histogram_from_samples(&round_samples, actual_runs);

    // Both histograms extrapolate sample proportions to total runs. Their integer
    // counts can differ slightly from total_runs due to independent rounding.
    // Compute Timeline Percentiles (Mean, Median, p10, p25, p75, p90 over time)
    let timeline_percentiles = calculate_timeline_percentiles(
        &final_acc.sample_runs_for_timeline,
        params,
        median_rounds_played,
        max_rounds_played,
    );

    let now = chrono::Local::now();
    let timestamp = now.to_rfc3339();
    let id = format!("sim_{}", now.timestamp_nanos_opt().unwrap_or_default());

    let statistics_sample_size = samples.len();
    let timeline_sample_size = final_acc.sample_runs_for_timeline.len();
    Some(SimulationSummary {
        id,
        timestamp,
        params: params.clone(),
        mode: params.mode.clone(),
        total_runs: actual_runs,
        failed_runs,
        failed_percentage,
        bankrupt_runs,
        bankrupt_percentage,
        successful_runs,
        successful_percentage,
        target_reached_runs: final_acc.target_reached_runs,
        target_reached_percentage,
        round_limit_runs: final_acc.round_limit_runs,
        round_limit_percentage,
        hit_safety_cap_runs: final_acc.hit_safety_cap_runs,
        hit_safety_cap_percentage,
        avg_rounds_played,
        median_rounds_played,
        min_rounds_played,
        max_rounds_played,
        p10_rounds,
        p25_rounds,
        p75_rounds,
        p90_rounds,
        avg_result,
        avg_profit,
        median_result,
        median_profit,
        min_result,
        max_result,
        max_capital_overall: final_acc.max_capital_overall,
        max_consecutive_losses: final_acc.max_loss_streak,
        max_consecutive_wins: final_acc.max_win_streak,
        duration_ms,
        distribution,
        rounds_distribution,
        sample_trajectories: final_acc.trajectories,
        timeline_percentiles,
        statistics_sample_size,
        timeline_sample_size,
    })
}

/// Named outcomes keep stop reasons distinct from whether the session made a loss.
#[derive(Debug)]
struct RunOutcome {
    final_capital: f64,
    peak_capital: f64,
    rounds_played: usize,
    loss_streak: usize,
    win_streak: usize,
    is_failed: bool,
    is_bankrupt: bool,
    reached_target: bool,
    hit_round_limit: bool,
    hit_safety_cap: bool,
    trajectory: Option<Vec<TrajectoryPoint>>,
}

/// Losses double the requested stake; actual stakes respect bankroll and table limits.
#[inline]
fn simulate_single_run_flexible(
    params: &SimulationParams,
    threshold: u64,
    rng: &mut impl RngCore,
    capture_trajectory: bool,
    cancel: &AtomicBool,
) -> Option<RunOutcome> {
    let mut current_capital = params.start_capital;
    let mut peak_capital = params.start_capital;
    let mut current_bet = params.min_bet;

    let mut cur_loss_streak = 0usize;
    let mut max_loss_streak = 0usize;
    let mut cur_win_streak = 0usize;
    let mut max_win_streak = 0usize;

    let min_bet = params.min_bet;
    let max_bet = params.max_bet;
    let unlimited = params.unlimited_max_bet;

    let mode = params.mode.as_str();

    let max_allowed_rounds = match mode {
        "till_bust" => MAX_ROUNDS,
        "target_profit" => TARGET_SAFETY_CAP,
        "combined" | "fixed_rounds" => params.rounds_per_run.max(1),
        _ => params.rounds_per_run.max(1),
    };

    let target_threshold = match mode {
        "target_profit" | "combined" => params.target_profit.map(|tp| params.start_capital + tp),
        _ => None,
    };

    let mut trajectory = if capture_trajectory {
        let mut t = Vec::with_capacity(105);
        t.push(TrajectoryPoint {
            round: 0,
            capital: current_capital,
        });
        Some(t)
    } else {
        None
    };

    let mut rounds_played = 0usize;
    let mut reached_target = false;
    let mut hit_round_limit = false;
    let mut hit_safety_cap = false;

    while rounds_played < max_allowed_rounds {
        // Check even during long survival sessions without polling on every spin.
        if rounds_played.is_multiple_of(256) && cancel.load(Ordering::Relaxed) {
            return None;
        }
        if current_capital < min_bet {
            break;
        }

        if let Some(target) = target_threshold {
            if current_capital >= target {
                reached_target = true;
                break;
            }
        }

        rounds_played += 1;

        let mut actual_bet = current_bet;
        if !unlimited && actual_bet > max_bet {
            actual_bet = max_bet;
        }
        if actual_bet > current_capital {
            actual_bet = current_capital;
        }

        let won = rng.next_u64() < threshold;

        if won {
            current_capital += actual_bet;
            if current_capital > peak_capital {
                peak_capital = current_capital;
            }
            current_bet = min_bet;
            cur_win_streak += 1;
            if cur_win_streak > max_win_streak {
                max_win_streak = cur_win_streak;
            }
            cur_loss_streak = 0;
        } else {
            current_capital -= actual_bet;
            current_bet *= 2.0;
            cur_loss_streak += 1;
            if cur_loss_streak > max_loss_streak {
                max_loss_streak = cur_loss_streak;
            }
            cur_win_streak = 0;
        }

        if let Some(t) = &mut trajectory {
            let should_record = if rounds_played <= 100 {
                true
            } else if rounds_played <= 500 {
                rounds_played.is_multiple_of(2)
            } else if rounds_played <= 2000 {
                rounds_played.is_multiple_of(5)
            } else if rounds_played <= 10000 {
                rounds_played.is_multiple_of(20)
            } else if rounds_played <= 50000 {
                rounds_played.is_multiple_of(100)
            } else if rounds_played <= 200000 {
                rounds_played.is_multiple_of(500)
            } else {
                rounds_played.is_multiple_of(2000)
            };

            if should_record
                || current_capital < min_bet
                || (target_threshold.is_some_and(|target| current_capital >= target))
            {
                t.push(TrajectoryPoint {
                    round: rounds_played,
                    capital: current_capital,
                });
            }
        }

        if current_capital < min_bet {
            break;
        }

        if let Some(target) = target_threshold {
            if current_capital >= target {
                reached_target = true;
                break;
            }
        }
    }

    if rounds_played >= max_allowed_rounds && current_capital >= min_bet && !reached_target {
        if matches!(mode, "till_bust" | "target_profit") {
            hit_safety_cap = true;
        } else {
            hit_round_limit = true;
        }
    }

    if let Some(t) = &mut trajectory {
        if let Some(last) = t.last() {
            if last.round != rounds_played {
                t.push(TrajectoryPoint {
                    round: rounds_played,
                    capital: current_capital,
                });
            }
        }
    }

    let is_bankrupt = current_capital < min_bet;
    let is_failed = current_capital < params.start_capital;

    Some(RunOutcome {
        final_capital: current_capital,
        peak_capital,
        rounds_played,
        loss_streak: max_loss_streak,
        win_streak: max_win_streak,
        is_failed,
        is_bankrupt,
        reached_target,
        hit_round_limit,
        hit_safety_cap,
        trajectory,
    })
}

fn get_nice_step_size(target_step: f64) -> usize {
    if target_step <= 1.0 {
        return 1;
    }
    let nice_steps = [
        1, 2, 5, 10, 20, 25, 50, 100, 200, 250, 500, 1_000, 2_000, 2_500, 5_000, 10_000, 20_000,
        25_000, 50_000, 100_000,
    ];
    for &s in &nice_steps {
        if s as f64 >= target_step * 0.75 {
            return s;
        }
    }
    target_step.round() as usize
}

fn calculate_timeline_percentiles(
    sample_trajectories: &[SampleTrajectory],
    params: &SimulationParams,
    median_rounds: f64,
    max_rounds: usize,
) -> Vec<PercentilePoint> {
    if sample_trajectories.is_empty() {
        return Vec::new();
    }

    let end_round = match params.mode.as_str() {
        "fixed_rounds" | "combined" => params.rounds_per_run.max(1),
        "till_bust" | "target_profit" => {
            let mut traj_rounds: Vec<usize> = sample_trajectories
                .iter()
                .map(|s| s.rounds_played)
                .collect();
            traj_rounds.sort_unstable();
            let p90_idx = ((traj_rounds.len() as f64 * 0.90).round() as usize)
                .min(traj_rounds.len().saturating_sub(1));
            let p90 = traj_rounds.get(p90_idx).copied().unwrap_or(100);
            let candidate = (p90 as f64 * 1.3).max(median_rounds * 2.5).round() as usize;
            candidate.clamp(30, max_rounds.max(30))
        }
        _ => params.rounds_per_run.max(1),
    };

    // Calculate clean, round steps targeting ~100 checkpoints for fluid line rendering
    let target_step = (end_round as f64 / 100.0).max(1.0);
    let step_size = get_nice_step_size(target_step);

    let mut check_rounds = Vec::with_capacity(120);
    let mut current = 0usize;
    while current <= end_round {
        check_rounds.push(current);
        current += step_size;
    }
    if check_rounds.last() != Some(&end_round) {
        check_rounds.push(end_round);
    }

    let num_samples = sample_trajectories.len();
    let mut result = Vec::with_capacity(check_rounds.len());

    for &r in &check_rounds {
        let mut capitals_at_r = Vec::with_capacity(num_samples);
        let mut bankrupt_count = 0usize;

        for traj in sample_trajectories {
            let cap = get_capital_at_round(&traj.points, r, traj.final_capital, traj.rounds_played);
            if cap < params.min_bet {
                bankrupt_count += 1;
            }
            capitals_at_r.push(cap);
        }

        capitals_at_r.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let n = capitals_at_r.len();
        let sum: f64 = capitals_at_r.iter().sum();
        let mean = sum / n as f64;

        let p10 = capitals_at_r[(n as f64 * 0.10).min((n - 1) as f64) as usize];
        let p25 = capitals_at_r[(n as f64 * 0.25).min((n - 1) as f64) as usize];
        let median = if n % 2 == 0 {
            (capitals_at_r[n / 2 - 1] + capitals_at_r[n / 2]) / 2.0
        } else {
            capitals_at_r[n / 2]
        };
        let p75 = capitals_at_r[(n as f64 * 0.75).min((n - 1) as f64) as usize];
        let p90 = capitals_at_r[(n as f64 * 0.90).min((n - 1) as f64) as usize];

        let bankrupt_pct = (bankrupt_count as f64 / n as f64) * 100.0;

        result.push(PercentilePoint {
            round: r,
            mean,
            median,
            p10,
            p25,
            p75,
            p90,
            bankrupt_pct,
        });
    }

    result
}

fn get_capital_at_round(
    points: &[TrajectoryPoint],
    target_round: usize,
    final_capital: f64,
    rounds_played: usize,
) -> f64 {
    if points.is_empty() {
        return final_capital;
    }

    if target_round >= rounds_played {
        return final_capital;
    }

    if target_round == 0 {
        return points[0].capital;
    }

    // Binary search for closest point <= target_round
    let idx = points.partition_point(|p| p.round <= target_round);
    if idx == 0 {
        points[0].capital
    } else {
        points[idx - 1].capital
    }
}

fn calculate_histogram_from_samples(samples: &[f64], total_runs: u64) -> Vec<HistogramBucket> {
    if samples.is_empty() {
        return Vec::new();
    }

    let mut bankrupt_samples = 0u64;
    let mut active_samples: Vec<f64> = Vec::with_capacity(samples.len());

    for &s in samples {
        if s <= 0.01 {
            bankrupt_samples += 1;
        } else {
            active_samples.push(s);
        }
    }

    let total_sample_count = samples.len() as f64;
    let mut buckets = Vec::new();

    // Isolated prominent bucket for 0 € (Zero bankroll)
    if bankrupt_samples > 0 {
        let pct = (bankrupt_samples as f64 / total_sample_count) * 100.0;
        let count = ((pct / 100.0) * total_runs as f64).round() as u64;
        buckets.push(HistogramBucket {
            label: "0 € (Zero bankroll)".to_string(),
            min_val: 0.0,
            max_val: 0.0,
            count,
            percentage: pct,
        });
    }

    if active_samples.is_empty() {
        return buckets;
    }

    active_samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // Distribution across actual active capital range
    let n_act = active_samples.len();
    let act_min = active_samples[0];
    let p99_idx = ((n_act as f64 * 0.99).round() as usize).min(n_act - 1);
    let act_p99 = active_samples[p99_idx];

    let act_upper = if act_p99 > act_min + 1.0 {
        act_p99
    } else {
        active_samples[n_act - 1]
    };

    let num_bins = 15;
    if (act_upper - act_min).abs() < 1e-2 {
        let pct = (n_act as f64 / total_sample_count) * 100.0;
        let count = ((pct / 100.0) * total_runs as f64).round() as u64;
        buckets.push(HistogramBucket {
            label: format!("{:.0} €", act_min),
            min_val: act_min,
            max_val: act_upper,
            count,
            percentage: pct,
        });
        return buckets;
    }

    let bin_width = (act_upper - act_min) / num_bins as f64;
    let mut bin_counts = vec![0u64; num_bins];
    let mut overflow_count = 0u64;

    for &val in &active_samples {
        if val > act_upper {
            overflow_count += 1;
        } else {
            let mut idx = ((val - act_min) / bin_width).floor() as usize;
            if idx >= num_bins {
                idx = num_bins - 1;
            }
            bin_counts[idx] += 1;
        }
    }

    for (i, &s_count) in bin_counts.iter().enumerate() {
        let b_min = act_min + i as f64 * bin_width;
        let b_max = act_min + (i + 1) as f64 * bin_width;
        let pct = (s_count as f64 / total_sample_count) * 100.0;
        let est_count = ((pct / 100.0) * total_runs as f64).round() as u64;

        buckets.push(HistogramBucket {
            label: format!("{:.0} – {:.0} €", b_min, b_max),
            min_val: b_min,
            max_val: b_max,
            count: est_count,
            percentage: pct,
        });
    }

    if overflow_count > 0 {
        let pct = (overflow_count as f64 / total_sample_count) * 100.0;
        let est_count = ((pct / 100.0) * total_runs as f64).round() as u64;
        buckets.push(HistogramBucket {
            label: format!("> {:.0} €", act_upper),
            min_val: act_upper,
            max_val: active_samples[n_act - 1],
            count: est_count,
            percentage: pct,
        });
    }

    buckets
}

fn calculate_rounds_histogram_from_samples(
    round_samples: &[usize],
    total_runs: u64,
) -> Vec<HistogramBucket> {
    if round_samples.is_empty() {
        return Vec::new();
    }

    let mut sorted = round_samples.to_vec();
    sorted.sort_unstable();

    let n = sorted.len();
    let min_r = sorted[0] as f64;
    let max_r = sorted[n - 1] as f64;

    // Use 98th percentile as upper limit for bins to prevent extreme survival outliers
    // from squishing the entire distribution into a single bin
    let p98_idx = ((n as f64 * 0.98).round() as usize).min(n - 1);
    let p98_r = sorted[p98_idx] as f64;

    let upper_r = if p98_r > min_r + 5.0 && max_r > p98_r * 2.0 {
        p98_r
    } else {
        max_r
    };

    let num_bins = 16;
    if (upper_r - min_r).abs() < 1.0 {
        return vec![HistogramBucket {
            label: format!("{:.0} spins", min_r),
            min_val: min_r,
            max_val: upper_r,
            count: total_runs,
            percentage: 100.0,
        }];
    }

    let bin_width = ((upper_r - min_r) / num_bins as f64).max(1.0);
    let mut bin_counts = vec![0u64; num_bins];
    let mut overflow_count = 0u64;

    for &r in &sorted {
        let rf = r as f64;
        if rf > upper_r {
            overflow_count += 1;
        } else {
            let mut idx = ((rf - min_r) / bin_width).floor() as usize;
            if idx >= num_bins {
                idx = num_bins - 1;
            }
            bin_counts[idx] += 1;
        }
    }

    let sample_total = n as f64;
    let mut buckets = Vec::with_capacity(num_bins + 1);

    for (i, &s_count) in bin_counts.iter().enumerate() {
        let b_min = min_r + i as f64 * bin_width;
        let b_max = min_r + (i + 1) as f64 * bin_width;
        let pct = (s_count as f64 / sample_total) * 100.0;
        let est_count = ((pct / 100.0) * total_runs as f64).round() as u64;

        buckets.push(HistogramBucket {
            label: format!("{:.0} – {:.0} spins", b_min, b_max),
            min_val: b_min,
            max_val: b_max,
            count: est_count,
            percentage: pct,
        });
    }

    if overflow_count > 0 {
        let pct = (overflow_count as f64 / sample_total) * 100.0;
        let est_count = ((pct / 100.0) * total_runs as f64).round() as u64;
        buckets.push(HistogramBucket {
            label: format!("> {:.0} spins (Long sessions)", upper_r),
            min_val: upper_r,
            max_val: max_r,
            count: est_count,
            percentage: pct,
        });
    }

    buckets
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::params;
    use std::collections::VecDeque;

    /// Forces exact spin outcomes, avoiding statistical assertions that can fail at random.
    struct ScriptedRng {
        values: VecDeque<u64>,
        fallback: u64,
    }
    impl ScriptedRng {
        fn new(wins: &[bool]) -> Self {
            Self {
                values: wins
                    .iter()
                    .map(|&win| if win { 0 } else { u64::MAX })
                    .collect(),
                fallback: u64::MAX,
            }
        }
    }
    impl RngCore for ScriptedRng {
        fn next_u64(&mut self) -> u64 {
            self.values.pop_front().unwrap_or(self.fallback)
        }
        fn next_u32(&mut self) -> u32 {
            self.next_u64() as u32
        }
        fn fill_bytes(&mut self, dest: &mut [u8]) {
            for chunk in dest.chunks_mut(8) {
                chunk.copy_from_slice(&self.next_u64().to_le_bytes()[..chunk.len()]);
            }
        }
        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
            self.fill_bytes(dest);
            Ok(())
        }
    }
    fn play(p: &SimulationParams, wins: &[bool]) -> RunOutcome {
        simulate_single_run_flexible(
            p,
            u64::MAX / 2,
            &mut ScriptedRng::new(wins),
            true,
            &AtomicBool::new(false),
        )
        .unwrap()
    }

    #[test]
    fn losses_double_the_stake_and_a_win_resets_it() {
        let mut p = params();
        p.rounds_per_run = 4;
        let result = play(&p, &[false, false, true, true]);
        let capitals: Vec<_> = result
            .trajectory
            .unwrap()
            .iter()
            .map(|point| point.capital)
            .collect();
        assert_eq!(capitals, [100.0, 95.0, 85.0, 105.0, 110.0]);
        assert_eq!(result.loss_streak, 2);
        assert_eq!(result.win_streak, 2);
        assert!(result.hit_round_limit);
        assert!(!result.is_failed);
    }
    #[test]
    fn table_limit_caps_stakes_instead_of_stopping_the_session() {
        let mut p = params();
        p.max_bet = 10.0;
        let result = play(&p, &[false, false, true]);
        assert_eq!(result.final_capital, 95.0);
        assert!(result.is_failed);
        assert!(!result.is_bankrupt);
    }
    #[test]
    fn unlimited_stakes_still_respect_available_bankroll() {
        let mut p = params();
        p.unlimited_max_bet = true;
        p.start_capital = 12.0;
        let result = play(&p, &[false, false]);
        assert_eq!(result.final_capital, 0.0);
        assert_eq!(result.rounds_played, 2);
        assert!(result.is_bankrupt);
        assert!(!result.hit_round_limit);
    }
    #[test]
    fn bankroll_below_minimum_bet_stops_without_a_spin() {
        let mut p = params();
        p.start_capital = 4.0;
        let result = play(&p, &[]);
        assert!(result.is_bankrupt);
        assert!(!result.is_failed);
        assert_eq!(result.rounds_played, 0);
        assert_eq!(result.final_capital, 4.0);
    }
    #[test]
    fn target_reached_on_final_spin_takes_precedence_over_round_limit() {
        let mut p = params();
        p.mode = "combined".into();
        p.target_profit = Some(15.0);
        let result = play(&p, &[true, true, true]);
        assert!(result.reached_target);
        assert!(!result.hit_round_limit);
        assert_eq!(result.final_capital, 115.0);
    }
    #[test]
    fn fixed_spins_ignore_an_unused_target() {
        let mut p = params();
        p.target_profit = Some(5.0);
        let result = play(&p, &[true, true, true]);
        assert_eq!(result.rounds_played, 3);
        assert!(!result.reached_target);
    }
    #[test]
    fn combined_mode_stops_at_round_limit_if_target_is_not_reached() {
        let mut p = params();
        p.mode = "combined".into();
        p.target_profit = Some(50.0);
        let result = play(&p, &[true, true, true]);
        assert!(result.hit_round_limit);
        assert!(!result.reached_target);
    }
    #[test]
    fn target_mode_stops_early_and_has_an_explicit_safety_cap() {
        let mut p = params();
        p.mode = "target_profit".into();
        p.target_profit = Some(5.0);
        assert_eq!(play(&p, &[true]).rounds_played, 1);
        p.target_profit = Some(1e9);
        let mut rng = ScriptedRng {
            values: VecDeque::new(),
            fallback: 0,
        };
        let result = simulate_single_run_flexible(
            &p,
            u64::MAX / 2,
            &mut rng,
            false,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(result.rounds_played, TARGET_SAFETY_CAP);
        assert!(result.hit_safety_cap);
        assert!(!result.hit_round_limit);
    }
    #[test]
    fn until_bust_safety_cap_is_not_a_bankruptcy() {
        let mut p = params();
        p.mode = "till_bust".into();
        let mut rng = ScriptedRng {
            values: VecDeque::new(),
            fallback: 0,
        };
        let result = simulate_single_run_flexible(
            &p,
            u64::MAX / 2,
            &mut rng,
            false,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(result.rounds_played, MAX_ROUNDS);
        assert!(result.hit_safety_cap);
        assert!(!result.is_bankrupt);
    }
    #[test]
    fn invalid_parameters_are_rejected() {
        let mut cases = Vec::new();
        let mut p = params();
        p.runs = 0;
        cases.push(p);
        let mut p = params();
        p.runs = MAX_RUNS + 1;
        cases.push(p);
        let mut p = params();
        p.min_bet = f64::NAN;
        cases.push(p);
        let mut p = params();
        p.start_capital = f64::INFINITY;
        cases.push(p);
        let mut p = params();
        p.min_bet = -1.0;
        cases.push(p);
        let mut p = params();
        p.max_bet = 1.0;
        cases.push(p);
        let mut p = params();
        p.mode = "unknown".into();
        cases.push(p);
        let mut p = params();
        p.roulette_type = "unknown".into();
        cases.push(p);
        let mut p = params();
        p.rounds_per_run = 0;
        cases.push(p);
        let mut p = params();
        p.rounds_per_run = MAX_ROUNDS + 1;
        cases.push(p);
        let mut p = params();
        p.mode = "target_profit".into();
        cases.push(p);
        let mut p = params();
        p.target_profit = Some(-5.0);
        cases.push(p);
        let mut p = params();
        p.target_profit = Some(f64::NAN);
        cases.push(p);
        for p in cases {
            assert!(p.validate().is_err(), "{p:?}");
        }
        assert!(params().validate().is_ok());
    }
    #[test]
    fn json_defaults_preserve_fixed_mode_and_european_rules() {
        let p: SimulationParams = serde_json::from_str(
            r#"{"runs":1,"min_bet":5,"max_bet":500,"unlimited_max_bet":false,"start_capital":100}"#,
        )
        .unwrap();
        assert_eq!(p.mode, "fixed_rounds");
        assert_eq!(p.rounds_per_run, 100);
        assert_eq!(p.roulette_type, "european");
        assert!(p.validate().is_ok());
    }
    #[test]
    fn cancelled_batch_has_no_summary_or_completed_runs() {
        let counter = Arc::new(AtomicU64::new(0));
        assert!(run_monte_carlo_stream(
            &params(),
            counter.clone(),
            Arc::new(AtomicBool::new(true))
        )
        .is_none());
        assert_eq!(counter.load(Ordering::Relaxed), 0);
    }
    #[test]
    fn long_sessions_observe_cancellation_between_checkpoints() {
        struct CancellingRng {
            calls: usize,
            cancel: Arc<AtomicBool>,
        }
        impl RngCore for CancellingRng {
            fn next_u64(&mut self) -> u64 {
                self.calls += 1;
                if self.calls == 10 {
                    self.cancel.store(true, Ordering::Relaxed);
                }
                0
            }
            fn next_u32(&mut self) -> u32 {
                self.next_u64() as u32
            }
            fn fill_bytes(&mut self, _: &mut [u8]) {
                unreachable!()
            }
            fn try_fill_bytes(&mut self, _: &mut [u8]) -> Result<(), rand::Error> {
                unreachable!()
            }
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let mut rng = CancellingRng {
            calls: 0,
            cancel: cancel.clone(),
        };
        let mut p = params();
        p.mode = "till_bust".into();
        assert!(simulate_single_run_flexible(&p, u64::MAX / 2, &mut rng, false, &cancel).is_none());
        assert!(rng.calls <= 256);
    }
    #[test]
    fn batch_aggregates_all_runs_and_reports_sample_sizes() {
        let mut p = params();
        p.runs = 123;
        p.start_capital = 4.0;
        let progress = Arc::new(AtomicU64::new(0));
        let s =
            run_monte_carlo_stream(&p, progress.clone(), Arc::new(AtomicBool::new(false))).unwrap();
        assert_eq!(progress.load(Ordering::Relaxed), 123);
        assert_eq!(s.total_runs, 123);
        assert_eq!(s.bankrupt_runs, 123);
        assert_eq!(s.avg_result, 4.0);
        assert_eq!(s.failed_runs, 0);
        assert_eq!(s.statistics_sample_size, 123);
        assert_eq!(s.timeline_sample_size, 123);
        assert_eq!(s.sample_trajectories.len(), 10);
        let mut ids: Vec<_> = s.sample_trajectories.iter().map(|t| t.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 10);
    }
    #[test]
    fn histogram_covers_zero_uniform_and_outlier_samples() {
        assert!(calculate_histogram_from_samples(&[], 100).is_empty());
        let samples = [0.0, 0.0, 5.0, 10.0, 20.0];
        let bins = calculate_histogram_from_samples(&samples, 100);
        assert_eq!(bins[0].count, 40);
        assert!((bins.iter().map(|b| b.percentage).sum::<f64>() - 100.0).abs() < 1e-8);
        assert_eq!(
            calculate_histogram_from_samples(&[7.0; 4], 100)[0].count,
            100
        );
        let mut samples: Vec<f64> = (0..999).map(|i| 10.0 + (i % 100) as f64).collect();
        samples.push(100000.0);
        assert!(calculate_histogram_from_samples(&samples, 1000)
            .last()
            .unwrap()
            .label
            .starts_with('>'));
    }
    #[test]
    fn round_histogram_covers_uniform_samples() {
        assert!(calculate_rounds_histogram_from_samples(&[], 100).is_empty());
        let bins = calculate_rounds_histogram_from_samples(&[3; 4], 100);
        assert_eq!(bins.len(), 1);
        assert_eq!(bins[0].count, 100);
    }
    #[test]
    fn timeline_holds_bankroll_at_previous_checkpoint_and_after_exit() {
        let points = vec![
            TrajectoryPoint {
                round: 0,
                capital: 100.0,
            },
            TrajectoryPoint {
                round: 10,
                capital: 110.0,
            },
        ];
        assert_eq!(get_capital_at_round(&points, 5, 120.0, 20), 100.0);
        assert_eq!(get_capital_at_round(&points, 15, 120.0, 20), 110.0);
        assert_eq!(get_capital_at_round(&points, 50, 120.0, 20), 120.0);
    }
}
