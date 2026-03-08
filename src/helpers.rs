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

#[derive(Debug, Deserialize, Default)]
pub struct PromptConfig {
    #[serde(default = "default_prompt_format")]
    pub format: String,
    #[serde(default)]
    pub show_exit_code: bool,
}

fn default_prompt_format() -> String {
    "{user}@{host}:{cwd}$ ".to_owned()
}

#[derive(Debug, Deserialize, Default)]
pub struct StartupConfig {
    #[serde(default)]
    pub commands: Vec<String>,
}

/// Default configuration file contents.
pub const DEFAULT_CONFIG: &str = r#"# rsshell configuration file

[prompt]
# Prompt format. Available variables: {user}, {host}, {cwd}, {home}
format = "{user}@{host}:{cwd}$ "
# Show the exit code of the last command when non-zero
show_exit_code = true

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

/// Build the prompt string from the config format.
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
    // Replace ~ alone (when cwd is exactly home)
    let cwd = if cwd == "~/" { "~".to_owned() } else { cwd };

    let mut prompt = config
        .prompt
        .format
        .replace("{user}", &user)
        .replace("{host}", &host)
        .replace("{cwd}", &cwd);

    if config.prompt.show_exit_code && last_exit_code != 0 {
        prompt = format!("[{last_exit_code}] {prompt}");
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
}
