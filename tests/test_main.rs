use std::process::Command;

#[test]
fn test_cli_version_subcommand() {
    let output = Command::new(env!("CARGO_BIN_EXE_rsshell"))
        .arg("version")
        .output()
        .expect("failed to run rsshell");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("GIT_SHA:"));
}

#[test]
fn test_cli_command_string() {
    let output = Command::new(env!("CARGO_BIN_EXE_rsshell"))
        .args(["-c", "echo hello"])
        .output()
        .expect("failed to run rsshell");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "hello");
}

#[test]
fn test_cli_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_rsshell"))
        .arg("--help")
        .output()
        .expect("failed to run rsshell");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rsshell"));
}
