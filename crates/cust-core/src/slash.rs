use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
    pub category: String,
}

pub struct SlashRegistry {
    commands: Vec<SlashCommand>,
}

impl Default for SlashRegistry {
    fn default() -> Self {
        let commands = vec![
            SlashCommand {
                name: "help".to_string(),
                description: "Show available slash commands and usage".to_string(),
                category: "General".to_string(),
            },
            SlashCommand {
                name: "compact".to_string(),
                description: "Trigger context compaction for active session".to_string(),
                category: "Session".to_string(),
            },
            SlashCommand {
                name: "clear".to_string(),
                description: "Clear terminal buffer and display".to_string(),
                category: "General".to_string(),
            },
            SlashCommand {
                name: "rewind".to_string(),
                description: "Rewind session history (Fork or InPlace)".to_string(),
                category: "Session".to_string(),
            },
            SlashCommand {
                name: "refine".to_string(),
                description: "Run trajectory refinement and memory update".to_string(),
                category: "Memory".to_string(),
            },
            SlashCommand {
                name: "goal".to_string(),
                description: "Track long-running task policy gates".to_string(),
                category: "Agent".to_string(),
            },
            SlashCommand {
                name: "schedule".to_string(),
                description: "Schedule background timers and recurring crons".to_string(),
                category: "Agent".to_string(),
            },
            SlashCommand {
                name: "model".to_string(),
                description: "Switch active LLM provider or model target".to_string(),
                category: "Config".to_string(),
            },
            SlashCommand {
                name: "skills".to_string(),
                description: "Display progressive disclosure skills catalog".to_string(),
                category: "Skills".to_string(),
            },
            SlashCommand {
                name: "tui".to_string(),
                description: "Toggle rich TUI dashboard mode".to_string(),
                category: "UI".to_string(),
            },
            SlashCommand {
                name: "statusline".to_string(),
                description: "Set up Cust Code status line UI".to_string(),
                category: "UI".to_string(),
            },
            SlashCommand {
                name: "init".to_string(),
                description: "Initialize a new CUST.md file with codebase documentation"
                    .to_string(),
                category: "Project".to_string(),
            },
            SlashCommand {
                name: "review".to_string(),
                description: "Review the working tree diff for bugs and cleanups".to_string(),
                category: "Project".to_string(),
            },
            SlashCommand {
                name: "permissions".to_string(),
                description: "Show or change the tool permission mode".to_string(),
                category: "Config".to_string(),
            },
            SlashCommand {
                name: "sandbox".to_string(),
                description: "Show the active sandbox profile".to_string(),
                category: "Config".to_string(),
            },
            SlashCommand {
                name: "status".to_string(),
                description: "Show provider, model, sandbox, and context usage".to_string(),
                category: "Session".to_string(),
            },
            SlashCommand {
                name: "resume".to_string(),
                description: "Resume a saved session by id".to_string(),
                category: "Session".to_string(),
            },
            SlashCommand {
                name: "export".to_string(),
                description: "Export the current session transcript to a file".to_string(),
                category: "Session".to_string(),
            },
            SlashCommand {
                name: "cost".to_string(),
                description: "Show token usage and estimated cost for this session".to_string(),
                category: "Session".to_string(),
            },
            SlashCommand {
                name: "memory".to_string(),
                description: "View and edit persistent project memory".to_string(),
                category: "Memory".to_string(),
            },
            SlashCommand {
                name: "mcp".to_string(),
                description: "List connected MCP servers and their tools".to_string(),
                category: "Tools".to_string(),
            },
            SlashCommand {
                name: "agents".to_string(),
                description: "List and manage subagents".to_string(),
                category: "Agent".to_string(),
            },
            SlashCommand {
                name: "doctor".to_string(),
                description: "Diagnose configuration, credentials, and connectivity".to_string(),
                category: "Config".to_string(),
            },
            SlashCommand {
                name: "quit".to_string(),
                description: "Exit the TUI".to_string(),
                category: "General".to_string(),
            },
        ];
        Self { commands }
    }
}

impl SlashRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn list(&self) -> &[SlashCommand] {
        &self.commands
    }

    pub fn match_prefix(&self, prefix: &str) -> Vec<SlashCommand> {
        let clean = prefix.trim_start_matches('/');
        self.commands
            .iter()
            .filter(|cmd| cmd.name.starts_with(clean))
            .cloned()
            .collect()
    }

    /// Expand a slash command into the prompt the agent should receive.
    ///
    /// Returns `None` for input that is not a command with a canned prompt —
    /// the caller either handles it locally or sends it through unchanged.
    pub fn expand(&self, input: &str) -> Option<String> {
        let (name, args) = self.parse_input(input)?;
        let prompt = match name {
            "init" => "Analyze this codebase and write a CUST.md at the repository root \
                 documenting its architecture, the crate layout, how to build and test it, \
                 and the conventions a new contributor must follow. Read the existing files \
                 before writing, and keep it concise."
                .to_string(),
            "review" => "Review the current working tree diff. Report correctness bugs first, \
                 then reuse and simplification opportunities. Cite file and line for each finding."
                .to_string(),
            "doctor" => "Check the cust configuration: which provider and model are active, \
                 whether credentials resolve, and whether the workspace looks healthy. \
                 Report anything misconfigured."
                .to_string(),
            _ => return None,
        };

        if args.is_empty() {
            Some(prompt)
        } else {
            Some(format!("{prompt}\n\nAdditional instructions: {args}"))
        }
    }

    pub fn parse_input<'a>(&self, input: &'a str) -> Option<(&'a str, &'a str)> {
        let trimmed = input.trim();
        if !trimmed.starts_with('/') {
            return None;
        }

        let cmd_body = &trimmed[1..];
        let mut parts = cmd_body.splitn(2, ' ');
        let name = parts.next().unwrap_or("");
        let args = parts.next().unwrap_or("").trim();

        Some((name, args))
    }
}
