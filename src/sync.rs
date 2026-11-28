use crate::compiler::{check_drift, compile_targets, init_workspace, write_generated};
use crate::unify::run_unify;
use std::fs;
use std::io;
use std::path::Path;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct SyncOutcome {
    pub mode: String, // "forward" | "absorbed" | "up-to-date"
    pub newest_source: Option<String>,
    pub files_written: usize,
}

fn latest_mtime_in_dir(dir: &Path) -> Option<SystemTime> {
    let entries = fs::read_dir(dir).ok()?;
    let mut best: Option<SystemTime> = None;
    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.is_file() {
            if let Ok(mt) = fs::metadata(&p).and_then(|m| m.modified()) {
                best = Some(best.map_or(mt, |b| b.max(mt)));
            }
        }
    }
    best
}

fn dotagents_mtime(root: &Path) -> Option<SystemTime> {
    let base = root.join(".dotagents");
    let rules_mt = latest_mtime_in_dir(&base.join("rules"));
    let mcp_mt = fs::metadata(base.join("mcp.json"))
        .and_then(|m| m.modified())
        .ok();
    match (rules_mt, mcp_mt) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

/// Smart two-way sync:
/// - If any drifted target file is newer than `.dotagents/` (e.g. Claude edited `CLAUDE.md` directly),
///   absorbs changes via `unify --auto` (with backup) before regenerating all targets.
/// - Otherwise regenerates targets from `.dotagents/`.
pub fn smart_sync(
    root: &Path,
    targets: &[String],
    home: &Path,
    is_windows: bool,
    force_from: Option<&str>,
) -> Result<SyncOutcome, String> {
    if !root.join(".dotagents").is_dir() {
        init_workspace(root)?;
    }

    if let Some(agent) = force_from {
        let mut sink = io::sink();
        let mut empty = io::empty();
        run_unify(root, true, &mut empty, &mut sink)?;
        let files = compile_targets(root, targets, home, is_windows)?;
        let written = write_generated(root, &files)?;
        return Ok(SyncOutcome {
            mode: "absorbed".into(),
            newest_source: Some(agent.to_string()),
            files_written: written,
        });
    }

    let files = compile_targets(root, targets, home, is_windows)?;
    let drifted = check_drift(root, &files);
    if drifted.is_empty() {
        return Ok(SyncOutcome {
            mode: "up-to-date".into(),
            newest_source: None,
            files_written: 0,
        });
    }

    let base_mt = dotagents_mtime(root);
    let mut newer_target: Option<(String, SystemTime)> = None;

    for rel in &drifted {
        let mut p = root.to_path_buf();
        for seg in rel.split('/') {
            p.push(seg);
        }
        if let Ok(mt) = fs::metadata(&p).and_then(|m| m.modified()) {
            let is_newer = base_mt.is_none_or(|b| mt > b);
            if is_newer {
                match &newer_target {
                    Some((_, best_mt)) if mt <= *best_mt => {}
                    _ => newer_target = Some((rel.clone(), mt)),
                }
            }
        }
    }

    if let Some((newest_rel, _)) = newer_target {
        let mut sink = io::sink();
        let mut empty = io::empty();
        run_unify(root, true, &mut empty, &mut sink)?;
        let refreshed = compile_targets(root, targets, home, is_windows)?;
        let written = write_generated(root, &refreshed)?;
        return Ok(SyncOutcome {
            mode: "absorbed".into(),
            newest_source: Some(newest_rel),
            files_written: written,
        });
    }

    let written = write_generated(root, &files)?;
    Ok(SyncOutcome {
        mode: "forward".into(),
        newest_source: None,
        files_written: written,
    })
}
