use std::collections::HashMap;

use rsshell::commands::{
    execute_line, expand_aliases, expand_local_vars, parse_variable_assignment,
};
use rsshell::helpers::Config;

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
fn test_parse_variable_assignment_empty_value() {
    assert_eq!(
        parse_variable_assignment("FOO="),
        Some(("FOO".to_owned(), String::new()))
    );
}

#[test]
fn test_parse_variable_assignment_with_equals_in_value() {
    assert_eq!(
        parse_variable_assignment("FOO=a=b"),
        Some(("FOO".to_owned(), "a=b".to_owned()))
    );
}

#[test]
fn test_parse_variable_assignment_invalid_name() {
    assert_eq!(parse_variable_assignment("foo-bar=baz"), None);
}

#[test]
fn test_parse_variable_assignment_numeric_name() {
    assert_eq!(
        parse_variable_assignment("VAR123=val"),
        Some(("VAR123".to_owned(), "val".to_owned()))
    );
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
fn test_expand_aliases_no_match() {
    let config = Config::default();
    assert_eq!(expand_aliases("ls -la", &config), "ls -la");
}

#[test]
fn test_expand_aliases_exact_match_no_args() {
    let mut config = Config::default();
    config.aliases.insert("ll".to_owned(), "ls -la".to_owned());
    assert_eq!(expand_aliases("ll", &config), "ls -la");
}

#[test]
fn test_expand_aliases_only_first_word() {
    let mut config = Config::default();
    config.aliases.insert("ll".to_owned(), "ls -la".to_owned());
    assert_eq!(expand_aliases("echo ll", &config), "echo ll");
}

#[test]
fn test_expand_local_vars() {
    let mut vars = HashMap::new();
    vars.insert("name".to_owned(), "world".to_owned());
    assert_eq!(expand_local_vars("hello $name", &vars), "hello world");
    assert_eq!(expand_local_vars("hello ${name}", &vars), "hello world");
}

#[test]
fn test_expand_local_vars_multiple() {
    let mut vars = HashMap::new();
    vars.insert("first".to_owned(), "hello".to_owned());
    vars.insert("second".to_owned(), "world".to_owned());
    assert_eq!(expand_local_vars("$first $second", &vars), "hello world");
}

#[test]
fn test_expand_local_vars_no_match() {
    let vars = HashMap::new();
    assert_eq!(expand_local_vars("hello $name", &vars), "hello $name");
}

#[test]
fn test_execute_line_empty_and_comment() {
    let config = Config::default();
    let mut vars = HashMap::new();
    assert_eq!(execute_line("", &config, &mut vars), 0);
    assert_eq!(execute_line("   ", &config, &mut vars), 0);
    assert_eq!(execute_line("# this is a comment", &config, &mut vars), 0);
}

#[test]
fn test_execute_line_variable_assignment() {
    let config = Config::default();
    let mut vars = HashMap::new();
    assert_eq!(execute_line("FOO=bar", &config, &mut vars), 0);
    assert_eq!(vars.get("FOO").unwrap(), "bar");
}

#[test]
fn test_execute_line_echo() {
    let config = Config::default();
    let mut vars = HashMap::new();
    assert_eq!(execute_line("echo hello", &config, &mut vars), 0);
}

#[test]
fn test_execute_line_nonexistent_command() {
    let config = Config::default();
    let mut vars = HashMap::new();
    assert_eq!(execute_line("rsshell_nonexistent_cmd_xyz", &config, &mut vars), 127);
}

#[test]
fn test_execute_line_true_false() {
    let config = Config::default();
    let mut vars = HashMap::new();
    assert_eq!(execute_line("true", &config, &mut vars), 0);
    assert_ne!(execute_line("false", &config, &mut vars), 0);
}

#[test]
fn test_execute_line_pipeline() {
    let config = Config::default();
    let mut vars = HashMap::new();
    assert_eq!(execute_line("echo hello | cat", &config, &mut vars), 0);
}
