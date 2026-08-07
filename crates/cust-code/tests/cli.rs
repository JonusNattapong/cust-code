use std::process::Command;

#[test]
fn test_version_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_cust"))
        .arg("version")
        .output()
        .expect("Failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0.0.0"));
}

#[test]
fn test_banner_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_cust"))
        .arg("banner")
        .env("CUST_SANDBOX", "read-only")
        .output()
        .expect("Failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("v0.0.0"));
    assert!(stdout.contains("sandbox: read-only"));
    assert!(stdout.contains("Ctrl+X terminal shell"));
    assert!(stdout.contains("/ slash commands"));
}

#[test]
fn test_help_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_cust"))
        .arg("help")
        .output()
        .expect("Failed to execute process");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("cust — a coding agent CLI"));
}
