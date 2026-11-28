#[path = "../src/compiler.rs"]
mod compiler;
#[path = "../src/sync.rs"]
mod sync;
#[path = "../src/unify.rs"]
mod unify;

use compiler::{
    check_drift, compile_targets, import_existing, init_workspace, sanitize_rule_name,
    write_generated, ALL_TARGETS,
};
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use sync::smart_sync;
use tempfile::TempDir;
use unify::run_unify;

#[test]
fn init_generate_and_drift_check() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    init_workspace(root).unwrap();

    fs::write(
        root.join(".dotagents").join("mcp.json"),
        r#"{"mcpServers":{"demo":{"command":"npx","args":["-y","demo","{{HOME}}/work"]}}}"#,
    )
    .unwrap();

    let targets = vec!["all".to_string()];
    let home = PathBuf::from("/home/alice");
    let files = compile_targets(root, &targets, &home, false).unwrap();
    write_generated(root, &files).unwrap();

    assert!(root.join("CLAUDE.md").is_file());
    assert!(root.join("AGENTS.md").is_file());
    assert!(root.join("GEMINI.md").is_file());
    assert!(root.join(".github/copilot-instructions.md").is_file());
    assert!(root.join(".cursor/rules/general.mdc").is_file());
    assert!(root.join(".mcp.json").is_file());
    assert!(root.join(".codex/config.toml").is_file());

    assert!(check_drift(root, &files).is_empty());

    // Simulate drift
    fs::write(root.join("CLAUDE.md"), "manual edit").unwrap();
    assert_eq!(check_drift(root, &files), vec!["CLAUDE.md".to_string()]);
}

#[test]
fn windows_mcp_wrapping_and_import() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    fs::write(root.join("CLAUDE.md"), "- Always write tests.").unwrap();
    let imported = import_existing(root).unwrap();
    assert_eq!(imported, 1);

    fs::write(
        root.join(".dotagents").join("mcp.json"),
        r#"{"mcpServers":{"gh":{"command":"npx","args":["-y","srv","{{HOME}}/repo"]}}}"#,
    )
    .unwrap();

    let win_home = PathBuf::from(r"C:\Users\alice");
    let files = compile_targets(root, &["claude".to_string()], &win_home, true).unwrap();
    let mcp = files.iter().find(|f| f.rel_path == ".mcp.json").unwrap();
    assert!(mcp.content.contains("\"cmd\""));
    assert!(mcp.content.contains("\"/c\""));
    assert!(
        mcp.content.contains("C:\\\\Users\\\\alice/repo")
            || mcp.content.contains("C:\\Users\\alice")
    );
}

#[test]
fn sanitizes_rule_names() {
    assert_eq!(ALL_TARGETS.len(), 5);
    assert_eq!(sanitize_rule_name("my-rule_1").unwrap(), "my-rule_1");
    assert_eq!(sanitize_rule_name("../evil").unwrap(), "evil");
    assert!(sanitize_rule_name("...").is_err());
}

#[test]
fn unify_deduplicates_resolves_conflicts_merges_mcp_and_backs_up() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    fs::write(
        root.join("CLAUDE.md"),
        "- Never commit .env files.\n- Always use pnpm for dependencies.\n- Only Claude special rule.\n",
    )
    .unwrap();
    fs::write(
        root.join("AGENTS.md"),
        "- Never commit .env files.\n- Always use npm for dependencies.\n",
    )
    .unwrap();
    fs::write(
        root.join(".mcp.json"),
        r#"{"mcpServers":{"github":{"command":"npx","args":["-y","gh"]}}}"#,
    )
    .unwrap();
    fs::create_dir_all(root.join(".cursor")).unwrap();
    fs::write(
        root.join(".cursor").join("mcp.json"),
        r#"{"mcpServers":{"postgres":{"command":"npx","args":["-y","pg"]}}}"#,
    )
    .unwrap();

    // Simulate user choosing option 1 for conflict (pnpm), and 's' (keep scoped) for unique Claude rule
    let input = b"1\ns\n";
    let mut reader = Cursor::new(&input[..]);
    let mut output = Vec::new();
    let summary = run_unify(root, false, &mut reader, &mut output).unwrap();

    assert_eq!(summary.deduplicated, 1);
    assert_eq!(summary.conflicts_resolved, 1);
    assert_eq!(summary.mcp_merged, 2);
    assert!(summary.backup_dir.ends_with("backup"));
    assert!(root.join(".dotagents/backup/CLAUDE.md").is_file());
    assert!(root.join(".dotagents/rules/unified.md").is_file());
    assert!(root.join(".dotagents/rules/agent-claude.md").is_file());

    let files =
        compile_targets(root, &["all".to_string()], &PathBuf::from("/home/u"), false).unwrap();
    write_generated(root, &files).unwrap();

    let agents_md = fs::read_to_string(root.join("AGENTS.md")).unwrap();
    let claude_md = fs::read_to_string(root.join("CLAUDE.md")).unwrap();
    assert!(agents_md.contains("pnpm"));
    assert!(!agents_md.contains("use npm"));
    assert!(claude_md.contains("Only Claude special rule"));
    assert!(!agents_md.contains("Only Claude special rule"));
}

#[test]
fn smart_sync_absorbs_newer_direct_agent_edits() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let home = PathBuf::from("/home/u");
    let targets = vec!["all".to_string()];

    init_workspace(root).unwrap();
    let files = compile_targets(root, &targets, &home, false).unwrap();
    write_generated(root, &files).unwrap();

    // Up-to-date check
    let o1 = smart_sync(root, &targets, &home, false, None).unwrap();
    assert_eq!(o1.mode, "up-to-date");

    // Simulate Claude editing CLAUDE.md directly after .dotagents/ was created
    thread::sleep(Duration::from_millis(50));
    fs::write(
        root.join("CLAUDE.md"),
        "- Keep functions small and well-tested.\n- Always run cargo clippy before finishing.\n",
    )
    .unwrap();

    let o2 = smart_sync(root, &targets, &home, false, None).unwrap();
    assert_eq!(o2.mode, "absorbed");
    assert_eq!(o2.newest_source.as_deref(), Some("CLAUDE.md"));
    assert!(o2.files_written > 0);

    // Verify GEMINI.md and AGENTS.md received the new rule
    let gemini = fs::read_to_string(root.join("GEMINI.md")).unwrap();
    let agents = fs::read_to_string(root.join("AGENTS.md")).unwrap();
    assert!(gemini.contains("Always run cargo clippy before finishing."));
    assert!(agents.contains("Always run cargo clippy before finishing."));
}
