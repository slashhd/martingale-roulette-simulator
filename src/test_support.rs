//! Isolated test fixtures: no test reads or modifies the user's history file.
use crate::simulation::{run_monte_carlo_stream, SimulationParams, SimulationSummary};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub struct TestDirectory(pub PathBuf);
impl TestDirectory {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "martingale-test-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    pub fn csv_path(&self) -> PathBuf {
        self.0.join("history.csv")
    }
}
impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
pub fn params() -> SimulationParams {
    SimulationParams {
        runs: 8,
        min_bet: 5.0,
        max_bet: 500.0,
        unlimited_max_bet: false,
        start_capital: 100.0,
        mode: "fixed_rounds".into(),
        rounds_per_run: 3,
        target_profit: None,
        roulette_type: "european".into(),
    }
}
pub fn summary() -> SimulationSummary {
    run_monte_carlo_stream(
        &params(),
        Arc::new(AtomicU64::new(0)),
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap()
}
