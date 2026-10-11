//! Martingale Monte Carlo engine, local HTTP API, and CSV persistence.
pub mod api;
pub mod simulation;
pub mod storage;

#[cfg(test)]
mod test_support;
