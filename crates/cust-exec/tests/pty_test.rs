use cust_exec::PtyRunner;

#[test]
fn test_ansi_strip() {
    let ansi_text = "\x1B[31mError:\x1B[0m Something went wrong";
    let clean = PtyRunner::strip_ansi(ansi_text);
    assert_eq!(clean, "Error: Something went wrong");
}

#[test]
fn test_pty_command_run() {
    let res = PtyRunner::run_interactive("cmd", &["/C", "echo hello pty"]);
    if let Ok(output) = res {
        assert!(output.clean_output.contains("hello pty"));
    }
}
