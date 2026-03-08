# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build                          # Debug build
cargo build --release                # Release build (stripped, LTO)
cargo nextest run                    # Run tests (preferred over cargo test)
cargo nextest run --release          # Run tests in release mode
cargo nextest run -E 'test(test_name)'  # Run a single test
cargo clippy                         # Lint
```

Requires Rust edition 2024. Uses `build.rs` to embed git metadata (SHA, branch, describe, dirty) and rustc version at compile time.

## Architecture

Single-binary Rust CLI implementing a Unix shell. Three source files:

- **`src/main.rs`** - CLI definition using `clap` derive. Defines `Commands` enum for subcommands (init-config, version, complete). Interactive REPL loop with rustyline. Unit tests in `mod tests`.
- **`src/commands.rs`** - Command execution: built-in commands (cd, exit, export, unset, source, pwd, echo, type, history), external command spawning, pipeline execution, alias expansion, and variable assignment.
- **`src/helpers.rs`** - Configuration loading (TOML), prompt building, command-line parsing (quote/escape handling), tilde expansion, environment variable expansion, glob expansion, and pipe splitting.

## Key Conventions

- Config file at `~/.config/rsshell/config.toml` with sections: prompt, aliases, env, startup.
- History stored at `~/.config/rsshell/history.txt`.
- Built-in commands return i32 exit codes. External commands return process exit code or 127 if not found.
- Pipeline execution spawns child processes connected via stdin/stdout pipes.
- Shell supports: quoting (single/double), escape sequences, tilde expansion, env var expansion ($VAR, ${VAR}), glob expansion, pipes, aliases, local variables, and command history.
