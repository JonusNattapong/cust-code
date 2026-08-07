use std::process::ExitCode;

const HELP: &str = "\
cust — a coding agent CLI

usage: cust <command>

  help      show this message
  version   print the version
";

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        None | Some("help") | Some("-h") | Some("--help") => {
            print!("{HELP}");
            ExitCode::SUCCESS
        }
        Some("version") | Some("-V") | Some("--version") => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("unknown command: {other}");
            ExitCode::FAILURE
        }
    }
}
