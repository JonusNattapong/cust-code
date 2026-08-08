use cust_config::{ConfigLoader, PartialConfig};
use cust_core::{AgentLoop, ApprovalHandler, EventKind};
use cust_tools::ToolRegistry;
use cust_tools_api::PermissionRequest;
use futures_util::StreamExt;
use std::io::Write;
use std::process::ExitCode;

const HELP: &str = "\
cust — a coding agent CLI

usage:
  cust \"<prompt>\"                run turn loop with tools (default)
  cust run \"<prompt>\"            run turn loop with tools
  cust ask \"<prompt>\"            single-shot prompt without tools
  cust tui                        interactive terminal UI
  cust list                       list saved sessions
  cust resume <id>                print a saved session's transcript
  cust banner                     print the welcome banner
  cust help                       show this message
  cust version                    print the version

options:
  --provider <name>              provider name (e.g. openai, anthropic, xai, mistral)
  --model <name>                 model name (e.g. gpt-4o, claude-3-5-sonnet-20241022)
  --api-key <key>                API key
  --base-url <url>               custom base URL
  --sandbox <profile>            off | workspace | read-only | strict
  -y, --yes                      auto-approve all tool permission requests
";

struct CliApprovalHandler {
    auto_approve: bool,
}

impl ApprovalHandler for CliApprovalHandler {
    fn request_approval(&self, tool: &str, request: &PermissionRequest) -> bool {
        if self.auto_approve {
            println!("\n[Auto-approved permission for {tool}: {request}]");
            return true;
        }

        print!("\n[?] Permission requested for {tool}: {request}\n    Allow execution? [y/N]: ");
        let _ = std::io::stdout().flush();

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_ok() {
            let trimmed = input.trim().to_lowercase();
            trimmed == "y" || trimmed == "yes"
        } else {
            false
        }
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() <= 1 {
        print_banner();
        println!();
        print!("{HELP}");
        return ExitCode::SUCCESS;
    }

    match args[1].as_str() {
        "banner" => {
            print_banner();
            ExitCode::SUCCESS
        }
        "tui" => {
            if let Err(err) = handle_tui(&args[2..]).await {
                eprintln!("{err}");
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        "help" | "-h" | "--help" => {
            print!("{HELP}");
            ExitCode::SUCCESS
        }
        "version" | "-V" | "--version" => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        "ask" => {
            if let Err(err) = handle_ask(&args[2..]).await {
                eprintln!("{err}");
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        "run" => {
            if let Err(err) = handle_run(&args[2..]).await {
                eprintln!("{err}");
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        "list" => {
            let store = cust_session::SessionStore::new(cust_session::SessionStore::default_dir());
            match store.list_sessions() {
                Ok(sessions) => {
                    println!("Saved Sessions ({} total):", sessions.len());
                    for s in sessions {
                        println!("  - [{}] {}", s.id, s.title);
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("Failed to list sessions: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        "resume" => {
            let Some(id) = args.get(2) else {
                eprintln!("usage: cust resume <id>");
                return ExitCode::FAILURE;
            };
            let store = cust_session::SessionStore::new(cust_session::SessionStore::default_dir());
            match store.load_session(id) {
                Ok((meta, messages)) => {
                    println!("Session [{}] {}", meta.id, meta.title);
                    println!("({} messages)\n", messages.len());
                    for msg in messages {
                        println!("{}: {}", msg.role, msg.content);
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("Failed to resume session '{id}': {e}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            // Default command: treat all args as prompt for run loop
            if let Err(err) = handle_run(&args[1..]).await {
                eprintln!("{err}");
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
    }
}

/// Print the welcome banner, using the resolved config when it loads and
/// falling back to placeholders when it does not (e.g. no API key yet).
fn print_banner() {
    let mut info = cust_tui::BannerInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        ..cust_tui::BannerInfo::default()
    };
    // Loading can fail before onboarding (no API key); show the defaults then.
    let config = ConfigLoader::load(PartialConfig::default())
        .unwrap_or_else(|_| cust_config_types::Config::default());
    info.provider = config.provider;
    info.model = config.model;
    if let Ok(profile) = std::env::var("CUST_SANDBOX") {
        info.sandbox = cust_tui::SandboxStatus::from_label(&profile);
    }
    print!(
        "{}",
        cust_tui::banner::render_text(&info, cust_tui::banner::terminal_width())
    );
}

async fn handle_ask(args: &[String]) -> Result<(), anyhow::Error> {
    let parsed = parse_args(args)?;
    if parsed.prompt.is_empty() {
        anyhow::bail!(
            "Error: missing prompt argument for 'cust ask'. Usage: cust ask \"<prompt>\""
        );
    }

    let prompt = parsed.prompt;
    let config = ConfigLoader::load(parsed.config)?;

    let client = cust_provider::ProviderClient::from_config(&config)?;
    let mut stream = client.stream_chat_prompt(&prompt);

    let mut stdout = std::io::stdout();
    let mut received_any = false;

    while let Some(chunk_res) = stream.next().await {
        match chunk_res {
            Ok(chunk) => {
                print!("{chunk}");
                stdout.flush()?;
                received_any = true;
            }
            Err(e) => {
                if received_any {
                    println!();
                }
                anyhow::bail!("Stream error: {e}");
            }
        }
    }

    if received_any {
        println!();
    }

    Ok(())
}

/// Flags shared by `run`, `ask`, and `tui`.
struct CliArgs {
    config: PartialConfig,
    prompt: String,
    auto_approve: bool,
    sandbox: cust_exec::SandboxProfile,
}

fn parse_args(args: &[String]) -> Result<CliArgs, anyhow::Error> {
    let mut parsed = CliArgs {
        config: PartialConfig::default(),
        prompt: String::new(),
        auto_approve: false,
        sandbox: cust_exec::SandboxProfile::default(),
    };
    let mut prompt_parts: Vec<&str> = Vec::new();
    let mut i = 0;

    while i < args.len() {
        // Flags that take a value; None means the value was missing.
        let value = || args.get(i + 1).cloned();
        match args[i].as_str() {
            "--provider" => {
                parsed.config.provider = value();
                i += 1;
            }
            "--model" => {
                parsed.config.model = value();
                i += 1;
            }
            "--api-key" => {
                parsed.config.api_key = value();
                i += 1;
            }
            "--base-url" => {
                parsed.config.base_url = value();
                i += 1;
            }
            "--sandbox" => {
                if let Some(profile) = value() {
                    parsed.sandbox = profile
                        .parse()
                        .map_err(|e| anyhow::anyhow!("Invalid --sandbox profile: {e}"))?;
                }
                i += 1;
            }
            "-y" | "--yes" => parsed.auto_approve = true,
            arg => prompt_parts.push(arg),
        }
        i += 1;
    }

    parsed.prompt = prompt_parts.join(" ");
    Ok(parsed)
}

async fn handle_tui(args: &[String]) -> Result<(), anyhow::Error> {
    let parsed = parse_args(args)?;
    let config = ConfigLoader::load(parsed.config)?;
    let registry = ToolRegistry::with_default_tools_sandboxed(parsed.sandbox);

    let mut state = cust_tui::TuiState::default();
    if let Ok(cwd) = std::env::current_dir() {
        state.workspace = cwd
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| cwd.display().to_string());
        state.git_branch = cust_core::GitTracker::fast_status(&cwd).branch;
        state.banner.workspace_path = Some(cwd.display().to_string());
    }
    // USERNAME on Windows, USER elsewhere; absent just falls back to a
    // plain "Welcome" instead of "Welcome back {name}!".
    state.banner.user_name = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .ok()
        .filter(|n| !n.trim().is_empty());
    state.set_session_info(
        config.provider.clone(),
        config.model.clone(),
        cust_tui::SandboxStatus::from_label(&parsed.sandbox.to_string()),
    );

    // -y starts the session in bypass mode; Shift+Tab cycles from there.
    if parsed.auto_approve {
        state
            .permission_mode
            .set(cust_tui::PermissionMode::BypassPermissions);
    }

    let agent_loop = AgentLoop::new(config, registry);
    // The stdin prompt cannot be used while the alternate screen is up, so
    // requests are answered by the current permission mode instead.
    let approval_handler = TuiApprovalHandler {
        mode: state.permission_mode.clone(),
    };
    cust_tui::run(&agent_loop, &approval_handler, state).await
}

struct TuiApprovalHandler {
    mode: cust_tui::SharedPermissionMode,
}

impl ApprovalHandler for TuiApprovalHandler {
    fn request_approval(&self, _tool: &str, request: &PermissionRequest) -> bool {
        self.mode.get().allows(request)
    }
}

async fn handle_run(args: &[String]) -> Result<(), anyhow::Error> {
    let parsed = parse_args(args)?;
    if parsed.prompt.is_empty() {
        anyhow::bail!("Error: missing prompt argument. Usage: cust \"<prompt>\"");
    }

    let prompt = parsed.prompt;
    let auto_approve = parsed.auto_approve;
    let config = ConfigLoader::load(parsed.config)?;
    let registry = ToolRegistry::with_default_tools_sandboxed(parsed.sandbox);

    let agent_loop = AgentLoop::new(config, registry);
    let approval_handler = CliApprovalHandler { auto_approve };

    let mut event_stream = agent_loop.run_turn(&prompt, &approval_handler);
    let mut stdout = std::io::stdout();

    while let Some(event) = event_stream.next().await {
        match event.kind {
            EventKind::TurnStarted { turn: _ } => {
                // Header if needed
            }
            EventKind::AssistantDelta { text } => {
                print!("{text}");
                stdout.flush()?;
            }
            EventKind::ReasoningDelta { text } => {
                print!("{text}");
                stdout.flush()?;
            }
            EventKind::ToolCall {
                effect: _,
                tool,
                args,
            } => {
                println!("\n[Tool Call: {tool} with args: {args}]");
            }
            EventKind::ApprovalRequested {
                effect: _,
                tool: _,
                request: _,
            } => {
                // Handled directly inside request_approval callback
            }
            EventKind::ToolStream { effect: _, chunk } => {
                print!("{chunk}");
                stdout.flush()?;
            }
            EventKind::ToolResult {
                effect: _,
                ok,
                summary,
            } => {
                let status = if ok { "SUCCESS" } else { "FAILED" };
                println!("[Tool Result ({status}): {summary}]");
            }
            EventKind::TurnEnded { turn: _, reason: _ } => {
                println!();
            }
            EventKind::Error {
                recoverable: _,
                message,
            } => {
                eprintln!("\nError: {message}");
            }
        }
    }

    Ok(())
}
