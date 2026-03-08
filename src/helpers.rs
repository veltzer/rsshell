use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

use serde::Deserialize;

/// Shell configuration loaded from config file.
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub prompt: PromptConfig,
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
/// Uses \x01 and \x02 markers so rustyline knows these are non-printable.
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
    format!("\x01\x1b[{seq}m\x02{text}\x01\x1b[0m\x02")
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
    fn test_default_prompt_config_parses() {
        let config: Config = toml::from_str(DEFAULT_CONFIG).unwrap();
        assert!(config.prompt.show_exit_code);
        assert!(!config.prompt.parts.is_empty());
        assert_eq!(config.prompt.exit_code_color, "red");
    }
}
