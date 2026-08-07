use cust_exec::{CommandRunner, SandboxProfile, ShellPlan};
use std::env;

#[test]
fn test_cmd_parser() {
    let plan = ShellPlan::parse("sh -c \"echo hello > output.txt\"");
    assert_eq!(plan.program, "sh");
    assert!(!plan.nested_commands.is_empty());
}

#[tokio::test]
async fn test_runner_echo() {
    let cwd = env::current_dir().unwrap();
    let res = CommandRunner::run_cmd("echo hello_world", &cwd, SandboxProfile::Off)
        .await
        .unwrap();

    assert_eq!(res.exit_code, 0);
    assert!(res.stdout.contains("hello_world"));
}

#[tokio::test]
async fn test_sandbox_readonly() {
    let cwd = env::current_dir().unwrap();
    let res = SandboxProfile::ReadOnly.check_permission(&cwd, true, None);
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("read-only"));
}
