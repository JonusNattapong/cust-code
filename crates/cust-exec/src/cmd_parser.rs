use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellPlan {
    pub raw_command: String,
    pub program: String,
    pub args: Vec<String>,
    pub nested_commands: Vec<ShellPlan>,
    pub target_paths: Vec<PathBuf>,
}

impl ShellPlan {
    pub fn parse(cmd: &str) -> Self {
        let trimmed = cmd.trim();
        let parts = split_args(trimmed);

        if parts.is_empty() {
            return Self {
                raw_command: cmd.to_string(),
                program: String::new(),
                args: Vec::new(),
                nested_commands: Vec::new(),
                target_paths: Vec::new(),
            };
        }

        let program = parts[0].clone();
        let args = parts[1..].to_vec();

        let mut nested_commands = Vec::new();
        let mut target_paths = Vec::new();

        // Detect shell subcommands like sh -c "subcmd" or bash -c "subcmd"
        if (program == "sh"
            || program == "bash"
            || program == "zsh"
            || program == "powershell"
            || program == "cmd")
            && args.len() >= 2
        {
            for i in 0..args.len() - 1 {
                if (args[i] == "-c" || args[i] == "/C" || args[i] == "-Command")
                    && i + 1 < args.len()
                {
                    let sub_cmd = &args[i + 1];
                    nested_commands.push(ShellPlan::parse(sub_cmd));
                }
            }
        }

        // Detect path arguments
        for arg in &args {
            if arg.contains('/')
                || arg.contains('\\')
                || arg.ends_with(".txt")
                || arg.ends_with(".rs")
                || arg.ends_with(".toml")
            {
                target_paths.push(PathBuf::from(arg));
            }
        }

        Self {
            raw_command: cmd.to_string(),
            program,
            args,
            nested_commands,
            target_paths,
        }
    }
}

fn split_args(cmd: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = ' ';

    for ch in cmd.chars() {
        match ch {
            '"' | '\'' if !in_quotes => {
                in_quotes = true;
                quote_char = ch;
            }
            c if in_quotes && c == quote_char => {
                in_quotes = false;
            }
            ' ' | '\t' if !in_quotes => {
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
