use std::env;

use rsshell::helpers::{
    Config, DEFAULT_CONFIG, PromptPart, PromptVars, bg_code, colorize, colorize_simple,
    expand_env_vars, expand_globs, expand_history, expand_tilde, fg_code, parse_command_line,
    parse_redirections, split_pipes,
};

fn s(val: &str) -> String {
    val.to_owned()
}

fn make_part(
    color: &str,
    bg: &str,
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
) -> PromptPart {
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

// ── parse_command_line ──

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
fn test_parse_empty() {
    assert!(parse_command_line("").is_empty());
    assert!(parse_command_line("   ").is_empty());
}

#[test]
fn test_parse_mixed_quotes() {
    assert_eq!(
        parse_command_line(r#"echo "it's" 'a "test"'"#),
        vec!["echo", "it's", r#"a "test""#]
    );
}

#[test]
fn test_parse_adjacent_quotes() {
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

// ── expand_tilde ──

#[test]
fn test_expand_tilde() {
    let home = dirs::home_dir().unwrap().display().to_string();
    assert_eq!(expand_tilde("~"), home);
    assert_eq!(expand_tilde("~/foo"), format!("{home}/foo"));
    assert_eq!(expand_tilde("/tmp"), "/tmp");
}

#[test]
fn test_expand_tilde_no_expansion() {
    assert_eq!(expand_tilde("/usr/bin"), "/usr/bin");
    assert_eq!(expand_tilde("relative/path"), "relative/path");
    assert_eq!(expand_tilde("~other/foo"), "~other/foo");
}

// ── split_pipes ──

#[test]
fn test_split_pipes() {
    assert_eq!(split_pipes("ls | grep foo"), vec!["ls", "grep foo"]);
    assert_eq!(
        split_pipes("cat file | sort | uniq"),
        vec!["cat file", "sort", "uniq"]
    );
}

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

// ── expand_env_vars ──

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
fn test_expand_env_vars_no_vars() {
    assert_eq!(expand_env_vars("hello world"), "hello world");
}

#[test]
fn test_expand_env_vars_undefined() {
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

// ── fg_code / bg_code ──

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

// ── colorize ──

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

// ── PromptVars ──

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

// ── expand_history ──

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
fn test_expand_history_bang_before_space() {
    let history = vec!["ls -la"];
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

// ── Config ──

#[test]
fn test_default_prompt_config_parses() {
    let config: Config = toml::from_str(DEFAULT_CONFIG).unwrap();
    assert!(config.prompt.show_exit_code);
    assert!(!config.prompt.parts.is_empty());
    assert_eq!(config.prompt.exit_code_color, "red");
}

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

// ── parse_redirections ──

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
    assert!(parse_redirections(&args).is_err());
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
