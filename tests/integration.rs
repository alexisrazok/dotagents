#[path = "../src/compiler.rs"]
mod compiler;

use compiler::{
    check_drift, compile_targets, import_existing, init_workspace, sanitize_rule_name,
    write_generated, ALL_TARGETS,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

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
