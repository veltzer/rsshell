# Introduction

**rsshell** is a simple Unix shell written in Rust. It provides an interactive command-line interface with common shell features like pipes, aliases, environment variable expansion, glob patterns, and command history.

## Features

- Interactive REPL with line editing (rustyline)
- Command history with persistence
- Pipes (`cmd1 | cmd2 | cmd3`)
- Built-in commands: `cd`, `pwd`, `echo`, `export`, `unset`, `source`, `type`, `history`, `exit`
- Alias expansion
- Environment variable expansion (`$VAR`, `${VAR}`)
- Tilde expansion (`~/path`)
- Glob pattern expansion (`*.txt`, `src/**/*.rs`)
- Quoting (single and double quotes)
- Escape sequences
- TOML-based configuration
- Shell completions generation

## Tech Stack

- **Language:** Rust (edition 2024)
- **CLI:** clap with derive macros
- **Line editing:** rustyline
- **Configuration:** TOML via serde
- **Unix APIs:** nix crate
- **Glob expansion:** glob crate
