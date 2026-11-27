use crate::compiler::{init_workspace, McpManifest, McpServerSpec};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{BufRead, Write};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct FoundRule {
    pub source_agent: String,
    pub source_file: String,
    pub text: String,
    pub normalized: String,
    pub polarity: char,
}

#[derive(Debug, Clone)]
pub struct ConflictPair {
    pub kind: String,
    pub a: FoundRule,
    pub b: FoundRule,
}

#[derive(Debug, Clone, Default)]
pub struct UnifySummary {
    pub sources_scanned: usize,
    pub total_rules_found: usize,
    pub deduplicated: usize,
    pub conflicts_resolved: usize,
    pub mcp_merged: usize,
    pub backup_dir: String,
}

fn normalize_rule(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c.is_whitespace() {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn detect_polarity(s: &str) -> char {
    let lower = s.to_lowercase();
    let neg_words = [
        "never ", "do not ", "don't ", "avoid ", "nunca ", "no uses ", "evita ",
    ];
    if neg_words.iter().any(|w| lower.contains(w)) {
        '-'
    } else {
        '+'
    }
}

fn strip_cue(norm: &str) -> String {
    for prefix in [
        "always ", "never ", "do not ", "don t ", "use ", "prefer ", "avoid ", "siempre ",
        "nunca ", "usa ", "evita ",
    ] {
        if let Some(rest) = norm.strip_prefix(prefix) {
            return rest.to_string();
        }
    }
    norm.to_string()
}

fn extract_rules_from_markdown(content: &str, agent: &str, file_rel: &str) -> Vec<FoundRule> {
    let mut out = Vec::new();
    let mut in_code = false;
    let mut skip_fm = content.starts_with("---\n");
    let mut fm_lines = 0usize;

    for line in content.lines() {
        let trimmed = line.trim();
        if skip_fm {
            fm_lines += 1;
            if fm_lines > 1 && trimmed == "---" {
                skip_fm = false;
            }
            continue;
        }
        if trimmed.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code || trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("<!--")
        {
            continue;
        }
        let clean = trimmed
            .trim_start_matches(['-', '*', '+'])
            .trim()
            .to_string();
        if clean.chars().count() < 6 {
            continue;
        }
        let norm = normalize_rule(&clean);
        let pol = detect_polarity(&clean);
        out.push(FoundRule {
            source_agent: agent.to_string(),
            source_file: file_rel.to_string(),
            text: clean,
            normalized: norm,
            polarity: pol,
        });
    }
    out
}

type McpCandidates = BTreeMap<String, Vec<(String, McpServerSpec)>>;

fn collect_existing_sources(root: &Path) -> (Vec<FoundRule>, McpCandidates, Vec<String>) {
    let mut rules = Vec::new();
    let mut mcp_candidates: BTreeMap<String, Vec<(String, McpServerSpec)>> = BTreeMap::new();
    let mut existing_files = Vec::new();

    let md_sources = [
        ("claude", "CLAUDE.md"),
        ("codex", "AGENTS.md"),
        ("gemini", "GEMINI.md"),
        ("cursor", ".cursorrules"),
        ("copilot", ".github/copilot-instructions.md"),
    ];
    for (agent, rel) in md_sources {
        let mut p = root.to_path_buf();
        for seg in rel.split('/') {
            p.push(seg);
        }
        if p.is_file() {
            if let Ok(text) = fs::read_to_string(&p) {
                existing_files.push(rel.to_string());
                rules.extend(extract_rules_from_markdown(&text, agent, rel));
            }
        }
    }

    let cursor_rules_dir = root.join(".cursor").join("rules");
    if let Ok(entries) = fs::read_dir(&cursor_rules_dir) {
        let mut paths: Vec<_> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect();
        paths.sort();
        for p in paths {
            let rel = format!(
                ".cursor/rules/{}",
                p.file_name().and_then(|s| s.to_str()).unwrap_or("rule.mdc")
            );
            if let Ok(text) = fs::read_to_string(&p) {
                existing_files.push(rel.clone());
                rules.extend(extract_rules_from_markdown(&text, "cursor", &rel));
            }
        }
    }

    let mcp_json_sources = [".mcp.json", ".cursor/mcp.json", ".vscode/mcp.json"];
    for rel in mcp_json_sources {
        let mut p = root.to_path_buf();
        for seg in rel.split('/') {
            p.push(seg);
        }
        if p.is_file() {
            if let Ok(raw) = fs::read_to_string(&p) {
                if let Ok(manifest) = serde_json::from_str::<McpManifest>(&raw) {
                    existing_files.push(rel.to_string());
                    for (name, spec) in manifest.mcp_servers {
                        mcp_candidates
                            .entry(name)
                            .or_default()
                            .push((rel.to_string(), spec));
                    }
                }
            }
        }
    }

    let codex_toml = root.join(".codex").join("config.toml");
    if codex_toml.is_file() {
        if let Ok(raw) = fs::read_to_string(&codex_toml) {
            if let Ok(val) = toml::from_str::<toml::Value>(&raw) {
                if let Some(tbl) = val.get("mcp_servers").and_then(|v| v.as_table()) {
                    existing_files.push(".codex/config.toml".into());
                    for (name, v) in tbl {
                        if let Some(obj) = v.as_table() {
                            let cmd = obj
                                .get("command")
                                .and_then(|c| c.as_str())
                                .unwrap_or("")
                                .to_string();
                            let args = obj
                                .get("args")
                                .and_then(|a| a.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|x| x.as_str().map(String::from))
                                        .collect()
                                })
                                .unwrap_or_default();
                            if !cmd.is_empty() {
                                mcp_candidates.entry(name.clone()).or_default().push((
                                    ".codex/config.toml".into(),
                                    McpServerSpec {
                                        command: cmd,
                                        args,
                                        env: BTreeMap::new(),
                                    },
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    (rules, mcp_candidates, existing_files)
}

fn find_exclusive_group_conflicts(rules: &[FoundRule]) -> Vec<ConflictPair> {
    let groups: &[(&str, &[&str])] = &[
        ("package_manager", &["npm", "pnpm", "yarn", "bun"]),
        ("python_env", &["uv", "poetry", "pipenv"]),
        ("indentation", &["tabs", "spaces"]),
        ("language", &["spanish", "english", "español"]),
    ];
    let mut conflicts = Vec::new();
    for (gname, opts) in groups {
        let mut by_opt: HashMap<&str, FoundRule> = HashMap::new();
        for r in rules.iter().filter(|r| r.polarity == '+') {
            for opt in *opts {
                let padded = format!(" {} ", r.normalized);
                if padded.contains(&format!(" {} ", opt)) {
                    by_opt.entry(*opt).or_insert_with(|| r.clone());
                }
            }
        }
        if by_opt.len() > 1 {
            let mut vals: Vec<FoundRule> = by_opt.into_values().collect();
            vals.sort_by(|a, b| a.source_agent.cmp(&b.source_agent));
            if vals.len() >= 2 && vals[0].normalized != vals[1].normalized {
                conflicts.push(ConflictPair {
                    kind: gname.to_string(),
                    a: vals[0].clone(),
                    b: vals[1].clone(),
                });
            }
        }
    }
    conflicts
}

fn prompt_choice<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    prompt: &str,
    default: &str,
) -> String {
    let _ = write!(writer, "{} [{}]: ", prompt, default);
    let _ = writer.flush();
    let mut line = String::new();
    if reader.read_line(&mut line).unwrap_or(0) == 0 {
        return default.to_string();
    }
    let trimmed = line.trim();
    if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn run_unify<R: BufRead, W: Write>(
    root: &Path,
    auto_accept: bool,
    reader: &mut R,
    writer: &mut W,
) -> Result<UnifySummary, String> {
    let (rules, mcp_candidates, existing_files) = collect_existing_sources(root);
    let base = init_workspace(root)?;

    // Backup existing files before modifying anything
    let backup_dir = base.join("backup");
    if !existing_files.is_empty() {
        fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;
        for rel in &existing_files {
            let mut src = root.to_path_buf();
            let mut dst = backup_dir.clone();
            for seg in rel.split('/') {
                src.push(seg);
                dst.push(seg);
            }
            if src.is_file() {
                if let Some(parent) = dst.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::copy(&src, &dst);
            }
        }
    }

    let mut summary = UnifySummary {
        sources_scanned: existing_files.len(),
        total_rules_found: rules.len(),
        backup_dir: backup_dir.to_string_lossy().into_owned(),
        ..Default::default()
    };

    // Group rules by stripped core text
    let mut grouped: BTreeMap<String, Vec<FoundRule>> = BTreeMap::new();
    for r in &rules {
        let key = strip_cue(&r.normalized);
        grouped.entry(key).or_default().push(r.clone());
    }

    let mut shared_rules: Vec<String> = Vec::new();
    let mut scoped_rules: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut excluded_normalized: Vec<String> = Vec::new();

    // Resolve direct polarity conflicts first
    for group in grouped.values() {
        let pos: Vec<&FoundRule> = group.iter().filter(|r| r.polarity == '+').collect();
        let neg: Vec<&FoundRule> = group.iter().filter(|r| r.polarity == '-').collect();
        if !pos.is_empty() && !neg.is_empty() {
            let a = pos[0];
            let b = neg[0];
            let choice = if auto_accept {
                "1".to_string()
            } else {
                let _ = writeln!(writer, "\n[CONFLICTO] Regla opuesta detectada:");
                let _ = writeln!(
                    writer,
                    "  1) [{} - {}] {}",
                    a.source_agent, a.source_file, a.text
                );
                let _ = writeln!(
                    writer,
                    "  2) [{} - {}] {}",
                    b.source_agent, b.source_file, b.text
                );
                let _ = writeln!(writer, "  3) Mantener cada una solo en su agente");
                prompt_choice(reader, writer, "Elige opcion (1/2/3)", "1")
            };
            summary.conflicts_resolved += 1;
            match choice.as_str() {
                "2" => {
                    shared_rules.push(b.text.clone());
                    excluded_normalized.push(a.normalized.clone());
                }
                "3" => {
                    scoped_rules
                        .entry(a.source_agent.clone())
                        .or_default()
                        .push(a.text.clone());
                    scoped_rules
                        .entry(b.source_agent.clone())
                        .or_default()
                        .push(b.text.clone());
                    excluded_normalized.push(a.normalized.clone());
                    excluded_normalized.push(b.normalized.clone());
                }
                _ => {
                    shared_rules.push(a.text.clone());
                    excluded_normalized.push(b.normalized.clone());
                }
            }
        }
    }

    // Resolve exclusive-group conflicts (e.g. npm vs pnpm)
    for cp in find_exclusive_group_conflicts(&rules) {
        if excluded_normalized.contains(&cp.a.normalized)
            || excluded_normalized.contains(&cp.b.normalized)
        {
            continue;
        }
        let choice = if auto_accept {
            "1".to_string()
        } else {
            let _ = writeln!(writer, "\n[CONFLICTO: {}]", cp.kind);
            let _ = writeln!(
                writer,
                "  1) [{} - {}] {}",
                cp.a.source_agent, cp.a.source_file, cp.a.text
            );
            let _ = writeln!(
                writer,
                "  2) [{} - {}] {}",
                cp.b.source_agent, cp.b.source_file, cp.b.text
            );
            let _ = writeln!(writer, "  3) Mantener cada una solo en su agente");
            prompt_choice(reader, writer, "Elige cual prevalece (1/2/3)", "1")
        };
        summary.conflicts_resolved += 1;
        match choice.as_str() {
            "2" => {
                excluded_normalized.push(cp.a.normalized.clone());
            }
            "3" => {
                scoped_rules
                    .entry(cp.a.source_agent.clone())
                    .or_default()
                    .push(cp.a.text.clone());
                scoped_rules
                    .entry(cp.b.source_agent.clone())
                    .or_default()
                    .push(cp.b.text.clone());
                excluded_normalized.push(cp.a.normalized.clone());
                excluded_normalized.push(cp.b.normalized.clone());
            }
            _ => {
                excluded_normalized.push(cp.b.normalized.clone());
            }
        }
    }

    // Deduplicate remaining rules
    for (_key, group) in grouped {
        let valid: Vec<FoundRule> = group
            .into_iter()
            .filter(|r| !excluded_normalized.contains(&r.normalized))
            .collect();
        if valid.is_empty() {
            continue;
        }
        if valid.len() > 1 {
            summary.deduplicated += valid.len() - 1;
        }
        let representative = &valid[0];
        if !shared_rules
            .iter()
            .any(|s| normalize_rule(s) == representative.normalized)
        {
            // If rule appeared in only 1 agent and multiple agents exist, ask or promote
            let unique_agents: Vec<&str> = {
                let mut v: Vec<&str> = valid.iter().map(|r| r.source_agent.as_str()).collect();
                v.sort();
                v.dedup();
                v
            };
            if unique_agents.len() == 1 && !auto_accept && existing_files.len() > 1 {
                let _ = writeln!(
                    writer,
                    "\n[REGLA UNICA en {}] {}",
                    unique_agents[0], representative.text
                );
                let ans = prompt_choice(
                    reader,
                    writer,
                    "¿Promover a todos los agentes (a), mantener solo en este (s), o descartar (d)?",
                    "a",
                );
                match ans.to_ascii_lowercase().as_str() {
                    "s" => {
                        scoped_rules
                            .entry(unique_agents[0].to_string())
                            .or_default()
                            .push(representative.text.clone());
                    }
                    "d" => {}
                    _ => shared_rules.push(representative.text.clone()),
                }
            } else {
                shared_rules.push(representative.text.clone());
            }
        }
    }

    // Write unified rules
    let rules_dir = base.join("rules");
    // Remove starter general.md if we actually unified real rules
    if !shared_rules.is_empty() || !scoped_rules.is_empty() {
        let starter = rules_dir.join("general.md");
        if starter.is_file() {
            let _ = fs::remove_file(starter);
        }
    }

    if !shared_rules.is_empty() {
        let bullets = shared_rules
            .iter()
            .map(|r| format!("- {}", r))
            .collect::<Vec<_>>()
            .join("\n");
        let content = format!(
            "---\ndescription: Unified project guidelines\ntargets: [\"*\"]\nglobs: [\"*\"]\n---\n\n{}\n",
            bullets
        );
        fs::write(rules_dir.join("unified.md"), content).map_err(|e| e.to_string())?;
    }

    for (agent, rlist) in scoped_rules {
        let bullets = rlist
            .iter()
            .map(|r| format!("- {}", r))
            .collect::<Vec<_>>()
            .join("\n");
        let content = format!(
            "---\ndescription: {} specific guidelines\ntargets: [\"{}\"]\nglobs: [\"*\"]\n---\n\n{}\n",
            agent, agent, bullets
        );
        fs::write(rules_dir.join(format!("agent-{}.md", agent)), content)
            .map_err(|e| e.to_string())?;
    }

    // Merge MCP servers
    let mut merged_mcp = McpManifest::default();
    for (srv_name, candidates) in mcp_candidates {
        let chosen = if candidates.len() == 1 || auto_accept {
            candidates[0].1.clone()
        } else {
            let all_same = candidates
                .windows(2)
                .all(|w| w[0].1.command == w[1].1.command && w[0].1.args == w[1].1.args);
            if all_same {
                candidates[0].1.clone()
            } else {
                let _ = writeln!(
                    writer,
                    "\n[MCP CONFLICTO] Servidor '{}' difiere entre archivos:",
                    srv_name
                );
                for (idx, (src, spec)) in candidates.iter().enumerate() {
                    let _ = writeln!(
                        writer,
                        "  {}) [{}] {} {}",
                        idx + 1,
                        src,
                        spec.command,
                        spec.args.join(" ")
                    );
                }
                let ans = prompt_choice(reader, writer, "Elige definicion", "1");
                let idx = ans.parse::<usize>().unwrap_or(1).saturating_sub(1);
                candidates
                    .get(idx)
                    .map(|c| c.1.clone())
                    .unwrap_or_else(|| candidates[0].1.clone())
            }
        };
        merged_mcp.mcp_servers.insert(srv_name, chosen);
        summary.mcp_merged += 1;
    }

    if !merged_mcp.mcp_servers.is_empty() {
        let json_out = serde_json::to_string_pretty(&merged_mcp).map_err(|e| e.to_string())? + "\n";
        fs::write(base.join("mcp.json"), json_out).map_err(|e| e.to_string())?;
    }

    let _ = writeln!(
        writer,
        "\nUnificacion completada: {} archivos escaneados, {} reglas encontradas, {} duplicadas eliminadas, {} conflictos resueltos, {} servidores MCP unificados.",
        summary.sources_scanned,
        summary.total_rules_found,
        summary.deduplicated,
        summary.conflicts_resolved,
        summary.mcp_merged
    );

    Ok(summary)
}
