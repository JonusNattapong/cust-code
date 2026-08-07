use cust_tui::{TuiState, ViewMode};

#[test]
fn test_rich_tui_state_memory_and_slash() {
    let mut state = TuiState::default();

    // 1. Memory budget percentage calculation
    state.current_tokens = 64_000;
    state.max_context_tokens = 128_000;
    assert_eq!(state.memory_percent(), 50);

    // 2. Input change triggers slash command suggestions
    state.on_input_change("/g".to_string());
    assert!(!state.slash_suggestions.is_empty());
    assert!(state.slash_suggestions.iter().any(|s| s.contains("goal")));

    // 3. View mode toggle
    assert_eq!(state.view_mode, ViewMode::Agent);
    state.toggle_view_mode();
    assert_eq!(state.view_mode, ViewMode::TerminalShell);
}
