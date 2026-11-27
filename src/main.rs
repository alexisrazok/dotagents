mod compiler;
mod unify;

use clap::{Parser, Subcommand};
use compiler::{
    check_drift, compile_targets, import_existing, init_workspace, write_generated, ALL_TARGETS,
};
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;
use unify::run_unify;

#[derive(Parser, Debug)]
#[command(
    name = "dotagents",
    version,
    about = "Universal compiler and syncer for AI coding agent rules and MCP configs."
)]
struct Cli {
    #[arg(long, default_value = ".")]
    root: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize .dotagents/ directory with starter rules and mcp.json
    Init,
    /// Import existing CLAUDE.md, AGENTS.md, GEMINI.md, .cursorrules, and .mcp.json
    Import,
    /// Interactive wizard to merge, deduplicate, and resolve conflicts across existing agent configs
    Unify {
        #[arg(long, default_value = "all")]
        targets: String,
        #[arg(long)]
        auto: bool,
    },
    /// Compile .dotagents/ into native files for each target agent
    Generate {
        #[arg(long, default_value = "all")]
        targets: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        check: bool,
    },
    /// Show sync/drift status of generated target files
    Status {
        #[arg(long, default_value = "all")]
        targets: String,
    },
}

fn parse_targets(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn run(cli: Cli) -> Result<i32, String> {
    let root = std::fs::canonicalize(&cli.root).unwrap_or(cli.root);
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let is_windows = cfg!(windows);

    match cli.command {
        Commands::Init => {
            let dir = init_workspace(&root)?;
            println!("Initialized {}", dir.display());
            Ok(0)
        }
        Commands::Import => {
            let count = import_existing(&root)?;
            println!("Imported {} existing files into .dotagents/", count);
            Ok(0)
        }
        Commands::Unify { targets, auto } => {
            let stdin = io::stdin();
            let stdout = io::stdout();
            let mut reader = stdin.lock();
            let mut writer = stdout.lock();
            let summary = run_unify(&root, auto, &mut reader, &mut writer)?;
            let tlist = parse_targets(&targets);
            let files = compile_targets(&root, &tlist, &home, is_windows)?;
            let written = write_generated(&root, &files)?;
            println!(
                "Backup guardado en: {}\nRegenerados {} archivos sincronizados.",
                summary.backup_dir, written
            );
            Ok(0)
        }
        Commands::Generate {
            targets,
            dry_run,
            check,
        } => {
            let tlist = parse_targets(&targets);
            let files = compile_targets(&root, &tlist, &home, is_windows)?;
            if check {
                let drifted = check_drift(&root, &files);
                if drifted.is_empty() {
                    println!("All {} generated files are up to date.", files.len());
                    return Ok(0);
                }
                eprintln!("Drift detected in:");
                for d in drifted {
                    eprintln!("  - {}", d);
                }
                return Ok(1);
            }
            if dry_run {
                println!("Dry run ({} files):", files.len());
                for f in &files {
                    println!("  -> {}", f.rel_path);
                }
                return Ok(0);
            }
            let n = write_generated(&root, &files)?;
            println!("Generated {} files for targets: {}", n, targets);
            Ok(0)
        }
        Commands::Status { targets } => {
            let tlist = parse_targets(&targets);
            let files = compile_targets(&root, &tlist, &home, is_windows)?;
            let drifted = check_drift(&root, &files);
            println!("Supported targets: {}", ALL_TARGETS.join(", "));
            println!(
                "Generated files: {}, Drifted: {}",
                files.len(),
                drifted.len()
            );
            for d in &drifted {
                println!("  [DRIFT] {}", d);
            }
            Ok(0)
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => ExitCode::from(code as u8),
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::from(1)
        }
    }
}
