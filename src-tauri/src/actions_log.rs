//! Reads the tail of the daemon's append-only activity log (`.actions.jsonl`) so
//! the UI can show exactly what the daemon changed on this machine — the
//! "observable, owned" half of the trust story. This is the same data
//! `hyperspell status` prints, rendered nicer.

use crate::daemon_paths::actions_log_path;
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};

/// Return up to `limit` most-recent actions (oldest→newest), each a JSON object.
pub fn recent(limit: usize) -> Vec<Value> {
    let Ok(file) = File::open(actions_log_path()) else {
        return Vec::new();
    };
    let mut lines: Vec<Value> = BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|l| serde_json::from_str::<Value>(&l).ok())
        .collect();
    if lines.len() > limit {
        lines = lines.split_off(lines.len() - limit);
    }
    lines
}
