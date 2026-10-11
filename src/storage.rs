use crate::simulation::SimulationSummary;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::sync::Mutex;

const CSV_FILE_PATH: &str = "martingale_simulations.csv";
const CSV_HEADER: &str = "id,timestamp,mode,runs,min_bet,max_bet,unlimited_max_bet,start_capital,rounds_per_run,target_profit,roulette_type,failed_runs,failed_percentage,bankrupt_runs,bankrupt_percentage,target_reached_runs,target_reached_percentage,round_limit_runs,round_limit_percentage,hit_safety_cap_runs,hit_safety_cap_percentage,avg_rounds_played,median_rounds_played,min_rounds_played,max_rounds_played,p10_rounds,p25_rounds,p75_rounds,p90_rounds,avg_result,avg_profit,median_result,median_profit,max_capital,max_consecutive_losses,max_consecutive_wins,duration_ms";

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Flat CSV summary. Unlimited table limits use max_bet = 0 and an explicit flag.
pub struct HistoricalRecord {
    pub id: String,
    pub timestamp: String,
    pub mode: String,
    pub runs: u64,
    pub min_bet: f64,
    pub max_bet: f64,
    pub unlimited_max_bet: bool,
    pub start_capital: f64,
    pub rounds_per_run: usize,
    pub target_profit: Option<f64>,
    pub roulette_type: String,
    pub failed_runs: u64,
    pub failed_percentage: f64,
    pub bankrupt_runs: u64,
    pub bankrupt_percentage: f64,
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
    pub max_capital: f64,
    pub max_consecutive_losses: usize,
    pub max_consecutive_wins: usize,
    pub duration_ms: f64,
}

impl From<&SimulationSummary> for HistoricalRecord {
    fn from(s: &SimulationSummary) -> Self {
        Self {
            id: s.id.clone(),
            timestamp: s.timestamp.clone(),
            mode: s.mode.clone(),
            runs: s.total_runs,
            min_bet: s.params.min_bet,
            max_bet: if s.params.unlimited_max_bet {
                0.0
            } else {
                s.params.max_bet
            },
            unlimited_max_bet: s.params.unlimited_max_bet,
            start_capital: s.params.start_capital,
            rounds_per_run: s.params.rounds_per_run,
            target_profit: s.params.target_profit,
            roulette_type: s.params.roulette_type.clone(),
            failed_runs: s.failed_runs,
            failed_percentage: s.failed_percentage,
            bankrupt_runs: s.bankrupt_runs,
            bankrupt_percentage: s.bankrupt_percentage,
            target_reached_runs: s.target_reached_runs,
            target_reached_percentage: s.target_reached_percentage,
            round_limit_runs: s.round_limit_runs,
            round_limit_percentage: s.round_limit_percentage,
            hit_safety_cap_runs: s.hit_safety_cap_runs,
            hit_safety_cap_percentage: s.hit_safety_cap_percentage,
            avg_rounds_played: s.avg_rounds_played,
            median_rounds_played: s.median_rounds_played,
            min_rounds_played: s.min_rounds_played,
            max_rounds_played: s.max_rounds_played,
            p10_rounds: s.p10_rounds,
            p25_rounds: s.p25_rounds,
            p75_rounds: s.p75_rounds,
            p90_rounds: s.p90_rounds,
            avg_result: s.avg_result,
            avg_profit: s.avg_profit,
            median_result: s.median_result,
            median_profit: s.median_profit,
            max_capital: s.max_capital_overall,
            max_consecutive_losses: s.max_consecutive_losses,
            max_consecutive_wins: s.max_consecutive_wins,
            duration_ms: s.duration_ms,
        }
    }
}

/// Serializes file access within one process. Use a separate CSV for each server.
pub struct StorageManager {
    path: PathBuf,
    file_lock: Mutex<()>,
}

impl StorageManager {
    pub fn new() -> Self {
        Self::with_path(CSV_FILE_PATH)
    }

    /// Use an explicit path to isolate history, including in tests.
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            file_lock: Mutex::new(()),
        }
    }

    pub fn save_record(&self, summary: &SimulationSummary) -> Result<HistoricalRecord, String> {
        let _guard = self.file_lock.lock().map_err(|e| e.to_string())?;
        let record = HistoricalRecord::from(summary);

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| format!("Failed to open CSV file: {e}"))?;
        let empty = file.metadata().map_err(|e| e.to_string())?.len() == 0;
        let mut file = file;
        if empty {
            writeln!(file, "{CSV_HEADER}").map_err(|e| e.to_string())?;
        }
        // csv::Writer quotes fields and preserves precision. Manual formatting
        // used to round money and could corrupt arbitrary text fields.
        let mut writer = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(file);
        writer
            .serialize(&record)
            .map_err(|e| format!("Failed to write CSV row: {e}"))?;
        writer
            .flush()
            .map_err(|e| format!("Failed to flush CSV: {e}"))?;
        Ok(record)
    }

    pub fn get_all_records(&self) -> Result<Vec<HistoricalRecord>, String> {
        let _guard = self.file_lock.lock().map_err(|e| e.to_string())?;
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.path).map_err(|e| format!("Failed to read CSV: {}", e))?;
        let reader = BufReader::new(file);
        let mut csv_reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_reader(reader);

        let mut records = Vec::new();

        for result in csv_reader.records() {
            match result {
                Ok(row) => {
                    if ![21, 27, 37].contains(&row.len()) {
                        return Err(format!("Unsupported CSV row with {} columns", row.len()));
                    }

                    // Handle 37-column current format
                    if row.len() >= 37 {
                        let unlimited_max_bet = row
                            .get(6)
                            .unwrap_or("false")
                            .parse::<bool>()
                            .unwrap_or(false);
                        let max_bet = if unlimited_max_bet {
                            0.0
                        } else {
                            row.get(5).unwrap_or("0").parse::<f64>().unwrap_or(0.0)
                        };

                        let target_profit_str = row.get(9).unwrap_or("");
                        let target_profit = if target_profit_str.is_empty() {
                            None
                        } else {
                            target_profit_str.parse::<f64>().ok()
                        };

                        let record = HistoricalRecord {
                            id: row.get(0).unwrap_or("").to_string(),
                            timestamp: row.get(1).unwrap_or("").to_string(),
                            mode: row.get(2).unwrap_or("fixed_rounds").to_string(),
                            runs: row.get(3).unwrap_or("0").parse().unwrap_or(0),
                            min_bet: row.get(4).unwrap_or("0").parse().unwrap_or(0.0),
                            max_bet,
                            unlimited_max_bet,
                            start_capital: row.get(7).unwrap_or("0").parse().unwrap_or(0.0),
                            rounds_per_run: row.get(8).unwrap_or("0").parse().unwrap_or(0),
                            target_profit,
                            roulette_type: row.get(10).unwrap_or("european").to_string(),
                            failed_runs: row.get(11).unwrap_or("0").parse().unwrap_or(0),
                            failed_percentage: row.get(12).unwrap_or("0").parse().unwrap_or(0.0),
                            bankrupt_runs: row.get(13).unwrap_or("0").parse().unwrap_or(0),
                            bankrupt_percentage: row.get(14).unwrap_or("0").parse().unwrap_or(0.0),
                            target_reached_runs: row.get(15).unwrap_or("0").parse().unwrap_or(0),
                            target_reached_percentage: row
                                .get(16)
                                .unwrap_or("0")
                                .parse()
                                .unwrap_or(0.0),
                            round_limit_runs: row.get(17).unwrap_or("0").parse().unwrap_or(0),
                            round_limit_percentage: row
                                .get(18)
                                .unwrap_or("0")
                                .parse()
                                .unwrap_or(0.0),
                            hit_safety_cap_runs: row.get(19).unwrap_or("0").parse().unwrap_or(0),
                            hit_safety_cap_percentage: row
                                .get(20)
                                .unwrap_or("0")
                                .parse()
                                .unwrap_or(0.0),
                            avg_rounds_played: row.get(21).unwrap_or("0").parse().unwrap_or(0.0),
                            median_rounds_played: row.get(22).unwrap_or("0").parse().unwrap_or(0.0),
                            min_rounds_played: row.get(23).unwrap_or("0").parse().unwrap_or(0),
                            max_rounds_played: row.get(24).unwrap_or("0").parse().unwrap_or(0),
                            p10_rounds: row.get(25).unwrap_or("0").parse().unwrap_or(0.0),
                            p25_rounds: row.get(26).unwrap_or("0").parse().unwrap_or(0.0),
                            p75_rounds: row.get(27).unwrap_or("0").parse().unwrap_or(0.0),
                            p90_rounds: row.get(28).unwrap_or("0").parse().unwrap_or(0.0),
                            avg_result: row.get(29).unwrap_or("0").parse().unwrap_or(0.0),
                            avg_profit: row.get(30).unwrap_or("0").parse().unwrap_or(0.0),
                            median_result: row.get(31).unwrap_or("0").parse().unwrap_or(0.0),
                            median_profit: row.get(32).unwrap_or("0").parse().unwrap_or(0.0),
                            max_capital: row.get(33).unwrap_or("0").parse().unwrap_or(0.0),
                            max_consecutive_losses: row.get(34).unwrap_or("0").parse().unwrap_or(0),
                            max_consecutive_wins: row.get(35).unwrap_or("0").parse().unwrap_or(0),
                            duration_ms: row.get(36).unwrap_or("0").parse().unwrap_or(0.0),
                        };
                        records.push(record);
                    } else if row.len() >= 27 {
                        // 27-column intermediate format
                        let unlimited_max_bet = row
                            .get(5)
                            .unwrap_or("false")
                            .parse::<bool>()
                            .unwrap_or(false);
                        let max_bet = if unlimited_max_bet {
                            0.0
                        } else {
                            row.get(4).unwrap_or("0").parse::<f64>().unwrap_or(0.0)
                        };

                        let target_profit_str = row.get(9).unwrap_or("");
                        let target_profit = if target_profit_str.is_empty() {
                            None
                        } else {
                            target_profit_str.parse::<f64>().ok()
                        };

                        let record = HistoricalRecord {
                            id: row.get(0).unwrap_or("").to_string(),
                            timestamp: row.get(1).unwrap_or("").to_string(),
                            mode: row.get(7).unwrap_or("fixed_rounds").to_string(),
                            runs: row.get(2).unwrap_or("0").parse().unwrap_or(0),
                            min_bet: row.get(3).unwrap_or("0").parse().unwrap_or(0.0),
                            max_bet,
                            unlimited_max_bet,
                            start_capital: row.get(6).unwrap_or("0").parse().unwrap_or(0.0),
                            rounds_per_run: row.get(8).unwrap_or("0").parse().unwrap_or(0),
                            target_profit,
                            roulette_type: row.get(10).unwrap_or("european").to_string(),
                            failed_runs: row.get(11).unwrap_or("0").parse().unwrap_or(0),
                            failed_percentage: row.get(12).unwrap_or("0").parse().unwrap_or(0.0),
                            bankrupt_runs: row.get(13).unwrap_or("0").parse().unwrap_or(0),
                            bankrupt_percentage: row.get(14).unwrap_or("0").parse().unwrap_or(0.0),
                            target_reached_runs: row.get(15).unwrap_or("0").parse().unwrap_or(0),
                            target_reached_percentage: row
                                .get(16)
                                .unwrap_or("0")
                                .parse()
                                .unwrap_or(0.0),
                            round_limit_runs: 0,
                            round_limit_percentage: 0.0,
                            hit_safety_cap_runs: 0,
                            hit_safety_cap_percentage: 0.0,
                            avg_rounds_played: row.get(17).unwrap_or("0").parse().unwrap_or(0.0),
                            median_rounds_played: row.get(18).unwrap_or("0").parse().unwrap_or(0.0),
                            min_rounds_played: 0,
                            max_rounds_played: 0,
                            p10_rounds: 0.0,
                            p25_rounds: 0.0,
                            p75_rounds: 0.0,
                            p90_rounds: 0.0,
                            avg_result: row.get(19).unwrap_or("0").parse().unwrap_or(0.0),
                            avg_profit: row.get(20).unwrap_or("0").parse().unwrap_or(0.0),
                            median_result: row.get(21).unwrap_or("0").parse().unwrap_or(0.0),
                            median_profit: row.get(22).unwrap_or("0").parse().unwrap_or(0.0),
                            max_capital: row.get(23).unwrap_or("0").parse().unwrap_or(0.0),
                            max_consecutive_losses: row.get(24).unwrap_or("0").parse().unwrap_or(0),
                            max_consecutive_wins: row.get(25).unwrap_or("0").parse().unwrap_or(0),
                            duration_ms: row.get(26).unwrap_or("0").parse().unwrap_or(0.0),
                        };
                        records.push(record);
                    } else {
                        // 21-column legacy format
                        let unlimited_max_bet = row
                            .get(5)
                            .unwrap_or("false")
                            .parse::<bool>()
                            .unwrap_or(false);
                        let max_bet = if unlimited_max_bet {
                            0.0
                        } else {
                            row.get(4).unwrap_or("0").parse::<f64>().unwrap_or(0.0)
                        };
                        let rounds = row.get(7).unwrap_or("100").parse::<usize>().unwrap_or(100);

                        let record = HistoricalRecord {
                            id: row.get(0).unwrap_or("").to_string(),
                            timestamp: row.get(1).unwrap_or("").to_string(),
                            mode: "fixed_rounds".to_string(),
                            runs: row.get(2).unwrap_or("0").parse().unwrap_or(0),
                            min_bet: row.get(3).unwrap_or("0").parse().unwrap_or(0.0),
                            max_bet,
                            unlimited_max_bet,
                            start_capital: row.get(6).unwrap_or("0").parse().unwrap_or(0.0),
                            rounds_per_run: rounds,
                            target_profit: None,
                            roulette_type: row.get(8).unwrap_or("european").to_string(),
                            failed_runs: row.get(9).unwrap_or("0").parse().unwrap_or(0),
                            failed_percentage: row.get(10).unwrap_or("0").parse().unwrap_or(0.0),
                            bankrupt_runs: row.get(11).unwrap_or("0").parse().unwrap_or(0),
                            bankrupt_percentage: row.get(12).unwrap_or("0").parse().unwrap_or(0.0),
                            target_reached_runs: 0,
                            target_reached_percentage: 0.0,
                            round_limit_runs: 0,
                            round_limit_percentage: 0.0,
                            hit_safety_cap_runs: 0,
                            hit_safety_cap_percentage: 0.0,
                            avg_rounds_played: rounds as f64,
                            median_rounds_played: rounds as f64,
                            min_rounds_played: 0,
                            max_rounds_played: 0,
                            p10_rounds: 0.0,
                            p25_rounds: 0.0,
                            p75_rounds: 0.0,
                            p90_rounds: 0.0,
                            avg_result: row.get(13).unwrap_or("0").parse().unwrap_or(0.0),
                            avg_profit: row.get(14).unwrap_or("0").parse().unwrap_or(0.0),
                            median_result: row.get(15).unwrap_or("0").parse().unwrap_or(0.0),
                            median_profit: row.get(16).unwrap_or("0").parse().unwrap_or(0.0),
                            max_capital: row.get(17).unwrap_or("0").parse().unwrap_or(0.0),
                            max_consecutive_losses: row.get(18).unwrap_or("0").parse().unwrap_or(0),
                            max_consecutive_wins: row.get(19).unwrap_or("0").parse().unwrap_or(0),
                            duration_ms: row.get(20).unwrap_or("0").parse().unwrap_or(0.0),
                        };
                        records.push(record);
                    }
                }
                Err(err) => return Err(format!("Error parsing CSV row: {err}")),
            }
        }

        records.reverse();
        Ok(records)
    }

    pub fn clear_records(&self) -> Result<(), String> {
        let _guard = self.file_lock.lock().map_err(|e| e.to_string())?;
        if self.path.exists() {
            std::fs::remove_file(&self.path).map_err(|e| format!("Failed to delete CSV: {}", e))?;
        }
        Ok(())
    }

    /// Export a consistent current schema even when the source has mixed legacy rows.
    pub fn export_csv(&self) -> Result<String, String> {
        let mut records = self.get_all_records()?;
        records.reverse(); // Exports remain chronological; the UI uses newest first.
        let mut writer = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(Vec::new());
        writer
            .write_record(CSV_HEADER.split(','))
            .map_err(|e| e.to_string())?;
        for record in records {
            writer.serialize(record).map_err(|e| e.to_string())?;
        }
        let bytes = writer.into_inner().map_err(|e| e.to_string())?;
        String::from_utf8(bytes).map_err(|e| e.to_string())
    }
}

impl Default for StorageManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{summary, TestDirectory};

    #[test]
    fn missing_history_exports_a_header_and_clear_is_idempotent() {
        let dir = TestDirectory::new();
        let storage = StorageManager::with_path(dir.csv_path());
        assert!(storage.get_all_records().unwrap().is_empty());
        assert_eq!(storage.export_csv().unwrap(), format!("{CSV_HEADER}\n"));
        storage.clear_records().unwrap();
        storage.clear_records().unwrap();
    }
    #[test]
    fn csv_roundtrip_preserves_precision_quotes_and_newest_first() {
        let dir = TestDirectory::new();
        let storage = StorageManager::with_path(dir.csv_path());
        std::fs::write(dir.csv_path(), "").unwrap();
        let mut first = summary();
        first.id = "first,\"quoted\"".into();
        first.avg_result = 123.456789;
        let mut second = first.clone();
        second.id = "second".into();
        storage.save_record(&first).unwrap();
        storage.save_record(&second).unwrap();
        let rows = storage.get_all_records().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, "second");
        assert_eq!(rows[1].id, first.id);
        assert_eq!(rows[1].avg_result, 123.456789);
        storage.clear_records().unwrap();
        assert!(storage.get_all_records().unwrap().is_empty());
    }
    #[test]
    fn unlimited_history_remains_a_finite_json_number() {
        let dir = TestDirectory::new();
        let storage = StorageManager::with_path(dir.csv_path());
        let mut s = summary();
        s.params.unlimited_max_bet = true;
        storage.save_record(&s).unwrap();
        let rows = storage.get_all_records().unwrap();
        let json = serde_json::to_value(&rows).unwrap();
        assert_eq!(json[0]["max_bet"], 0.0);
        assert_eq!(json[0]["unlimited_max_bet"], true);
    }
    #[test]
    fn mixed_legacy_and_current_rows_are_read_without_changing_existing_data() {
        let dir = TestDirectory::new();
        let storage = StorageManager::with_path(dir.csv_path());
        // Original 21-column and intermediate 27-column layouts, including "unlimited".
        let old =
            "legacy,2026-01-01,10,5,unlimited,true,100,3,european,2,20,1,10,90,-10,100,0,120,2,3,1";
        let intermediate="intermediate,2026-01-02,10,5,500,false,100,target_profit,3,15,european,2,20,1,10,7,70,2,2,110,10,115,15,120,2,3,1";
        std::fs::write(
            dir.csv_path(),
            format!("{CSV_HEADER}\n{old}\n{intermediate}\n"),
        )
        .unwrap();
        let mut s = summary();
        s.id = "current".into();
        storage.save_record(&s).unwrap();
        let rows = storage.get_all_records().unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].id, "current");
        assert_eq!(rows[1].mode, "target_profit");
        assert_eq!(rows[1].target_profit, Some(15.0));
        assert!(rows[2].unlimited_max_bet);
        assert_eq!(rows[2].max_bet, 0.0);
        assert!(std::fs::read_to_string(dir.csv_path())
            .unwrap()
            .contains(old));
        let export = storage.export_csv().unwrap();
        let mut reader = csv::Reader::from_reader(export.as_bytes());
        assert_eq!(reader.headers().unwrap().len(), 37);
        let exported_rows: Result<Vec<_>, _> = reader.records().collect();
        assert_eq!(exported_rows.unwrap().len(), 3);
    }
    #[test]
    fn file_errors_are_returned_to_the_caller() {
        let dir = TestDirectory::new();
        let storage = StorageManager::with_path(dir.0.join("missing-parent/history.csv"));
        assert!(storage.save_record(&summary()).is_err());
    }
    #[test]
    fn concurrent_saves_produce_complete_rows() {
        let dir = TestDirectory::new();
        let storage = std::sync::Arc::new(StorageManager::with_path(dir.csv_path()));
        let s = summary();
        std::thread::scope(|scope| {
            for i in 0..8 {
                let storage = storage.clone();
                let mut s = s.clone();
                s.id = format!("record-{i}");
                scope.spawn(move || storage.save_record(&s).unwrap());
            }
        });
        assert_eq!(storage.get_all_records().unwrap().len(), 8);
    }
}

#[cfg(test)]
mod malformed_tests {
    use super::*;
    use crate::test_support::TestDirectory;
    #[test]
    fn unsupported_row_width_is_reported() {
        let dir = TestDirectory::new();
        std::fs::write(dir.csv_path(), format!("{CSV_HEADER}\ninvalid,row\n")).unwrap();
        let storage = StorageManager::with_path(dir.csv_path());
        assert!(storage.get_all_records().unwrap_err().contains("columns"));
    }
}
