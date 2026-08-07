//! Integration tests that run the real binary, so the smoke checks in PLAN.md
//! are enforced by CI rather than by remembering to type them.

use std::path::PathBuf;
use std::process::Command;
use std::process::Output;

fn binary() -> PathBuf {
    // target/debug/deps/cli-<hash> -> target/debug/cust
    let mut path = std::env::current_exe().expect("test binary path");
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join(format!("cust{}", std::env::consts::EXE_SUFFIX))
}

fn run(args: &[&str]) -> Output {
    Command::new(binary())
        .args(args)
        .output()
        .expect("failed to run cust")
}

#[test]
fn no_args_prints_help_and_succeeds() {
    let output = run(&[]);
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");

    assert!(output.status.success());
    assert!(stdout.contains("a coding agent CLI"), "got: {stdout}");
    assert!(stdout.contains("usage: cust <command>"), "got: {stdout}");
}

#[test]
fn help_flags_match_the_help_command() {
    let expected = String::from_utf8(run(&["help"]).stdout).expect("utf-8 stdout");

    for flag in ["-h", "--help"] {
        let stdout = String::from_utf8(run(&[flag]).stdout).expect("utf-8 stdout");
        assert_eq!(stdout, expected, "`{flag}` diverged from `help`");
    }
}

#[test]
fn version_prints_the_crate_version() {
    for arg in ["version", "-V", "--version"] {
        let output = run(&[arg]);
        let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");

        assert!(output.status.success(), "`{arg}` did not exit 0");
        assert_eq!(stdout.trim(), env!("CARGO_PKG_VERSION"), "for `{arg}`");
    }
}

#[test]
fn unknown_command_fails_and_names_the_command() {
    let output = run(&["bogus"]);
    let stderr = String::from_utf8(output.stderr).expect("utf-8 stderr");

    assert!(!output.status.success());
    assert!(stderr.contains("bogus"), "got: {stderr}");
}
