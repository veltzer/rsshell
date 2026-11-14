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

Single-binary Rust CLI implementing a Unix shell. Library modules:

- **`src/lib.rs`** - Module declarations (`commands`, `helpers`, `ssh`, `ssh_config`).
- **`src/main.rs`** - CLI definition using `clap` derive. Defines `Commands` enum for subcommands (init-config, version, complete). Interactive REPL loop with rustyline.
- **`src/commands.rs`** - Command execution: built-in commands (cd, exit, export, unset, source, pwd, echo, type, history, ssh), external command spawning, pipeline execution, alias expansion, and variable assignment.
- **`src/helpers.rs`** - Configuration loading (TOML), prompt building, command-line parsing (quote/escape handling), tilde expansion, environment variable expansion, glob expansion, and pipe splitting.
- **`src/ssh.rs`** - Built-in `ssh` command: russh-based SSH client. Connects over TCP, through a `ProxyCommand` child process (its stdio is the SSH transport), or through a `ProxyJump` hop (`direct-tcpip`). Authentication: identity file (passphrase prompt), ssh-agent, default keys, interactive password. Interactive PTY sessions (raw mode via crossterm) and one-shot command execution; known_hosts verification and learning.
- **`src/ssh_config.rs`** - OpenSSH `~/.ssh/config` parser: Host pattern blocks (with `*`, `?`, `!` negation), first-value-wins semantics, Include directives, and the `%h`/`%p`/`%r`/`%%` token expansion used by ProxyCommand.

## Key Conventions

- Config file at `~/.config/rsshell/config.toml` with sections: prompt, aliases, env, startup.
- History stored at `~/.config/rsshell/history.txt`.
- Built-in commands return i32 exit codes. External commands return process exit code or 127 if not found.
- Pipeline execution spawns child processes connected via stdin/stdout pipes.
- Shell supports: quoting (single/double), escape sequences, tilde expansion, env var expansion ($VAR, ${VAR}), glob expansion, pipes, aliases, local variables, and command history.
- The `ssh` builtin follows OpenSSH CLI conventions: options precede the destination, everything after it is the remote command; failures exit 255.
