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
    let d = || "none".to_owned();
    vec![
        PromptPart { text: "{user}".to_owned(), color: "green".to_owned(), bg: d(), bold: false, dim: false, italic: false, underline: false, strikethrough: false },
        PromptPart { text: "@".to_owned(), color: d(), bg: d(), bold: false, dim: false, italic: false, underline: false, strikethrough: false },
        PromptPart { text: "{host}".to_owned(), color: "green".to_owned(), bg: d(), bold: false, dim: false, italic: false, underline: false, strikethrough: false },
        PromptPart { text: ":".to_owned(), color: d(), bg: d(), bold: false, dim: false, italic: false, underline: false, strikethrough: false },
        PromptPart { text: "{cwd}".to_owned(), color: "blue".to_owned(), bg: d(), bold: true, dim: false, italic: false, underline: false, strikethrough: false },
        PromptPart { text: "$ ".to_owned(), color: d(), bg: d(), bold: false, dim: false, italic: false, underline: false, strikethrough: false },
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
    #[serde(default = "default_color")]
    pub bg: String,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub dim: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub underline: bool,
    #[serde(default)]
    pub strikethrough: bool,
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
#   text  - literal text or a variable (see below)
#   color - foreground color (see below)
#   bg    - background color (see below)
#   bold, dim, italic, underline, strikethrough - style flags (default: false)
#
# Variables:
#   {user}          - current username
#   {host}          - hostname
#   {cwd}           - current working directory (~ for home)
#   {cwd_basename}  - basename of current directory
#   {git_branch}    - current git branch (empty if not in a repo)
#   {git_dirty}     - "dirty" or "clean" (empty if not in a repo)
#   {git_status}    - "*" if dirty, empty if clean or not in a repo
#   {git_sha}       - full git commit SHA
#   {git_sha_short} - short git commit SHA
#   {git_repo}      - git repository name (top-level directory name)
#   {date}          - current date (YYYY-MM-DD)
#   {time}          - current time (HH:MM:SS)
#   {shell}         - shell name (rsshell)
#   {newline}       - a newline character
#   {$}             - a literal "$"
#
# Colors: none, black, red, green, yellow, blue, magenta, cyan, white,
#   bright_black (gray/grey), bright_red, bright_green, bright_yellow,
#   bright_blue, bright_magenta, bright_cyan, bright_white

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

/// Map a foreground color name to its ANSI code.
fn fg_code(name: &str) -> Option<&'static str> {
    match name {
        "black" => Some("30"),
        "red" => Some("31"),
        "green" => Some("32"),
        "yellow" => Some("33"),
        "blue" => Some("34"),
        "magenta" => Some("35"),
        "cyan" => Some("36"),
        "white" => Some("37"),
        "bright_black" | "gray" | "grey" => Some("90"),
        "bright_red" => Some("91"),
        "bright_green" => Some("92"),
        "bright_yellow" => Some("93"),
        "bright_blue" => Some("94"),
        "bright_magenta" => Some("95"),
        "bright_cyan" => Some("96"),
        "bright_white" => Some("97"),
        _ => None,
    }
}

/// Map a background color name to its ANSI code.
fn bg_code(name: &str) -> Option<&'static str> {
    match name {
        "black" => Some("40"),
        "red" => Some("41"),
        "green" => Some("42"),
        "yellow" => Some("43"),
        "blue" => Some("44"),
        "magenta" => Some("45"),
        "cyan" => Some("46"),
        "white" => Some("47"),
        "bright_black" | "gray" | "grey" => Some("100"),
        "bright_red" => Some("101"),
        "bright_green" => Some("102"),
        "bright_yellow" => Some("103"),
        "bright_blue" => Some("104"),
        "bright_magenta" => Some("105"),
        "bright_cyan" => Some("106"),
        "bright_white" => Some("107"),
        _ => None,
    }
}

/// Wrap text in ANSI escape sequences for color and style.
fn colorize(text: &str, part: &PromptPart) -> String {
    let fg = fg_code(&part.color);
    let bg = bg_code(&part.bg);
    let has_style = part.bold || part.dim || part.italic || part.underline || part.strikethrough;

    if fg.is_none() && bg.is_none() && !has_style {
        return text.to_owned();
    }

    let mut params = Vec::new();
    if part.bold {
        params.push("1");
    }
    if part.dim {
        params.push("2");
    }
    if part.italic {
        params.push("3");
    }
    if part.underline {
        params.push("4");
    }
    if part.strikethrough {
        params.push("9");
    }
    if let Some(c) = fg {
        params.push(c);
    }
    if let Some(c) = bg {
        params.push(c);
    }
    let seq = params.join(";");
    format!("\x1b[{seq}m{text}\x1b[0m")
}

/// Wrap text with a simple color + bold (used for exit code display).
fn colorize_simple(text: &str, color: &str, bold: bool) -> String {
    let code = fg_code(color);
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

/// Collected prompt variables, computed once per prompt render.
struct PromptVars {
    user: String,
    host: String,
    cwd: String,
    cwd_basename: String,
    git_branch: String,
    git_dirty: String,
    git_status: String,
    git_sha: String,
    git_sha_short: String,
    git_repo: String,
    date: String,
    time: String,
    shell: String,
    newline: String,
    dollar: String,
}

impl PromptVars {
    fn collect() -> Self {
        let user = env::var("USER").unwrap_or_else(|_| "user".to_owned());
        let host = hostname();
        let cwd = env::current_dir()
            .map(|p| {
                let home = dirs::home_dir().unwrap_or_default();
                if let Ok(rest) = p.strip_prefix(&home) {
                    let s = format!("~/{}", rest.display());
                    s.trim_end_matches('/').to_owned()
                } else {
                    p.display().to_string()
                }
            })
            .unwrap_or_else(|_| "?".to_owned());
        let cwd = if cwd == "~/" { "~".to_owned() } else { cwd };
        let cwd_basename = cwd.rsplit('/').next().unwrap_or(&cwd).to_owned();

        let git_branch = git_cmd(&["rev-parse", "--abbrev-ref", "HEAD"]);
        let in_repo = !git_branch.is_empty();

        let git_dirty = if in_repo {
            let status = git_cmd(&["status", "--porcelain"]);
            if status.is_empty() { "clean".to_owned() } else { "dirty".to_owned() }
        } else {
            String::new()
        };

        let git_status = if in_repo {
            if git_dirty == "dirty" { "*".to_owned() } else { String::new() }
        } else {
            String::new()
        };

        let git_sha = if in_repo {
            git_cmd(&["rev-parse", "HEAD"])
        } else {
            String::new()
        };

        let git_sha_short = if in_repo {
            git_cmd(&["rev-parse", "--short", "HEAD"])
        } else {
            String::new()
        };

        let git_repo = if in_repo {
            git_cmd(&["rev-parse", "--show-toplevel"])
                .rsplit('/')
                .next()
                .unwrap_or("")
                .to_owned()
        } else {
            String::new()
        };

        let now = chrono_now();
        let date = now.0;
        let time = now.1;

        Self {
            user,
            host,
            cwd,
            cwd_basename,
            git_branch,
            git_dirty,
            git_status,
            git_sha,
            git_sha_short,
            git_repo,
            date,
            time,
            shell: "rsshell".to_owned(),
            newline: "\n".to_owned(),
            dollar: "$".to_owned(),
        }
    }

    fn expand(&self, text: &str) -> String {
        text.replace("{user}", &self.user)
            .replace("{host}", &self.host)
            .replace("{cwd}", &self.cwd)
            .replace("{cwd_basename}", &self.cwd_basename)
            .replace("{git_branch}", &self.git_branch)
            .replace("{git_dirty}", &self.git_dirty)
            .replace("{git_status}", &self.git_status)
            .replace("{git_sha}", &self.git_sha)
            .replace("{git_sha_short}", &self.git_sha_short)
            .replace("{git_repo}", &self.git_repo)
            .replace("{date}", &self.date)
            .replace("{time}", &self.time)
            .replace("{shell}", &self.shell)
            .replace("{newline}", &self.newline)
            .replace("{$}", &self.dollar)
    }
}

/// Run a git command and return trimmed stdout, or empty string on failure.
fn git_cmd(args: &[&str]) -> String {
    std::process::Command::new("git")
        .args(args)
        .stderr(std::process::Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_default()
}

/// Get current date and time as (YYYY-MM-DD, HH:MM:SS).
fn chrono_now() -> (String, String) {
    std::process::Command::new("date")
        .arg("+%Y-%m-%d %H:%M:%S")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_owned();
            let mut parts = s.splitn(2, ' ');
            let date = parts.next().unwrap_or("").to_owned();
            let time = parts.next().unwrap_or("").to_owned();
            (date, time)
        })
        .unwrap_or_else(|| (String::new(), String::new()))
}

/// Build the prompt string from the configured parts.
pub fn build_prompt(config: &Config, last_exit_code: i32) -> String {
    let vars = PromptVars::collect();

    let mut prompt = String::new();

    if config.prompt.show_exit_code && last_exit_code != 0 {
        let code_text = format!("[{last_exit_code}] ");
        prompt.push_str(&colorize_simple(&code_text, &config.prompt.exit_code_color, true));
    }

    for part in &config.prompt.parts {
        let expanded = vars.expand(&part.text);
        prompt.push_str(&colorize(&expanded, part));
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
    fn test_fg_code() {
        assert_eq!(fg_code("red"), Some("31"));
        assert_eq!(fg_code("green"), Some("32"));
        assert_eq!(fg_code("blue"), Some("34"));
        assert_eq!(fg_code("bright_red"), Some("91"));
        assert_eq!(fg_code("gray"), Some("90"));
        assert_eq!(fg_code("grey"), Some("90"));
        assert_eq!(fg_code("none"), None);
        assert_eq!(fg_code(""), None);
    }

    #[test]
    fn test_bg_code() {
        assert_eq!(bg_code("red"), Some("41"));
        assert_eq!(bg_code("bright_blue"), Some("104"));
        assert_eq!(bg_code("none"), None);
    }

    fn make_part(color: &str, bg: &str, bold: bool, dim: bool, italic: bool, underline: bool, strikethrough: bool) -> PromptPart {
        PromptPart {
            text: String::new(),
            color: color.to_owned(),
            bg: bg.to_owned(),
            bold,
            dim,
            italic,
            underline,
            strikethrough,
        }
    }

    #[test]
    fn test_colorize_none() {
        let part = make_part("none", "none", false, false, false, false, false);
        assert_eq!(colorize("hello", &part), "hello");
    }

    #[test]
    fn test_colorize_with_color() {
        let part = make_part("red", "none", false, false, false, false, false);
        let result = colorize("hello", &part);
        assert!(result.contains("\x1b[31m"));
        assert!(result.contains("hello"));
        assert!(result.contains("\x1b[0m"));
    }

    #[test]
    fn test_colorize_bold() {
        let part = make_part("none", "none", true, false, false, false, false);
        let result = colorize("hello", &part);
        assert!(result.contains("\x1b[1m"));
        assert!(result.contains("hello"));
    }

    #[test]
    fn test_colorize_bold_and_color() {
        let part = make_part("green", "none", true, false, false, false, false);
        let result = colorize("hello", &part);
        assert!(result.contains("\x1b[1;32m"));
        assert!(result.contains("hello"));
    }

    #[test]
    fn test_colorize_italic_underline() {
        let part = make_part("none", "none", false, false, true, true, false);
        let result = colorize("hello", &part);
        assert!(result.contains("\x1b[3;4m"));
    }

    #[test]
    fn test_colorize_bg() {
        let part = make_part("white", "blue", false, false, false, false, false);
        let result = colorize("hello", &part);
        assert!(result.contains("37"));
        assert!(result.contains("44"));
    }

    #[test]
    fn test_colorize_bright() {
        let part = make_part("bright_cyan", "none", false, false, false, false, false);
        let result = colorize("hello", &part);
        assert!(result.contains("\x1b[96m"));
    }

    #[test]
    fn test_prompt_vars_expand() {
        let vars = PromptVars {
            user: "alice".to_owned(),
            host: "box".to_owned(),
            cwd: "~/src".to_owned(),
            cwd_basename: "src".to_owned(),
            git_branch: "main".to_owned(),
            git_dirty: "clean".to_owned(),
            git_status: String::new(),
            git_sha: "abc123".to_owned(),
            git_sha_short: "abc".to_owned(),
            git_repo: "myrepo".to_owned(),
            date: "2026-01-01".to_owned(),
            time: "12:00:00".to_owned(),
            shell: "rsshell".to_owned(),
            newline: "\n".to_owned(),
            dollar: "$".to_owned(),
        };
        assert_eq!(vars.expand("{user}@{host}"), "alice@box");
        assert_eq!(vars.expand("{cwd} ({git_branch})"), "~/src (main)");
        assert_eq!(vars.expand("{cwd_basename}"), "src");
        assert_eq!(vars.expand("{git_dirty}"), "clean");
        assert_eq!(vars.expand("{git_sha_short}"), "abc");
        assert_eq!(vars.expand("{git_repo}"), "myrepo");
        assert_eq!(vars.expand("{date} {time}"), "2026-01-01 12:00:00");
        assert_eq!(vars.expand("{shell}"), "rsshell");
        assert_eq!(vars.expand("{$}"), "$");
        assert_eq!(vars.expand("{newline}"), "\n");
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

    // ── parse_command_line edge cases ──

    #[test]
    fn test_parse_mixed_quotes() {
        assert_eq!(
            parse_command_line(r#"echo "it's" 'a "test"'"#),
            vec!["echo", "it's", r#"a "test""#]
        );
    }

    #[test]
    fn test_parse_adjacent_quotes() {
        // Adjacent quoted segments merge into one arg
        assert_eq!(
            parse_command_line(r#""hello""world""#),
            vec!["helloworld"]
        );
    }

    #[test]
    fn test_parse_escaped_quote() {
        assert_eq!(
            parse_command_line(r#"echo \"hello\""#),
            vec!["echo", r#""hello""#]
        );
    }

    #[test]
    fn test_parse_escaped_space_in_double_quotes() {
        // Backslash inside double quotes is consumed as escape
        assert_eq!(
            parse_command_line(r#"echo "hello\ world""#),
            vec!["echo", "hello world"]
        );
    }

    #[test]
    fn test_parse_tabs_as_separators() {
        assert_eq!(
            parse_command_line("ls\t-la\t/tmp"),
            vec!["ls", "-la", "/tmp"]
        );
    }

    #[test]
    fn test_parse_multiple_spaces() {
        assert_eq!(
            parse_command_line("ls   -la    /tmp"),
            vec!["ls", "-la", "/tmp"]
        );
    }

    // ── split_pipes edge cases ──

    #[test]
    fn test_split_pipes_single_command() {
        assert_eq!(split_pipes("ls -la"), vec!["ls -la"]);
    }

    #[test]
    fn test_split_pipes_pipe_in_single_quotes() {
        assert_eq!(
            split_pipes("echo 'hello | world'"),
            vec!["echo 'hello | world'"]
        );
    }

    #[test]
    fn test_split_pipes_pipe_in_double_quotes() {
        assert_eq!(
            split_pipes(r#"echo "hello | world""#),
            vec![r#"echo "hello | world""#]
        );
    }

    #[test]
    fn test_split_pipes_escaped_pipe() {
        assert_eq!(
            split_pipes(r"echo hello \| world"),
            vec![r"echo hello \| world"]
        );
    }

    #[test]
    fn test_split_pipes_empty() {
        assert!(split_pipes("").is_empty());
        assert!(split_pipes("   ").is_empty());
    }

    // ── expand_env_vars edge cases ──

    #[test]
    fn test_expand_env_vars_no_vars() {
        assert_eq!(expand_env_vars("hello world"), "hello world");
    }

    #[test]
    fn test_expand_env_vars_undefined() {
        // Undefined vars expand to empty string
        assert_eq!(expand_env_vars("$RSSHELL_NONEXISTENT_VAR_XYZ"), "");
    }

    #[test]
    fn test_expand_env_vars_braced_undefined() {
        assert_eq!(expand_env_vars("${RSSHELL_NONEXISTENT_VAR_XYZ}"), "");
    }

    #[test]
    fn test_expand_env_vars_adjacent_text() {
        unsafe { env::set_var("RSSHELL_TEST_ADJ", "foo"); }
        assert_eq!(expand_env_vars("${RSSHELL_TEST_ADJ}bar"), "foobar");
        unsafe { env::remove_var("RSSHELL_TEST_ADJ"); }
    }

    #[test]
    fn test_expand_env_vars_multiple() {
        unsafe {
            env::set_var("RSSHELL_T1", "hello");
            env::set_var("RSSHELL_T2", "world");
        }
        assert_eq!(expand_env_vars("$RSSHELL_T1 $RSSHELL_T2"), "hello world");
        unsafe {
            env::remove_var("RSSHELL_T1");
            env::remove_var("RSSHELL_T2");
        }
    }

    // ── expand_tilde edge cases ──

    #[test]
    fn test_expand_tilde_no_expansion() {
        assert_eq!(expand_tilde("/usr/bin"), "/usr/bin");
        assert_eq!(expand_tilde("relative/path"), "relative/path");
        // ~user syntax is not expanded
        assert_eq!(expand_tilde("~other/foo"), "~other/foo");
    }

    // ── expand_globs ──

    #[test]
    fn test_expand_globs_no_pattern() {
        let args = vec![s("ls"), s("-la"), s("/tmp")];
        assert_eq!(expand_globs(&args), args);
    }

    #[test]
    fn test_expand_globs_no_match_returns_literal() {
        let args = vec![s("/nonexistent_rsshell_dir_xyz/*.txt")];
        assert_eq!(expand_globs(&args), args);
    }

    // ── colorize_simple ──

    #[test]
    fn test_colorize_simple_no_style() {
        assert_eq!(colorize_simple("hello", "none", false), "hello");
    }

    #[test]
    fn test_colorize_simple_bold_only() {
        let result = colorize_simple("hello", "none", true);
        assert!(result.contains("\x1b[1m"));
        assert!(result.contains("hello"));
        assert!(result.contains("\x1b[0m"));
    }

    #[test]
    fn test_colorize_simple_color_and_bold() {
        let result = colorize_simple("hello", "red", true);
        assert!(result.contains("1"));
        assert!(result.contains("31"));
        assert!(result.contains("hello"));
    }

    // ── colorize all styles ──

    #[test]
    fn test_colorize_dim() {
        let part = make_part("none", "none", false, true, false, false, false);
        let result = colorize("test", &part);
        assert!(result.contains("\x1b[2m"));
    }

    #[test]
    fn test_colorize_strikethrough() {
        let part = make_part("none", "none", false, false, false, false, true);
        let result = colorize("test", &part);
        assert!(result.contains("\x1b[9m"));
    }

    #[test]
    fn test_colorize_all_styles() {
        let part = make_part("red", "blue", true, true, true, true, true);
        let result = colorize("test", &part);
        assert!(result.contains("1"));  // bold
        assert!(result.contains("2"));  // dim
        assert!(result.contains("3"));  // italic
        assert!(result.contains("4"));  // underline
        assert!(result.contains("9"));  // strikethrough
        assert!(result.contains("31")); // red fg
        assert!(result.contains("44")); // blue bg
    }

    // ── Config loading ──

    #[test]
    fn test_default_config_values() {
        let config = Config::default();
        assert!(!config.prompt.show_exit_code);
        assert_eq!(config.prompt.exit_code_color, "red");
        assert_eq!(config.history.max_entries, 10000);
        assert!(config.history.ignore_duplicates);
        assert!(config.history.ignore_space);
        assert!(config.aliases.is_empty());
        assert!(config.env.is_empty());
        assert!(config.startup.commands.is_empty());
    }

    #[test]
    fn test_config_from_toml() {
        let toml_str = r#"
[prompt]
show_exit_code = true

[aliases]
ll = "ls -la"
gs = "git status"

[env]
EDITOR = "vim"

[startup]
commands = ["echo hello"]
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.prompt.show_exit_code);
        assert_eq!(config.aliases.get("ll").unwrap(), "ls -la");
        assert_eq!(config.aliases.get("gs").unwrap(), "git status");
        assert_eq!(config.env.get("EDITOR").unwrap(), "vim");
        assert_eq!(config.startup.commands, vec!["echo hello"]);
    }

    #[test]
    fn test_config_empty_toml() {
        let config: Config = toml::from_str("").unwrap();
        assert!(!config.prompt.show_exit_code);
        assert!(config.aliases.is_empty());
    }

    // ── expand_history additional edge cases ──

    #[test]
    fn test_expand_history_bang_before_space() {
        let history = vec!["ls -la"];
        // '!' followed by space is kept literal
        assert_eq!(expand_history("echo ! hello", &history).unwrap(), "echo ! hello");
    }

    #[test]
    fn test_expand_history_bang_zero() {
        let history = vec!["ls -la"];
        assert!(expand_history("!0", &history).is_err());
    }

    #[test]
    fn test_expand_history_negative_zero() {
        let history = vec!["ls -la"];
        assert!(expand_history("!-0", &history).is_err());
    }

    #[test]
    fn test_expand_history_prefix_with_dots() {
        let history = vec!["./configure --prefix=/usr", "make"];
        assert_eq!(expand_history("!./conf", &history).unwrap(), "./configure --prefix=/usr");
    }

    #[test]
    fn test_expand_history_prefix_with_slash() {
        let history = vec!["/usr/bin/foo --bar", "echo done"];
        assert_eq!(expand_history("!/usr", &history).unwrap(), "/usr/bin/foo --bar");
    }

    // ── parse_redirections additional ──

    #[test]
    fn test_parse_redirections_no_space_stdin() {
        let args = vec![s("sort"), s("<input.txt")];
        let (remaining, redir) = parse_redirections(&args).unwrap();
        assert_eq!(remaining, vec!["sort"]);
        assert_eq!(redir.stdin_file.as_deref(), Some("input.txt"));
    }

    #[test]
    fn test_parse_redirections_no_space_append() {
        let args = vec![s("echo"), s("hi"), s(">>out.txt")];
        let (remaining, redir) = parse_redirections(&args).unwrap();
        assert_eq!(remaining, vec!["echo", "hi"]);
        assert_eq!(redir.stdout_file.as_deref(), Some("out.txt"));
        assert!(redir.stdout_append);
    }

    #[test]
    fn test_parse_redirections_2_stderr_no_space() {
        let args = vec![s("cmd"), s("2>err.txt")];
        let (remaining, redir) = parse_redirections(&args).unwrap();
        assert_eq!(remaining, vec!["cmd"]);
        assert_eq!(redir.stderr_file.as_deref(), Some("err.txt"));
    }

    #[test]
    fn test_parse_redirections_2_stderr_to_stdout_no_space() {
        let args = vec![s("cmd"), s("2>&1")];
        let (remaining, redir) = parse_redirections(&args).unwrap();
        assert_eq!(remaining, vec!["cmd"]);
        assert!(redir.stderr_to_stdout);
    }

    #[test]
    fn test_parse_redirections_missing_file_stdin() {
        let args = vec![s("sort"), s("<")];
        assert!(parse_redirections(&args).is_err());
    }

    #[test]
    fn test_parse_redirections_missing_file_stderr() {
        let args = vec![s("cmd"), s("2>")];
        // "2>" as a token requires a following arg
        assert!(parse_redirections(&args).is_err());
    }
}
