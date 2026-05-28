//! Tokens-per-second (TPS) throughput telemetry for the REPL `/tps` command.
//!
//! Each completed model turn contributes a [`TpsSample`] (output tokens over
//! the request duration). Samples are aggregated per model into a [`TpsRecord`]
//! exposing the best and most recent throughput plus a sample count, mirroring
//! the Swift implementation's `ModelPerformanceStore` output. Samples are held
//! in memory for the session; cross-session persistence is intentionally left
//! as a follow-up.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One completed turn's throughput measurement.
#[derive(Debug, Clone, PartialEq)]
pub struct TpsSample {
    pub model: String,
    pub output_tokens: usize,
    pub duration_ms: usize,
}

impl TpsSample {
    /// Tokens per second for this sample, or `0.0` when the duration is zero.
    pub fn tps(&self) -> f64 {
        if self.duration_ms == 0 {
            return 0.0;
        }
        self.output_tokens as f64 / (self.duration_ms as f64 / 1000.0)
    }
}

/// Aggregated throughput for a single model across the session's samples.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TpsRecord {
    pub model: String,
    pub best_tps: f64,
    pub best_output_tokens: usize,
    pub best_duration_ms: usize,
    pub last_tps: f64,
    pub last_output_tokens: usize,
    pub last_duration_ms: usize,
    pub samples: usize,
}

/// Aggregate raw samples into per-model records, sorted by best TPS descending.
///
/// Samples are assumed to be in chronological order, so the final sample for a
/// model provides its "last" throughput.
pub fn aggregate(samples: &[TpsSample]) -> Vec<TpsRecord> {
    let mut records: Vec<TpsRecord> = Vec::new();
    for sample in samples {
        merge_sample(&mut records, sample);
    }
    sort_by_best_desc(&mut records);
    records
}

/// Fold a single sample into a record set: bump the matching model's sample
/// count and "last" throughput, track its "best", or insert a new record.
fn merge_sample(records: &mut Vec<TpsRecord>, sample: &TpsSample) {
    let tps = sample.tps();
    if let Some(record) = records.iter_mut().find(|r| r.model == sample.model) {
        record.samples += 1;
        record.last_tps = tps;
        record.last_output_tokens = sample.output_tokens;
        record.last_duration_ms = sample.duration_ms;
        if tps > record.best_tps {
            record.best_tps = tps;
            record.best_output_tokens = sample.output_tokens;
            record.best_duration_ms = sample.duration_ms;
        }
    } else {
        records.push(TpsRecord {
            model: sample.model.clone(),
            best_tps: tps,
            best_output_tokens: sample.output_tokens,
            best_duration_ms: sample.duration_ms,
            last_tps: tps,
            last_output_tokens: sample.output_tokens,
            last_duration_ms: sample.duration_ms,
            samples: 1,
        });
    }
}

fn sort_by_best_desc(records: &mut [TpsRecord]) {
    records.sort_by(|a, b| {
        b.best_tps
            .partial_cmp(&a.best_tps)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// Default on-disk location for the persistent TPS store, under the shared
/// tide config directory (honors `TIDE_CONFIG_DIR`).
pub fn default_store_dir() -> PathBuf {
    crate::memory::MemorySystem::tide_config_dir()
}

/// Cross-session persistence for TPS records, mirroring the Swift
/// `ModelPerformanceStore`. Records are kept in `model-tps.json` under the
/// supplied directory; every function takes the directory explicitly so it can
/// be pointed at a temp path in tests.
pub struct TpsStore;

impl TpsStore {
    fn file(dir: &Path) -> PathBuf {
        dir.join("model-tps.json")
    }

    /// Load persisted records, or an empty set when the file is absent or
    /// unreadable.
    pub fn load(dir: &Path) -> Vec<TpsRecord> {
        std::fs::read_to_string(Self::file(dir))
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    /// Merge one sample into the persisted records and write them back.
    pub fn record(dir: &Path, sample: &TpsSample) {
        let mut records = Self::load(dir);
        merge_sample(&mut records, sample);
        sort_by_best_desc(&mut records);
        let _ = Self::save(dir, &records);
    }

    /// Remove the persisted store. No-op when the file is absent.
    pub fn reset(dir: &Path) {
        let _ = std::fs::remove_file(Self::file(dir));
    }

    fn save(dir: &Path, records: &[TpsRecord]) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let json = serde_json::to_string_pretty(records).unwrap_or_else(|_| String::from("[]"));
        std::fs::write(Self::file(dir), json)
    }
}

/// Render aggregated records as a human-readable table for `/tps`.
pub fn render(records: &[TpsRecord]) -> String {
    if records.is_empty() {
        return String::from(
            "No model TPS samples yet. Run a streamed model prompt, then /tps again.",
        );
    }
    let mut lines = vec![String::from("Model TPS samples:")];
    for record in records {
        lines.push(format!(
            "  {model}  best {best} tps ({best_tok} tok / {best_dur})  last {last} tps ({last_tok} tok / {last_dur})  samples {samples}",
            model = record.model,
            best = format_tps(record.best_tps),
            best_tok = record.best_output_tokens,
            best_dur = format_duration(record.best_duration_ms),
            last = format_tps(record.last_tps),
            last_tok = record.last_output_tokens,
            last_dur = format_duration(record.last_duration_ms),
            samples = record.samples,
        ));
    }
    lines.join("\n")
}

/// Serialize aggregated records to a JSON array for `/tps --json`.
pub fn to_json(records: &[TpsRecord]) -> String {
    serde_json::to_string_pretty(records).unwrap_or_else(|_| String::from("[]"))
}

fn format_tps(tps: f64) -> String {
    format!("{tps:.1}")
}

fn format_duration(duration_ms: usize) -> String {
    if duration_ms >= 1000 {
        format!("{:.1}s", duration_ms as f64 / 1000.0)
    } else {
        format!("{duration_ms}ms")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tps_sample_computes_tokens_per_second() {
        let sample = TpsSample {
            model: String::from("m"),
            output_tokens: 100,
            duration_ms: 2000,
        };
        assert!((sample.tps() - 50.0).abs() < f64::EPSILON);

        let zero = TpsSample {
            model: String::from("m"),
            output_tokens: 10,
            duration_ms: 0,
        };
        assert_eq!(zero.tps(), 0.0);
    }

    #[test]
    fn aggregate_tracks_best_last_and_sample_count() {
        let samples = vec![
            TpsSample {
                model: String::from("fast"),
                output_tokens: 100,
                duration_ms: 1000,
            }, // 100 tps
            TpsSample {
                model: String::from("fast"),
                output_tokens: 60,
                duration_ms: 1000,
            }, // 60 tps (last)
            TpsSample {
                model: String::from("slow"),
                output_tokens: 10,
                duration_ms: 1000,
            }, // 10 tps
        ];
        let records = aggregate(&samples);

        // Sorted by best TPS descending → fast first.
        assert_eq!(records[0].model, "fast");
        assert_eq!(records[1].model, "slow");

        let fast = &records[0];
        assert_eq!(fast.samples, 2);
        assert!((fast.best_tps - 100.0).abs() < f64::EPSILON);
        assert!((fast.last_tps - 60.0).abs() < f64::EPSILON);
        assert_eq!(fast.last_output_tokens, 60);
    }

    #[test]
    fn render_and_json_handle_empty_and_populated() {
        assert!(render(&[]).contains("No model TPS samples"));
        assert_eq!(to_json(&[]), "[]");

        let records = aggregate(&[TpsSample {
            model: String::from("deepseek-v4-pro"),
            output_tokens: 200,
            duration_ms: 5000,
        }]);
        let text = render(&records);
        assert!(text.contains("Model TPS samples:"));
        assert!(text.contains("deepseek-v4-pro"));
        assert!(text.contains("40.0 tps"));
        assert!(text.contains("5.0s"));

        let json = to_json(&records);
        assert!(json.contains("\"model\": \"deepseek-v4-pro\""));
        assert!(json.contains("\"best_tps\": 40.0"));
        assert!(json.contains("\"samples\": 1"));
    }

    #[test]
    fn tps_store_persists_records_across_loads_and_resets() {
        let dir = tempfile::tempdir().expect("tempdir");

        assert!(TpsStore::load(dir.path()).is_empty());

        TpsStore::record(
            dir.path(),
            &TpsSample {
                model: String::from("m"),
                output_tokens: 100,
                duration_ms: 1000,
            },
        );
        TpsStore::record(
            dir.path(),
            &TpsSample {
                model: String::from("m"),
                output_tokens: 60,
                duration_ms: 1000,
            },
        );

        // A fresh load (simulating a new session) sees the aggregated record.
        let records = TpsStore::load(dir.path());
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].samples, 2);
        assert!((records[0].best_tps - 100.0).abs() < f64::EPSILON);
        assert!((records[0].last_tps - 60.0).abs() < f64::EPSILON);

        TpsStore::reset(dir.path());
        assert!(TpsStore::load(dir.path()).is_empty());
    }
}
