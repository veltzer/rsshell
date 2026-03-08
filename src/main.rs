use std::collections::HashMap;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use rustyline::config::Builder as RlBuilder;
use rustyline::history::DefaultHistory;
use rustyline::Editor;
use rustyline::error::ReadlineError;

use rsshell::commands;
use rsshell::helpers::{build_prompt, expand_history, history_path, load_config};

/// rsshell - A simple Unix shell written in Rust
#[derive(Parser)]
#[command(name = "rsshell", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Execute a command string and exit
    #[arg(short = 'c', long)]
    command_string: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a default configuration file
    InitConfig,
    /// Show version and build metadata
    Version,
    /// Generate shell completions
    Complete {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::InitConfig) => {
            std::process::exit(commands::cmd_init_config());
        }
        Some(Commands::Version) => {
            commands::cmd_version();
        }
        Some(Commands::Complete { shell }) => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "rsshell", &mut std::io::stdout());
        }
        None => {
            if let Some(cmd_str) = cli.command_string {
                // Non-interactive: run single command
                let config = load_config();
                let mut vars = HashMap::new();
                let code = commands::execute_line(&cmd_str, &config, &mut vars);
                std::process::exit(code);
            }
            // Interactive shell
            run_interactive();
        }
    }
}

fn run_interactive() {
    let config = load_config();

    // Apply env vars from config
    for (key, value) in &config.env {
        // SAFETY: done before spawning any threads
        unsafe { std::env::set_var(key, value); }
    }

    // Run startup commands
    commands::run_startup_commands(&config);

    let rl_config = RlBuilder::new()
        .max_history_size(config.history.max_entries)
        .expect("invalid max_history_size")
        .auto_add_history(false)
        .build();
    let mut editor = Editor::<(), DefaultHistory>::with_config(rl_config)
        .expect("failed to initialize line editor");

    // Load history
    let hist_path = history_path();
    if hist_path.exists() {
        let _ = editor.load_history(&hist_path);
    }

    let mut last_exit_code: i32 = 0;
    let mut local_vars: HashMap<String, String> = HashMap::new();

    loop {
        let prompt = build_prompt(&config, last_exit_code);
        match editor.readline(&prompt) {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // Skip entries starting with space if configured
                if config.history.ignore_space && trimmed.starts_with(' ') {
                    last_exit_code = commands::execute_line(trimmed, &config, &mut local_vars);
                    continue;
                }

                // Expand history references (!! !n !-n !prefix)
                let history_entries: Vec<&str> = editor
                    .history()
                    .iter()
                    .map(|s| s.as_str())
                    .collect();
                let expanded = match expand_history(trimmed, &history_entries) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("rsshell: {e}");
                        last_exit_code = 1;
                        continue;
                    }
                };

                // If expansion changed the line, show what will run
                if expanded != trimmed {
                    println!("{expanded}");
                }

                // Add to history (skip consecutive duplicates if configured)
                let should_add = if config.history.ignore_duplicates {
                    editor
                        .history()
                        .iter()
                        .last()
                        .is_none_or(|last| last != expanded.as_str())
                } else {
                    true
                };
                if should_add {
                    let _ = editor.add_history_entry(expanded.as_str());
                }

                last_exit_code = commands::execute_line(&expanded, &config, &mut local_vars);
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl-C: just print a new prompt
                continue;
            }
            Err(ReadlineError::Eof) => {
                // Ctrl-D: exit
                break;
            }
            Err(err) => {
                eprintln!("rsshell: readline error: {err}");
                break;
            }
        }
    }

    // Save history
    if let Some(parent) = hist_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = editor.save_history(&hist_path);
}
