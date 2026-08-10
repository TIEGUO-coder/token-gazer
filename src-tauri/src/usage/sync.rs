use super::{claude::parse_claude_jsonl, codex::parse_codex_jsonl};
use crate::db::insert_usage_event;
use rusqlite::Connection;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize)]
pub struct SyncReport {
    pub files_scanned: u64,
    pub events_imported: u64,
    pub events_skipped: u64,
    pub errors: Vec<String>,
}

fn collect_jsonl_recursive(root: &Path, max_depth: usize) -> Vec<PathBuf> {
    fn walk(dir: &Path, depth: usize, max_depth: usize, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && depth < max_depth {
                walk(&path, depth + 1, max_depth, out);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
                out.push(path);
            }
        }
    }

    let mut files = Vec::new();
    walk(root, 0, max_depth, &mut files);
    files.sort();
    files
}

pub fn sync_usage_logs(conn: &Connection, home: &Path) -> SyncReport {
    let mut report = SyncReport::default();

    let claude_root = home.join(".claude").join("projects");
    if claude_root.is_dir() {
        for file in collect_jsonl_recursive(&claude_root, 5) {
            report.files_scanned += 1;
            match fs::File::open(&file) {
                Ok(handle) => {
                    for event in parse_claude_jsonl(handle) {
                        match insert_usage_event(conn, &event) {
                            Ok(true) => report.events_imported += 1,
                            Ok(false) => report.events_skipped += 1,
                            Err(error) => {
                                report.errors.push(format!("{}: {error}", file.display()))
                            }
                        }
                    }
                }
                Err(error) => report.errors.push(format!("{}: {error}", file.display())),
            }
        }
    }

    let codex_sessions = home.join(".codex").join("sessions");
    if codex_sessions.is_dir() {
        for file in collect_jsonl_recursive(&codex_sessions, 4) {
            report.files_scanned += 1;
            match fs::File::open(&file) {
                Ok(handle) => {
                    let stem = file
                        .file_stem()
                        .and_then(|value| value.to_str())
                        .unwrap_or("unknown");
                    for event in parse_codex_jsonl(handle, stem) {
                        match insert_usage_event(conn, &event) {
                            Ok(true) => report.events_imported += 1,
                            Ok(false) => report.events_skipped += 1,
                            Err(error) => {
                                report.errors.push(format!("{}: {error}", file.display()))
                            }
                        }
                    }
                }
                Err(error) => report.errors.push(format!("{}: {error}", file.display())),
            }
        }
    }

    let codex_archived = home.join(".codex").join("archived_sessions");
    if codex_archived.is_dir() {
        for file in collect_jsonl_recursive(&codex_archived, 1) {
            report.files_scanned += 1;
            match fs::File::open(&file) {
                Ok(handle) => {
                    let stem = file
                        .file_stem()
                        .and_then(|value| value.to_str())
                        .unwrap_or("unknown");
                    for event in parse_codex_jsonl(handle, stem) {
                        match insert_usage_event(conn, &event) {
                            Ok(true) => report.events_imported += 1,
                            Ok(false) => report.events_skipped += 1,
                            Err(error) => {
                                report.errors.push(format!("{}: {error}", file.display()))
                            }
                        }
                    }
                }
                Err(error) => report.errors.push(format!("{}: {error}", file.display())),
            }
        }
    }

    report
}
