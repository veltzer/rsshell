# Platform Limitations

rsshell is designed as a **Unix shell** and currently only compiles and runs on Unix-like systems (Linux, macOS). This page documents all the issues that prevent Windows compilation.

## Summary

| Category | Count | Severity |
|----------|-------|----------|
| Unix-only crate dependency | 1 | Blocker |
| Unix system calls | 1 | Blocker |
| Unix signal handling | 2 | Blocker |
| Unix-specific imports | 1 | Blocker |
| PATH separator assumptions | 1 | Runtime bug |
| Path separator assumptions | 3 | Runtime bug |
| Unix `date` command usage | 2 | Runtime failure |

## Detailed Issues

### 1. The `nix` crate (Blocker)

**File:** `Cargo.toml`

```toml
nix = { version = "0.29", features = ["signal", "process", "fs", "hostname"] }
```

The [nix](https://crates.io/crates/nix) crate provides Rust-friendly bindings to Unix system APIs. It does not compile on Windows at all. It is used for:

- Getting the system hostname via `gethostname()`

**Possible fix:** Replace with the [hostname](https://crates.io/crates/hostname) crate or use `std::env::var("COMPUTERNAME")` on Windows.

### 2. `std::os::unix` import (Blocker)

**File:** `src/commands.rs`

```rust
use std::os::unix::process::ExitStatusExt;
```

The `std::os::unix` module does not exist on Windows. The `ExitStatusExt` trait provides the `.signal()` method used to detect if a process was killed by a signal.

**Possible fix:** Use `#[cfg(unix)]` conditional compilation and provide a Windows fallback using `std::os::windows::process::ExitStatusExt`.

### 3. Unix signal handling (Blocker)

**File:** `src/commands.rs` (two locations)

```rust
// In execute_pipeline()
status.signal().map(|s| 128 + s).unwrap_or(1)

// In cmd_external()
status.signal().map(|s| 128 + s).unwrap_or(1)
```

Unix processes can be terminated by signals (SIGTERM, SIGKILL, etc.), and the convention is to report exit code `128 + signal_number`. Windows does not have Unix signals.

**Possible fix:** Wrap in `#[cfg(unix)]` and use a simpler fallback on Windows (e.g., just return the exit code or 1).

### 4. `gethostname()` system call

**File:** `src/helpers.rs`

```rust
nix::unistd::gethostname()
```

This is a POSIX system call not available on Windows.

**Possible fix:** Use the cross-platform [hostname](https://crates.io/crates/hostname) crate, or `std::env::var("COMPUTERNAME")` on Windows.

### 5. PATH separator hardcoded as `:`

**File:** `src/commands.rs`

```rust
for dir in path_var.split(':') {
```

Unix uses `:` to separate entries in `$PATH`, while Windows uses `;`. This would cause the `type` builtin to fail to find any commands on Windows.

**Possible fix:** Use `std::env::split_paths()` which handles this cross-platform.

### 6. Forward slash as path separator

**File:** `src/helpers.rs` (multiple locations)

```rust
// Extracting cwd basename
let cwd_basename = cwd.rsplit('/').next().unwrap_or(&cwd).to_owned();

// Extracting git repo name
.rsplit('/')
```

These assume `/` as the path separator. On Windows, paths use `\`.

**Possible fix:** Use `std::path::Path::file_name()` or `std::path::MAIN_SEPARATOR`.

### 7. Unix `date` command

**File:** `src/helpers.rs` and `build.rs`

```rust
Command::new("date").arg("+%Y-%m-%d %H:%M:%S")
```

The `date` command with `+` format specifiers is Unix-specific. Windows does not have this command.

**Possible fix:** Use the [chrono](https://crates.io/crates/chrono) crate for cross-platform date/time formatting, or use `std::time::SystemTime` with manual formatting.

### 8. Git path references in `build.rs`

**File:** `build.rs`

```rust
println!("cargo:rerun-if-changed=.git/HEAD");
std::fs::read_to_string(".git/HEAD")
let loose = format!(".git/{refpath}");
```

While forward slashes generally work on Windows for file access, these hardcoded paths may fail with non-standard git configurations (e.g., git worktrees, `GIT_DIR`).

**Possible fix:** Use `std::path::PathBuf` to construct paths portably.

## Why Not Fix These?

rsshell is fundamentally a **Unix shell** — it spawns Unix processes, uses Unix pipes, and implements Unix shell semantics. While the compilation blockers above could be fixed with conditional compilation, the shell itself would have limited usefulness on Windows since:

- The built-in commands (`cd`, `pwd`, `source`, etc.) follow Unix conventions
- External command execution assumes Unix process semantics
- Pipeline implementation uses Unix pipe mechanisms
- Users on Windows who want a Unix-like shell experience typically use WSL, Git Bash, or similar environments

For these reasons, the Windows target has been intentionally removed from the release workflow.
