mod commands;
mod helpers;

use std::collections::HashMap;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;

use crate::helpers::{build_prompt, history_path, load_config};

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

    let mut editor = DefaultEditor::new().expect("failed to initialize line editor");

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
                let _ = editor.add_history_entry(trimmed);
                last_exit_code = commands::execute_line(trimmed, &config, &mut local_vars);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse() {
        // Verify CLI parses without error
        let cli = Cli::try_parse_from(["rsshell", "version"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_command_string() {
        let cli = Cli::try_parse_from(["rsshell", "-c", "echo hello"]).unwrap();
        assert_eq!(cli.command_string, Some("echo hello".to_owned()));
    }

    #[test]
    fn test_cli_init_config() {
        let cli = Cli::try_parse_from(["rsshell", "init-config"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::InitConfig)));
    }
}
