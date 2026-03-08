use std::collections::HashMap;
use std::env;
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::helpers::{
    self, Config, DEFAULT_CONFIG, config_dir, config_path, expand_env_vars, expand_globs,
    expand_tilde, history_path, parse_command_line, split_pipes,
};

/// Run startup commands from config.
pub fn run_startup_commands(config: &Config) {
    for cmd in &config.startup.commands {
        let trimmed = cmd.trim();
        if !trimmed.is_empty() {
            execute_line(trimmed, config, &mut HashMap::new());
        }
    }
}

/// Execute a full input line (may contain pipes).
/// Returns the exit code of the last command.
pub fn execute_line(
    line: &str,
    config: &Config,
    local_vars: &mut HashMap<String, String>,
) -> i32 {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return 0;
    }

    // Handle variable assignment: VAR=value
    if let Some((name, value)) = parse_variable_assignment(line) {
        local_vars.insert(name, value);
        return 0;
    }

    // Expand environment variables
    let line = expand_env_vars(line);

    // Expand local variables
    let line = expand_local_vars(&line, local_vars);

    // Check for alias expansion
    let line = expand_aliases(&line, config);

    let pipe_segments = split_pipes(&line);

    if pipe_segments.len() == 1 {
        let args = parse_command_line(&pipe_segments[0]);
        if args.is_empty() {
            return 0;
        }
        execute_command(&args, config)
    } else {
        execute_pipeline(&pipe_segments, config)
    }
}

/// Parse a variable assignment like VAR=value.
fn parse_variable_assignment(line: &str) -> Option<(String, String)> {
    if let Some(eq_pos) = line.find('=') {
        let name = &line[..eq_pos];
        // Variable names must be alphanumeric/underscore and not empty
        if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            let value = &line[eq_pos + 1..];
            return Some((name.to_owned(), value.to_owned()));
        }
    }
    None
}

/// Expand local shell variables.
fn expand_local_vars(input: &str, vars: &HashMap<String, String>) -> String {
    let mut result = input.to_owned();
    for (name, value) in vars {
        result = result.replace(&format!("${name}"), value);
        result = result.replace(&format!("${{{name}}}"), value);
    }
    result
}

/// Expand aliases (first word only, non-recursive).
fn expand_aliases(line: &str, config: &Config) -> String {
    let trimmed = line.trim();
    let first_space = trimmed.find(' ');
    let first_word = match first_space {
        Some(pos) => &trimmed[..pos],
        None => trimmed,
    };

    if let Some(expansion) = config.aliases.get(first_word) {
        match first_space {
            Some(pos) => format!("{expansion}{}", &trimmed[pos..]),
            None => expansion.clone(),
        }
    } else {
        line.to_owned()
    }
}

/// Execute a single command (no pipes). Returns exit code.
fn execute_command(args: &[String], _config: &Config) -> i32 {
    if args.is_empty() {
        return 0;
    }

    let cmd = &args[0];

    // Built-in commands
    match cmd.as_str() {
        "cd" => cmd_cd(args),
        "exit" | "quit" => cmd_exit(args),
        "export" => cmd_export(args),
        "unset" => cmd_unset(args),
        "source" | "." => cmd_source(args),
        "pwd" => cmd_pwd(),
        "echo" => cmd_echo(args),
        "type" => cmd_type(args),
        "history" => cmd_history(),
        _ => cmd_external(args),
    }
}

/// Execute a pipeline of commands.
fn execute_pipeline(segments: &[String], config: &Config) -> i32 {
    if segments.is_empty() {
        return 0;
    }

    let mut children = Vec::new();
    let mut prev_stdout: Option<Stdio> = None;

    for (i, segment) in segments.iter().enumerate() {
        let args = parse_command_line(segment);
        if args.is_empty() {
            continue;
        }

        let args = expand_globs(&args);
        let args: Vec<String> = args.iter().map(|a| expand_tilde(a)).collect();

        // Check for builtins in pipeline - they need special handling
        let is_last = i == segments.len() - 1;

        let stdin = prev_stdout.take().unwrap_or(Stdio::inherit());
        let stdout = if is_last {
            Stdio::inherit()
        } else {
            Stdio::piped()
        };

        // For builtins in a pipeline, we'd need to fork. For simplicity,
        // only handle external commands in pipelines.
        let _ = config;
        match Command::new(&args[0])
            .args(&args[1..])
            .stdin(stdin)
            .stdout(stdout)
            .spawn()
        {
            Ok(mut child) => {
                if !is_last
                    && let Some(child_stdout) = child.stdout.take()
                {
                    prev_stdout = Some(Stdio::from(child_stdout));
                }
                children.push(child);
            }
            Err(e) => {
                eprintln!("rsshell: {}: {e}", args[0]);
                return 127;
            }
        }
    }

    // Wait for all children, return the last exit code
    let mut last_code = 0;
    for mut child in children {
        match child.wait() {
            Ok(status) => {
                last_code = status.code().unwrap_or_else(|| {
                    // If killed by signal, return 128 + signal number
                    status.signal().map(|s| 128 + s).unwrap_or(1)
                });
            }
            Err(e) => {
                eprintln!("rsshell: wait error: {e}");
                last_code = 1;
            }
        }
    }
    last_code
}

// ───── Built-in commands ─────

fn cmd_cd(args: &[String]) -> i32 {
    let target = if args.len() < 2 {
        dirs::home_dir()
            .map(|h| h.display().to_string())
            .unwrap_or_else(|| ".".to_owned())
    } else {
        expand_tilde(&args[1])
    };

    match env::set_current_dir(&target) {
        Ok(()) => {
            // Update PWD
            if let Ok(cwd) = env::current_dir() {
                // SAFETY: rsshell is single-threaded at this point
                unsafe { env::set_var("PWD", cwd); }
            }
            0
        }
        Err(e) => {
            eprintln!("cd: {target}: {e}");
            1
        }
    }
}

fn cmd_exit(args: &[String]) -> i32 {
    let code = if args.len() > 1 {
        args[1].parse::<i32>().unwrap_or(0)
    } else {
        0
    };
    std::process::exit(code);
}

fn cmd_export(args: &[String]) -> i32 {
    for arg in &args[1..] {
        if let Some(eq_pos) = arg.find('=') {
            let name = &arg[..eq_pos];
            let value = &arg[eq_pos + 1..];
            // SAFETY: rsshell is single-threaded at this point
            unsafe { env::set_var(name, value); }
        } else {
            // export without = just marks it (already in env)
        }
    }
    0
}

fn cmd_unset(args: &[String]) -> i32 {
    for arg in &args[1..] {
        // SAFETY: rsshell is single-threaded at this point
        unsafe { env::remove_var(arg); }
    }
    0
}

fn cmd_source(args: &[String]) -> i32 {
    if args.len() < 2 {
        eprintln!("source: filename argument required");
        return 1;
    }
    let path = expand_tilde(&args[1]);
    match fs::read_to_string(&path) {
        Ok(contents) => {
            let config = helpers::load_config();
            let mut vars = HashMap::new();
            let mut last_code = 0;
            for line in contents.lines() {
                last_code = execute_line(line, &config, &mut vars);
            }
            last_code
        }
        Err(e) => {
            eprintln!("source: {path}: {e}");
            1
        }
    }
}

fn cmd_pwd() -> i32 {
    match env::current_dir() {
        Ok(dir) => {
            println!("{}", dir.display());
            0
        }
        Err(e) => {
            eprintln!("pwd: {e}");
            1
        }
    }
}

fn cmd_echo(args: &[String]) -> i32 {
    let output = args[1..].join(" ");
    println!("{output}");
    0
}

fn cmd_type(args: &[String]) -> i32 {
    let builtins = [
        "cd", "exit", "quit", "export", "unset", "source", ".", "pwd", "echo", "type", "history",
    ];

    let mut code = 0;
    for arg in &args[1..] {
        if builtins.contains(&arg.as_str()) {
            println!("{arg} is a shell builtin");
        } else if let Ok(path_var) = env::var("PATH") {
            let mut found = false;
            for dir in path_var.split(':') {
                let full = Path::new(dir).join(arg);
                if full.exists() {
                    println!("{arg} is {}", full.display());
                    found = true;
                    break;
                }
            }
            if !found {
                eprintln!("type: {arg}: not found");
                code = 1;
            }
        } else {
            eprintln!("type: {arg}: not found");
            code = 1;
        }
    }
    code
}

fn cmd_history() -> i32 {
    let path = history_path();
    match fs::read_to_string(&path) {
        Ok(contents) => {
            for (i, line) in contents.lines().enumerate() {
                println!("{:>5}  {line}", i + 1);
            }
            0
        }
        Err(_) => {
            // No history file yet
            0
        }
    }
}

/// Execute an external command.
fn cmd_external(args: &[String]) -> i32 {
    let args = expand_globs(args);
    let args: Vec<String> = args.iter().map(|a| expand_tilde(a)).collect();

    match Command::new(&args[0]).args(&args[1..]).spawn() {
        Ok(mut child) => match child.wait() {
            Ok(status) => status.code().unwrap_or_else(|| {
                status.signal().map(|s| 128 + s).unwrap_or(1)
            }),
            Err(e) => {
                eprintln!("rsshell: wait error: {e}");
                1
            }
        },
        Err(e) => {
            eprintln!("rsshell: {}: {e}", args[0]);
            127
        }
    }
}

/// Initialize the config file.
pub fn cmd_init_config() -> i32 {
    let dir = config_dir();
    if let Err(e) = fs::create_dir_all(&dir) {
        eprintln!("rsshell: cannot create config directory: {e}");
        return 1;
    }
    let path = config_path();
    if path.exists() {
        eprintln!("rsshell: config file already exists at {}", path.display());
        return 1;
    }
    match fs::write(&path, DEFAULT_CONFIG) {
        Ok(()) => {
            println!("Config file created at {}", path.display());
            0
        }
        Err(e) => {
            eprintln!("rsshell: cannot write config file: {e}");
            1
        }
    }
}

/// Print version information.
pub fn cmd_version() {
    println!("rsshell {}", env!("CARGO_PKG_VERSION"));
    println!("  git sha:     {}", env!("GIT_SHA"));
    println!("  git branch:  {}", env!("GIT_BRANCH"));
    println!("  git describe:{}", env!("GIT_DESCRIBE"));
    println!("  git dirty:   {}", env!("GIT_DIRTY"));
    println!("  rustc:       {}", env!("RUSTC_SEMVER"));
    println!("  edition:     {}", env!("RUST_EDITION"));
    println!("  built:       {}", env!("BUILD_TIMESTAMP"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_variable_assignment() {
        assert_eq!(
            parse_variable_assignment("FOO=bar"),
            Some(("FOO".to_owned(), "bar".to_owned()))
        );
        assert_eq!(
            parse_variable_assignment("MY_VAR=hello world"),
            Some(("MY_VAR".to_owned(), "hello world".to_owned()))
        );
        assert_eq!(parse_variable_assignment("ls -la"), None);
        assert_eq!(parse_variable_assignment("=value"), None);
    }

    #[test]
    fn test_expand_aliases() {
        let mut config = Config::default();
        config.aliases.insert("ll".to_owned(), "ls -la".to_owned());
        assert_eq!(expand_aliases("ll /tmp", &config), "ls -la /tmp");
        assert_eq!(expand_aliases("ll", &config), "ls -la");
        assert_eq!(expand_aliases("ls /tmp", &config), "ls /tmp");
    }

    #[test]
    fn test_expand_local_vars() {
        let mut vars = HashMap::new();
        vars.insert("name".to_owned(), "world".to_owned());
        assert_eq!(expand_local_vars("hello $name", &vars), "hello world");
        assert_eq!(expand_local_vars("hello ${name}", &vars), "hello world");
    }
}
