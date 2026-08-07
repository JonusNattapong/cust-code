//! Slash commands the TUI handles itself, and the ones it forwards.

use cust_tui::{PermissionMode, SlashOutcome, TuiState};

fn last_log(state: &TuiState) -> &str {
    state.logs.last().map(String::as_str).unwrap_or("")
}

#[test]
fn statusline_command_reconfigures_the_status_line() {
    let mut state = TuiState::default();

    assert_eq!(state.handle_slash("/statusline branch off"), SlashOutcome::Consumed);
    assert!(!state.statusline.branch);
    assert!(last_log(&state).contains("branch off"), "{}", last_log(&state));

    // Unknown segments report usage instead of changing anything.
    state.handle_slash("/statusline bogus");
    assert!(last_log(&state).contains("usage:"), "{}", last_log(&state));
    assert!(!state.statusline.branch);
}

#[test]
fn permissions_command_reports_and_sets_the_mode() {
    let mut state = TuiState::default();

    state.handle_slash("/permissions");
    assert!(last_log(&state).contains("ask every time"), "{}", last_log(&state));

    state.handle_slash("/permissions bypass");
    assert_eq!(state.permission_mode.get(), PermissionMode::BypassPermissions);

    state.handle_slash("/permissions nonsense");
    assert!(last_log(&state).contains("usage:"), "{}", last_log(&state));
    // The bad input left the mode untouched.
    assert_eq!(state.permission_mode.get(), PermissionMode::BypassPermissions);
}

#[test]
fn status_command_summarises_the_session() {
    let mut state = TuiState::default();
    state.current_tokens = 64_000;
    state.max_context_tokens = 128_000;

    state.handle_slash("/status");
    let log = last_log(&state);
    assert!(log.contains("openai/gpt-4o"), "{log}");
    assert!(log.contains("ctx 50%"), "{log}");
}

#[test]
fn quit_command_asks_the_loop_to_exit() {
    let mut state = TuiState::default();
    assert_eq!(state.handle_slash("/quit"), SlashOutcome::Quit);
}

#[test]
fn init_expands_into_a_real_prompt_and_plain_text_passes_through() {
    let mut state = TuiState::default();

    let SlashOutcome::Prompt(prompt) = state.handle_slash("/init") else {
        panic!("/init should become a prompt for the agent");
    };
    assert!(prompt.contains("CUST.md"), "{prompt}");
    assert!(prompt.len() > "/init".len());

    // Arguments are appended rather than dropped.
    let SlashOutcome::Prompt(with_args) = state.handle_slash("/init focus on the tui crate") else {
        panic!("expected a prompt");
    };
    assert!(with_args.contains("focus on the tui crate"), "{with_args}");

    // Ordinary input is untouched.
    assert_eq!(
        state.handle_slash("what does cust-core do?"),
        SlashOutcome::Prompt("what does cust-core do?".to_string())
    );
}

#[test]
fn clear_empties_the_transcript() {
    let mut state = TuiState::default();
    state.assistant_text = "hello".to_string();
    state.logs.push("noise".to_string());

    assert_eq!(state.handle_slash("/clear"), SlashOutcome::Consumed);
    assert!(state.assistant_text.is_empty());
    assert!(state.logs.is_empty());
}

#[test]
fn help_lists_every_registered_command() {
    let mut state = TuiState::default();
    state.handle_slash("/help");
    let listing = state.logs.join("\n");

    for expected in ["/init", "/statusline", "/permissions", "/status", "/review"] {
        assert!(listing.contains(expected), "{expected} missing from:\n{listing}");
    }
}
