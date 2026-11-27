use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const ALL_TARGETS: &[&str] = &["claude", "codex", "gemini", "cursor", "copilot"];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpServerSpec {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpManifest {
    #[serde(default, rename = "mcpServers")]
    pub mcp_servers: BTreeMap<String, McpServerSpec>,
}

#[derive(Debug, Clone)]
pub struct RuleDoc {
    pub name: String,
    pub description: String,
    pub globs: Vec<String>,
    pub targets: Vec<String>,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub rel_path: String,
    pub content: String,
}

pub fn sanitize_rule_name(raw: &str) -> Result<String, String> {
    let clean: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if clean.is_empty() {
        return Err(format!("Invalid rule name: '{}'", raw));
    }
    Ok(clean)
}

pub fn init_workspace(root: &Path) -> Result<PathBuf, String> {
    let base = root.join(".dotagents");
    let rules_dir = base.join("rules");
    fs::create_dir_all(&rules_dir).map_err(|e| e.to_string())?;

    let sample_rule = rules_dir.join("general.md");
    if !sample_rule.exists() {
        fs::write(
            &sample_rule,
            "---\ndescription: Core project guidelines\ntargets: [\"*\"]\nglobs: [\"*\"]\n---\n\n- Keep functions small and well-tested.\n- Never commit secrets or .env files.\n",
        )
        .map_err(|e| e.to_string())?;
    }

    let mcp_file = base.join("mcp.json");
    if !mcp_file.exists() {
        fs::write(&mcp_file, "{\n  \"mcpServers\": {}\n}\n").map_err(|e| e.to_string())?;
    }
    Ok(base)
}

pub fn import_existing(root: &Path) -> Result<usize, String> {
    let base = init_workspace(root)?;
    let rules_dir = base.join("rules");
    let mut imported = 0usize;

    let sources = [
        ("claude", "CLAUDE.md"),
        ("agents", "AGENTS.md"),
        ("gemini", "GEMINI.md"),
        ("cursorrules", ".cursorrules"),
    ];
    for (name, rel) in sources {
        let src = root.join(rel);
        if src.is_file() {
            let content = fs::read_to_string(&src).map_err(|e| e.to_string())?;
            let dest = rules_dir.join(format!("imported-{}.md", name));
            let wrapped = format!(
                "---\ndescription: Imported from {}\ntargets: [\"*\"]\nglobs: [\"*\"]\n---\n\n{}\n",
                rel,
                content.trim()
            );
            fs::write(dest, wrapped).map_err(|e| e.to_string())?;
            imported += 1;
        }
    }

    let mcp_src = root.join(".mcp.json");
    if mcp_src.is_file() {
        let raw = fs::read_to_string(&mcp_src).map_err(|e| e.to_string())?;
        fs::write(base.join("mcp.json"), raw).map_err(|e| e.to_string())?;
        imported += 1;
    }
    Ok(imported)
}

fn parse_frontmatter(raw: &str, name: &str) -> RuleDoc {
    let mut description = String::from("Project guidelines");
    let mut globs = vec!["*".to_string()];
    let mut targets = vec!["*".to_string()];
    let mut body = raw.trim().to_string();

    if let Some(rest) = raw.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---\n") {
            let fm = &rest[..end];
            body = rest[end + 5..].trim().to_string();
            for line in fm.lines() {
                if let Some((k, v)) = line.split_once(':') {
                    let key = k.trim();
                    let val = v.trim();
                    match key {
                        "description" => description = val.trim_matches('"').to_string(),
                        "targets" => targets = parse_inline_list(val),
                        "globs" => globs = parse_inline_list(val),
                        _ => {}
                    }
                }
            }
        }
    }

    RuleDoc {
        name: name.to_string(),
        description,
        globs,
        targets,
        body,
    }
}

fn parse_inline_list(val: &str) -> Vec<String> {
    val.trim_matches(|c| c == '[' || c == ']')
        .split(',')
        .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn load_rules(root: &Path) -> Result<Vec<RuleDoc>, String> {
    let rules_dir = root.join(".dotagents").join("rules");
    if !rules_dir.is_dir() {
        return Err("Missing .dotagents/rules directory. Run `dotagents init` first.".into());
    }
    let mut entries: Vec<PathBuf> = fs::read_dir(&rules_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("md"))
        .collect();
    entries.sort();

    let mut docs = Vec::new();
    for path in entries {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("rule");
        let name = sanitize_rule_name(stem)?;
        let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        docs.push(parse_frontmatter(&raw, &name));
    }
    Ok(docs)
}

fn rule_applies(doc: &RuleDoc, target: &str) -> bool {
    doc.targets
        .iter()
        .any(|t| t == "*" || t.eq_ignore_ascii_case(target))
}

fn render_markdown_bundle(docs: &[RuleDoc], target: &str) -> String {
    let mut sections =
        vec!["<!-- Generated by dotagents. Edit .dotagents/rules/ instead. -->".to_string()];
    for d in docs.iter().filter(|d| rule_applies(d, target)) {
        sections.push(format!("## {}\n\n{}", d.description, d.body));
    }
    sections.join("\n\n") + "\n"
}

fn render_mcp_for_os(manifest: &McpManifest, home: &Path, is_windows: bool) -> McpManifest {
    let home_str = if is_windows {
        home.to_string_lossy().into_owned()
    } else {
        home.to_string_lossy().replace('\\', "/")
    };

    let mut out = McpManifest::default();
    for (name, spec) in &manifest.mcp_servers {
        let mut cmd = spec.command.clone();
        let mut args: Vec<String> = spec
            .args
            .iter()
            .map(|a| a.replace("{{HOME}}", &home_str))
            .collect();
        if is_windows && matches!(cmd.as_str(), "npx" | "uvx" | "pnpx" | "bunx") {
            let mut wrapped = vec!["/c".to_string(), cmd];
            wrapped.extend(args);
            cmd = "cmd".to_string();
            args = wrapped;
        }
        let env = spec
            .env
            .iter()
            .map(|(k, v)| (k.clone(), v.replace("{{HOME}}", &home_str)))
            .collect();
        out.mcp_servers.insert(
            name.clone(),
            McpServerSpec {
                command: cmd,
                args,
                env,
            },
        );
    }
    out
}

pub fn compile_targets(
    root: &Path,
    targets: &[String],
    home: &Path,
    is_windows: bool,
) -> Result<Vec<GeneratedFile>, String> {
    let docs = load_rules(root)?;
    let mut outputs = Vec::new();

    let wants = |t: &str| {
        targets
            .iter()
            .any(|x| x == "all" || x == "*" || x.eq_ignore_ascii_case(t))
    };

    if wants("claude") {
        outputs.push(GeneratedFile {
            rel_path: "CLAUDE.md".into(),
            content: render_markdown_bundle(&docs, "claude"),
        });
    }
    if wants("codex") {
        outputs.push(GeneratedFile {
            rel_path: "AGENTS.md".into(),
            content: render_markdown_bundle(&docs, "codex"),
        });
    }
    if wants("gemini") {
        outputs.push(GeneratedFile {
            rel_path: "GEMINI.md".into(),
            content: render_markdown_bundle(&docs, "gemini"),
        });
    }
    if wants("copilot") {
        outputs.push(GeneratedFile {
            rel_path: ".github/copilot-instructions.md".into(),
            content: render_markdown_bundle(&docs, "copilot"),
        });
    }
    if wants("cursor") {
        for d in docs.iter().filter(|d| rule_applies(d, "cursor")) {
            let globs = d.globs.join(", ");
            let mdc = format!(
                "---\ndescription: {}\nglobs: {}\nalwaysApply: {}\n---\n\n{}\n",
                d.description,
                globs,
                d.globs.iter().any(|g| g == "*"),
                d.body
            );
            outputs.push(GeneratedFile {
                rel_path: format!(".cursor/rules/{}.mdc", d.name),
                content: mdc,
            });
        }
    }

    let mcp_path = root.join(".dotagents").join("mcp.json");
    if mcp_path.is_file() {
        let raw = fs::read_to_string(&mcp_path).map_err(|e| e.to_string())?;
        let manifest: McpManifest = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        if !manifest.mcp_servers.is_empty() {
            let rendered = render_mcp_for_os(&manifest, home, is_windows);
            let json_text =
                serde_json::to_string_pretty(&rendered).map_err(|e| e.to_string())? + "\n";

            if wants("claude") || wants("gemini") {
                outputs.push(GeneratedFile {
                    rel_path: ".mcp.json".into(),
                    content: json_text.clone(),
                });
            }
            if wants("cursor") {
                outputs.push(GeneratedFile {
                    rel_path: ".cursor/mcp.json".into(),
                    content: json_text,
                });
            }
            if wants("codex") {
                let mut toml_root = toml::map::Map::new();
                let mut servers_tbl = toml::map::Map::new();
                for (sname, spec) in &rendered.mcp_servers {
                    let mut entry = toml::map::Map::new();
                    entry.insert("command".into(), toml::Value::String(spec.command.clone()));
                    entry.insert(
                        "args".into(),
                        toml::Value::Array(
                            spec.args.iter().cloned().map(toml::Value::String).collect(),
                        ),
                    );
                    servers_tbl.insert(sname.clone(), toml::Value::Table(entry));
                }
                toml_root.insert("mcp_servers".into(), toml::Value::Table(servers_tbl));
                let toml_text = toml::to_string_pretty(&toml::Value::Table(toml_root))
                    .map_err(|e| e.to_string())?;
                outputs.push(GeneratedFile {
                    rel_path: ".codex/config.toml".into(),
                    content: toml_text,
                });
            }
        }
    }

    Ok(outputs)
}

pub fn write_generated(root: &Path, files: &[GeneratedFile]) -> Result<usize, String> {
    for gf in files {
        let mut dest = root.to_path_buf();
        for seg in gf.rel_path.split('/') {
            dest.push(seg);
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(dest, &gf.content).map_err(|e| e.to_string())?;
    }
    Ok(files.len())
}

pub fn check_drift(root: &Path, files: &[GeneratedFile]) -> Vec<String> {
    let mut drifted = Vec::new();
    for gf in files {
        let mut dest = root.to_path_buf();
        for seg in gf.rel_path.split('/') {
            dest.push(seg);
        }
        let current = fs::read_to_string(&dest).unwrap_or_default();
        if current != gf.content {
            drifted.push(gf.rel_path.clone());
        }
    }
    drifted
}
