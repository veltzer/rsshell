use std::collections::HashMap;
use std::env;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use std::process::Stdio;

use serde::Deserialize;

/// Shell configuration loaded from config file.
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub prompt: PromptConfig,
    #[serde(default)]
    pub history: HistoryConfig,
    #[serde(default)]
    pub aliases: HashMap<String, String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub startup: StartupConfig,
}

#[derive(Debug, Deserialize)]
pub struct PromptConfig {
    #[serde(default = "default_prompt_parts")]
    pub parts: Vec<PromptPart>,
    #[serde(default)]
    pub show_exit_code: bool,
    #[serde(default = "default_exit_code_color")]
    pub exit_code_color: String,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            parts: default_prompt_parts(),
            show_exit_code: false,
            exit_code_color: default_exit_code_color(),
        }
    }
}

fn default_prompt_parts() -> Vec<PromptPart> {
    vec![
        PromptPart { text: "{user}".to_owned(), color: "green".to_owned(), bold: false },
        PromptPart { text: "@".to_owned(), color: "none".to_owned(), bold: false },
        PromptPart { text: "{host}".to_owned(), color: "green".to_owned(), bold: false },
        PromptPart { text: ":".to_owned(), color: "none".to_owned(), bold: false },
        PromptPart { text: "{cwd}".to_owned(), color: "blue".to_owned(), bold: true },
        PromptPart { text: "$ ".to_owned(), color: "none".to_owned(), bold: false },
    ]
}

fn default_exit_code_color() -> String {
    "red".to_owned()
}

#[derive(Debug, Deserialize, Clone)]
pub struct PromptPart {
    pub text: String,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default)]
    pub bold: bool,
}

fn default_color() -> String {
    "none".to_owned()
}

#[derive(Debug, Deserialize)]
pub struct HistoryConfig {
    #[serde(default = "default_max_entries")]
    pub max_entries: usize,
    #[serde(default = "default_true")]
    pub ignore_duplicates: bool,
    #[serde(default = "default_true")]
    pub ignore_space: bool,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            max_entries: default_max_entries(),
            ignore_duplicates: true,
            ignore_space: true,
        }
    }
}

fn default_max_entries() -> usize {
    10000
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, Default)]
pub struct StartupConfig {
    #[serde(default)]
    pub commands: Vec<String>,
}

/// Default configuration file contents.
pub const DEFAULT_CONFIG: &str = r#"# rsshell configuration file

[prompt]
# Show the exit code of the last command when non-zero
show_exit_code = true
# Color for the exit code indicator (when non-zero)
exit_code_color = "red"

# Prompt is built from an ordered list of parts.
# Each part has:
#   text  - literal text or a variable: {user}, {host}, {cwd}, {git_branch}
#   color - one of: none, black, red, green, yellow, blue, magenta, cyan, white
#   bold  - true/false (default: false)

[[prompt.parts]]
text = "{user}"
color = "green"

[[prompt.parts]]
text = "@"

[[prompt.parts]]
text = "{host}"
color = "green"

[[prompt.parts]]
text = ":"

[[prompt.parts]]
text = "{cwd}"
color = "blue"
bold = true

[[prompt.parts]]
text = "$ "

[history]
# Maximum number of history entries to keep
max_entries = 10000
# Skip duplicate consecutive entries
ignore_duplicates = true
# Skip entries that start with a space
ignore_space = true

[aliases]
# Define command aliases
# ll = "ls -la"
# la = "ls -A"
# gs = "git status"

[env]
# Set environment variables
# EDITOR = "vim"

[startup]
# Commands to run at shell startup
# commands = ["echo 'Welcome to rsshell!'"]
"#;

/// Return the path to the config directory: ~/.config/rsshell/
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rsshell")
}

/// Return the path to the config file: ~/.config/rsshell/config.toml
pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

/// Return the path to the history file: ~/.config/rsshell/history.txt
pub fn history_path() -> PathBuf {
    config_dir().join("history.txt")
}

/// Load config from disk, returning default if not found.
pub fn load_config() -> Config {
    let path = config_path();
    if path.exists() {
        let contents = std::fs::read_to_string(&path).unwrap_or_default();
        toml::from_str(&contents).unwrap_or_default()
    } else {
        Config::default()
    }
}

/// Map a color name to its ANSI code.
fn color_code(name: &str) -> Option<&'static str> {
    match name {
        "black" => Some("30"),
        "red" => Some("31"),
        "green" => Some("32"),
        "yellow" => Some("33"),
        "blue" => Some("34"),
        "magenta" => Some("35"),
        "cyan" => Some("36"),
        "white" => Some("37"),
        _ => None,
    }
}

/// Wrap text in ANSI color/bold escape sequences.
fn colorize(text: &str, color: &str, bold: bool) -> String {
    let code = color_code(color);
    if code.is_none() && !bold {
        return text.to_owned();
    }

    let mut params = Vec::new();
    if bold {
        params.push("1");
    }
    if let Some(c) = code {
        params.push(c);
    }
    let seq = params.join(";");
    format!("\x1b[{seq}m{text}\x1b[0m")
}

/// Expand prompt variables in a text string.
fn expand_prompt_vars(text: &str, user: &str, host: &str, cwd: &str, git_branch: &str) -> String {
    text.replace("{user}", user)
        .replace("{host}", host)
        .replace("{cwd}", cwd)
        .replace("{git_branch}", git_branch)
}

/// Detect the current git branch, or return an empty string if not in a repo.
fn git_branch() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_default()
}

/// Build the prompt string from the configured parts.
pub fn build_prompt(config: &Config, last_exit_code: i32) -> String {
    let user = env::var("USER").unwrap_or_else(|_| "user".to_owned());
    let host = hostname();
    let cwd = env::current_dir()
        .map(|p| {
            let home = dirs::home_dir().unwrap_or_default();
            if let Ok(rest) = p.strip_prefix(&home) {
                format!("~/{}", rest.display())
                    .trim_end_matches('/')
                    .to_owned()
            } else {
                p.display().to_string()
            }
        })
        .unwrap_or_else(|_| "?".to_owned());
    let cwd = if cwd == "~/" { "~".to_owned() } else { cwd };
    let branch = git_branch();

    let mut prompt = String::new();

    if config.prompt.show_exit_code && last_exit_code != 0 {
        let code_text = format!("[{last_exit_code}] ");
        prompt.push_str(&colorize(&code_text, &config.prompt.exit_code_color, true));
    }

    for part in &config.prompt.parts {
        let expanded = expand_prompt_vars(&part.text, &user, &host, &cwd, &branch);
        prompt.push_str(&colorize(&expanded, &part.color, part.bold));
    }

    prompt
}

/// Get the system hostname.
fn hostname() -> String {
    nix::unistd::gethostname()
        .ok()
        .and_then(|h: std::ffi::OsString| h.into_string().ok())
        .unwrap_or_else(|| "localhost".to_owned())
}

/// Expand history references in the input line.
/// Supports: !! (last command), !n (nth entry), !-n (nth from end), !prefix (last match).
/// Returns Ok(expanded) or Err(message) if the reference is invalid.
pub fn expand_history(input: &str, history: &[&str]) -> Result<String, String> {
    if !input.contains('!') {
        return Ok(input.to_owned());
    }

    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_single_quote = false;

    while let Some(ch) = chars.next() {
        if ch == '\'' {
            in_single_quote = !in_single_quote;
            result.push(ch);
            continue;
        }

        if ch != '!' || in_single_quote {
            result.push(ch);
            continue;
        }

        // We have '!' outside single quotes
        match chars.peek() {
            Some(&'!') => {
                // !! = last command
                chars.next();
                if history.is_empty() {
                    return Err("!!: event not found".to_owned());
                }
                result.push_str(history[history.len() - 1]);
            }
            Some(&c) if c.is_ascii_digit() => {
                // !n = nth history entry (1-based)
                let mut num_str = String::new();
                while let Some(&d) = chars.peek() {
                    if d.is_ascii_digit() {
                        num_str.push(d);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let n: usize = num_str.parse().unwrap();
                if n == 0 || n > history.len() {
                    return Err(format!("!{n}: event not found"));
                }
                result.push_str(history[n - 1]);
            }
            Some(&'-') => {
                // !-n = nth from end (1-based)
                chars.next();
                let mut num_str = String::new();
                while let Some(&d) = chars.peek() {
                    if d.is_ascii_digit() {
                        num_str.push(d);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if num_str.is_empty() {
                    return Err("!-: event not found".to_owned());
                }
                let n: usize = num_str.parse().unwrap();
                if n == 0 || n > history.len() {
                    return Err(format!("!-{n}: event not found"));
                }
                result.push_str(history[history.len() - n]);
            }
            Some(&c) if c.is_alphanumeric() || c == '_' || c == '/' || c == '.' => {
                // !prefix = most recent command starting with prefix
                let mut prefix = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' || c == '/' || c == '.' || c == '-' {
                        prefix.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if let Some(entry) = history.iter().rev().find(|e| e.starts_with(&prefix)) {
                    result.push_str(entry);
                } else {
                    return Err(format!("!{prefix}: event not found"));
                }
            }
            _ => {
                // Lone '!' at end or before space/special char — keep literal
                result.push('!');
            }
        }
    }

    Ok(result)
}

/// Parsed I/O redirections for a command.
#[derive(Debug, Default)]
pub struct Redirections {
    pub stdin_file: Option<String>,
    pub stdout_file: Option<String>,
    pub stdout_append: bool,
    pub stderr_file: Option<String>,
    pub stderr_append: bool,
    pub stderr_to_stdout: bool,
}

impl Redirections {
    /// Open the configured stdin as a Stdio, or return the given default.
    pub fn stdin_stdio(&self, default: Stdio) -> Result<Stdio, String> {
        match &self.stdin_file {
            Some(path) => {
                let path = expand_tilde(path);
                File::open(&path)
                    .map(Stdio::from)
                    .map_err(|e| format!("rsshell: {path}: {e}"))
            }
            None => Ok(default),
        }
    }

    /// Open the configured stdout as a Stdio, or return the given default.
    pub fn stdout_stdio(&self, default: Stdio) -> Result<Stdio, String> {
        match &self.stdout_file {
            Some(path) => {
                let path = expand_tilde(path);
                let file = if self.stdout_append {
                    OpenOptions::new().create(true).append(true).open(&path)
                } else {
                    File::create(&path)
                };
                file.map(Stdio::from)
                    .map_err(|e| format!("rsshell: {path}: {e}"))
            }
            None => Ok(default),
        }
    }

    /// Open the configured stderr as a Stdio, or return the given default.
    pub fn stderr_stdio(&self, default: Stdio) -> Result<Stdio, String> {
        if self.stderr_to_stdout {
            // 2>&1 is handled at the Command level by the caller
            return Ok(default);
        }
        match &self.stderr_file {
            Some(path) => {
                let path = expand_tilde(path);
                let file = if self.stderr_append {
                    OpenOptions::new().create(true).append(true).open(&path)
                } else {
                    File::create(&path)
                };
                file.map(Stdio::from)
                    .map_err(|e| format!("rsshell: {path}: {e}"))
            }
            None => Ok(default),
        }
    }
}

/// Extract redirections from a list of args, returning the remaining args and the redirections.
/// Supports: < file, > file, >> file, 2> file, 2>> file, 2>&1
pub fn parse_redirections(args: &[String]) -> Result<(Vec<String>, Redirections), String> {
    let mut remaining = Vec::new();
    let mut redir = Redirections::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "<" => {
                i += 1;
                if i >= args.len() {
                    return Err("syntax error near unexpected token `newline'".to_owned());
                }
                redir.stdin_file = Some(args[i].clone());
            }
            ">" => {
                i += 1;
                if i >= args.len() {
                    return Err("syntax error near unexpected token `newline'".to_owned());
                }
                redir.stdout_file = Some(args[i].clone());
                redir.stdout_append = false;
            }
            ">>" => {
                i += 1;
                if i >= args.len() {
                    return Err("syntax error near unexpected token `newline'".to_owned());
                }
                redir.stdout_file = Some(args[i].clone());
                redir.stdout_append = true;
            }
            "2>" => {
                i += 1;
                if i >= args.len() {
                    return Err("syntax error near unexpected token `newline'".to_owned());
                }
                redir.stderr_file = Some(args[i].clone());
                redir.stderr_append = false;
            }
            "2>>" => {
                i += 1;
                if i >= args.len() {
                    return Err("syntax error near unexpected token `newline'".to_owned());
                }
                redir.stderr_file = Some(args[i].clone());
                redir.stderr_append = true;
            }
            "2>&1" => {
                redir.stderr_to_stdout = true;
            }
            _ => {
                // Handle >file, >>file, <file (no space between operator and filename)
                if let Some(path) = arg.strip_prefix(">>") {
                    if path.is_empty() {
                        // Already handled above as ">>" token
                        remaining.push(arg.clone());
                    } else {
                        redir.stdout_file = Some(path.to_owned());
                        redir.stdout_append = true;
                    }
                } else if let Some(path) = arg.strip_prefix('>') {
                    if path.is_empty() {
                        remaining.push(arg.clone());
                    } else {
                        redir.stdout_file = Some(path.to_owned());
                        redir.stdout_append = false;
                    }
                } else if let Some(path) = arg.strip_prefix('<') {
                    if path.is_empty() {
                        remaining.push(arg.clone());
                    } else {
                        redir.stdin_file = Some(path.to_owned());
                    }
                } else if let Some(path) = arg.strip_prefix("2>>") {
                    if !path.is_empty() {
                        redir.stderr_file = Some(path.to_owned());
                        redir.stderr_append = true;
                    } else {
                        remaining.push(arg.clone());
                    }
                } else if let Some(path) = arg.strip_prefix("2>") {
                    if path == "&1" {
                        redir.stderr_to_stdout = true;
                    } else if !path.is_empty() {
                        redir.stderr_file = Some(path.to_owned());
                        redir.stderr_append = false;
                    } else {
                        remaining.push(arg.clone());
                    }
                } else {
                    remaining.push(arg.clone());
                }
            }
        }
        i += 1;
    }

    Ok((remaining, redir))
}

/// Parse a command line into arguments, handling single and double quotes.
pub fn parse_command_line(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape_next = false;

    for ch in input.chars() {
        if escape_next {
            current.push(ch);
            escape_next = false;
            continue;
        }

        match ch {
            '\\' if !in_single_quote => {
                escape_next = true;
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            ' ' | '\t' if !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}

/// Expand tilde in a path string.
pub fn expand_tilde(path: &str) -> String {
    if path == "~" {
        dirs::home_dir()
            .map(|h| h.display().to_string())
            .unwrap_or_else(|| path.to_owned())
    } else if let Some(rest) = path.strip_prefix("~/") {
        dirs::home_dir()
            .map(|h| format!("{}/{rest}", h.display()))
            .unwrap_or_else(|| path.to_owned())
    } else {
        path.to_owned()
    }
}

/// Expand environment variables in the form $VAR or ${VAR}.
pub fn expand_env_vars(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            let braced = chars.peek() == Some(&'{');
            if braced {
                chars.next(); // consume '{'
            }
            let mut var_name = String::new();
            while let Some(&c) = chars.peek() {
                if braced {
                    if c == '}' {
                        chars.next();
                        break;
                    }
                } else if !c.is_alphanumeric() && c != '_' {
                    break;
                }
                var_name.push(c);
                chars.next();
            }
            if let Ok(val) = env::var(&var_name) {
                result.push_str(&val);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Expand glob patterns in arguments.
pub fn expand_globs(args: &[String]) -> Vec<String> {
    let mut expanded = Vec::new();
    for arg in args {
        if arg.contains('*') || arg.contains('?') || arg.contains('[') {
            match glob::glob(arg) {
                Ok(paths) => {
                    let mut matched = false;
                    for entry in paths.flatten() {
                        expanded.push(entry.display().to_string());
                        matched = true;
                    }
                    if !matched {
                        expanded.push(arg.clone());
                    }
                }
                Err(_) => expanded.push(arg.clone()),
            }
        } else {
            expanded.push(arg.clone());
        }
    }
    expanded
}

/// Split a command line by pipes, returning each segment.
pub fn split_pipes(input: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape_next = false;

    for ch in input.chars() {
        if escape_next {
            current.push(ch);
            escape_next = false;
            continue;
        }
        match ch {
            '\\' if !in_single_quote => {
                escape_next = true;
                current.push(ch);
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                current.push(ch);
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                current.push(ch);
            }
            '|' if !in_single_quote && !in_double_quote => {
                segments.push(current.trim().to_owned());
                current.clear();
            }
            _ => {
                current.push(ch);
            }
        }
    }
    let trimmed = current.trim().to_owned();
    if !trimmed.is_empty() {
        segments.push(trimmed);
    }
    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        assert_eq!(parse_command_line("ls -la /tmp"), vec!["ls", "-la", "/tmp"]);
    }

    #[test]
    fn test_parse_quotes() {
        assert_eq!(
            parse_command_line(r#"echo "hello world""#),
            vec!["echo", "hello world"]
        );
    }

    #[test]
    fn test_parse_single_quotes() {
        assert_eq!(
            parse_command_line("echo 'hello world'"),
            vec!["echo", "hello world"]
        );
    }

    #[test]
    fn test_parse_escape() {
        assert_eq!(
            parse_command_line(r"echo hello\ world"),
            vec!["echo", "hello world"]
        );
    }

    #[test]
    fn test_expand_tilde() {
        let home = dirs::home_dir().unwrap().display().to_string();
        assert_eq!(expand_tilde("~"), home);
        assert_eq!(expand_tilde("~/foo"), format!("{home}/foo"));
        assert_eq!(expand_tilde("/tmp"), "/tmp");
    }

    #[test]
    fn test_split_pipes() {
        assert_eq!(split_pipes("ls | grep foo"), vec!["ls", "grep foo"]);
        assert_eq!(
            split_pipes("cat file | sort | uniq"),
            vec!["cat file", "sort", "uniq"]
        );
    }

    #[test]
    fn test_expand_env_vars() {
        // SAFETY: test runs are single-threaded via nextest
        unsafe { env::set_var("RSSHELL_TEST_VAR", "hello"); }
        assert_eq!(expand_env_vars("$RSSHELL_TEST_VAR"), "hello");
        assert_eq!(expand_env_vars("${RSSHELL_TEST_VAR}"), "hello");
        assert_eq!(expand_env_vars("say $RSSHELL_TEST_VAR!"), "say hello!");
        unsafe { env::remove_var("RSSHELL_TEST_VAR"); }
    }

    #[test]
    fn test_parse_empty() {
        assert!(parse_command_line("").is_empty());
        assert!(parse_command_line("   ").is_empty());
    }

    #[test]
    fn test_color_code() {
        assert_eq!(color_code("red"), Some("31"));
        assert_eq!(color_code("green"), Some("32"));
        assert_eq!(color_code("blue"), Some("34"));
        assert_eq!(color_code("none"), None);
        assert_eq!(color_code(""), None);
    }

    #[test]
    fn test_colorize_none() {
        assert_eq!(colorize("hello", "none", false), "hello");
    }

    #[test]
    fn test_colorize_with_color() {
        let result = colorize("hello", "red", false);
        assert!(result.contains("\x1b[31m"));
        assert!(result.contains("hello"));
        assert!(result.contains("\x1b[0m"));
    }

    #[test]
    fn test_colorize_bold() {
        let result = colorize("hello", "none", true);
        assert!(result.contains("\x1b[1m"));
        assert!(result.contains("hello"));
    }

    #[test]
    fn test_colorize_bold_and_color() {
        let result = colorize("hello", "green", true);
        assert!(result.contains("\x1b[1;32m"));
        assert!(result.contains("hello"));
    }

    #[test]
    fn test_expand_prompt_vars() {
        assert_eq!(
            expand_prompt_vars("{user}@{host}", "alice", "box", "/tmp", "main"),
            "alice@box"
        );
        assert_eq!(
            expand_prompt_vars("{cwd} ({git_branch})", "alice", "box", "~/src", "dev"),
            "~/src (dev)"
        );
    }

    #[test]
    fn test_expand_history_double_bang() {
        let history = vec!["ls -la", "echo hello"];
        assert_eq!(expand_history("!!", &history).unwrap(), "echo hello");
    }

    #[test]
    fn test_expand_history_number() {
        let history = vec!["ls -la", "echo hello", "pwd"];
        assert_eq!(expand_history("!1", &history).unwrap(), "ls -la");
        assert_eq!(expand_history("!2", &history).unwrap(), "echo hello");
        assert_eq!(expand_history("!3", &history).unwrap(), "pwd");
    }

    #[test]
    fn test_expand_history_negative() {
        let history = vec!["ls -la", "echo hello", "pwd"];
        assert_eq!(expand_history("!-1", &history).unwrap(), "pwd");
        assert_eq!(expand_history("!-2", &history).unwrap(), "echo hello");
        assert_eq!(expand_history("!-3", &history).unwrap(), "ls -la");
    }

    #[test]
    fn test_expand_history_prefix() {
        let history = vec!["ls -la", "echo hello", "ls /tmp"];
        assert_eq!(expand_history("!ls", &history).unwrap(), "ls /tmp");
        assert_eq!(expand_history("!echo", &history).unwrap(), "echo hello");
    }

    #[test]
    fn test_expand_history_not_found() {
        let history = vec!["ls -la"];
        assert!(expand_history("!5", &history).is_err());
        assert!(expand_history("!-5", &history).is_err());
        assert!(expand_history("!xyz", &history).is_err());
    }

    #[test]
    fn test_expand_history_empty() {
        let history: Vec<&str> = vec![];
        assert!(expand_history("!!", &history).is_err());
    }

    #[test]
    fn test_expand_history_no_expansion() {
        let history = vec!["ls -la"];
        assert_eq!(expand_history("echo hello", &history).unwrap(), "echo hello");
        assert_eq!(expand_history("echo !", &history).unwrap(), "echo !");
    }

    #[test]
    fn test_expand_history_in_single_quotes() {
        let history = vec!["ls -la"];
        assert_eq!(expand_history("echo '!!'", &history).unwrap(), "echo '!!'");
    }

    #[test]
    fn test_expand_history_mixed() {
        let history = vec!["ls -la", "echo hello"];
        assert_eq!(
            expand_history("echo ran: !!", &history).unwrap(),
            "echo ran: echo hello"
        );
    }

    #[test]
    fn test_default_prompt_config_parses() {
        let config: Config = toml::from_str(DEFAULT_CONFIG).unwrap();
        assert!(config.prompt.show_exit_code);
        assert!(!config.prompt.parts.is_empty());
        assert_eq!(config.prompt.exit_code_color, "red");
    }

    fn s(val: &str) -> String {
        val.to_owned()
    }

    #[test]
    fn test_parse_redirections_stdout() {
        let args = vec![s("echo"), s("hello"), s(">"), s("out.txt")];
        let (remaining, redir) = parse_redirections(&args).unwrap();
        assert_eq!(remaining, vec!["echo", "hello"]);
        assert_eq!(redir.stdout_file.as_deref(), Some("out.txt"));
        assert!(!redir.stdout_append);
    }

    #[test]
    fn test_parse_redirections_stdout_append() {
        let args = vec![s("echo"), s("hello"), s(">>"), s("out.txt")];
        let (remaining, redir) = parse_redirections(&args).unwrap();
        assert_eq!(remaining, vec!["echo", "hello"]);
        assert_eq!(redir.stdout_file.as_deref(), Some("out.txt"));
        assert!(redir.stdout_append);
    }

    #[test]
    fn test_parse_redirections_stdin() {
        let args = vec![s("sort"), s("<"), s("input.txt")];
        let (remaining, redir) = parse_redirections(&args).unwrap();
        assert_eq!(remaining, vec!["sort"]);
        assert_eq!(redir.stdin_file.as_deref(), Some("input.txt"));
    }

    #[test]
    fn test_parse_redirections_stderr() {
        let args = vec![s("cmd"), s("2>"), s("err.txt")];
        let (remaining, redir) = parse_redirections(&args).unwrap();
        assert_eq!(remaining, vec!["cmd"]);
        assert_eq!(redir.stderr_file.as_deref(), Some("err.txt"));
        assert!(!redir.stderr_append);
    }

    #[test]
    fn test_parse_redirections_stderr_append() {
        let args = vec![s("cmd"), s("2>>"), s("err.txt")];
        let (remaining, redir) = parse_redirections(&args).unwrap();
        assert_eq!(remaining, vec!["cmd"]);
        assert_eq!(redir.stderr_file.as_deref(), Some("err.txt"));
        assert!(redir.stderr_append);
    }

    #[test]
    fn test_parse_redirections_stderr_to_stdout() {
        let args = vec![s("cmd"), s("2>&1")];
        let (remaining, redir) = parse_redirections(&args).unwrap();
        assert_eq!(remaining, vec!["cmd"]);
        assert!(redir.stderr_to_stdout);
    }

    #[test]
    fn test_parse_redirections_no_space() {
        let args = vec![s("echo"), s("hello"), s(">out.txt")];
        let (remaining, redir) = parse_redirections(&args).unwrap();
        assert_eq!(remaining, vec!["echo", "hello"]);
        assert_eq!(redir.stdout_file.as_deref(), Some("out.txt"));
    }

    #[test]
    fn test_parse_redirections_combined() {
        let args = vec![s("cmd"), s("<"), s("in.txt"), s(">"), s("out.txt"), s("2>"), s("err.txt")];
        let (remaining, redir) = parse_redirections(&args).unwrap();
        assert_eq!(remaining, vec!["cmd"]);
        assert_eq!(redir.stdin_file.as_deref(), Some("in.txt"));
        assert_eq!(redir.stdout_file.as_deref(), Some("out.txt"));
        assert_eq!(redir.stderr_file.as_deref(), Some("err.txt"));
    }

    #[test]
    fn test_parse_redirections_missing_file() {
        let args = vec![s("echo"), s("hello"), s(">")];
        assert!(parse_redirections(&args).is_err());
    }

    #[test]
    fn test_parse_redirections_none() {
        let args = vec![s("ls"), s("-la")];
        let (remaining, redir) = parse_redirections(&args).unwrap();
        assert_eq!(remaining, vec!["ls", "-la"]);
        assert!(redir.stdin_file.is_none());
        assert!(redir.stdout_file.is_none());
        assert!(redir.stderr_file.is_none());
        assert!(!redir.stderr_to_stdout);
    }
}
