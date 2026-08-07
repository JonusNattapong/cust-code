use cust_core::{Event, EventKind};
use cust_tui::TuiState;

#[test]
fn test_tui_state_event_handling() {
    let mut state = TuiState::default();

    state.handle_event(Event {
        id: 1,
        generation: 1,
        kind: EventKind::TurnStarted { turn: 1 },
    });
    assert!(state.status.contains("Turn #1"));

    state.handle_event(Event {
        id: 2,
        generation: 1,
        kind: EventKind::AssistantDelta {
            text: "Hello world".to_string(),
        },
    });
    assert_eq!(state.assistant_text, "Hello world");
}

#[test]
fn test_view_mode_toggle() {
    use cust_tui::ViewMode;

    let mut state = TuiState::default();
    assert_eq!(state.view_mode, ViewMode::Agent);

    let mode1 = state.toggle_view_mode();
    assert_eq!(mode1, ViewMode::TerminalShell);

    let mode2 = state.toggle_view_mode();
    assert_eq!(mode2, ViewMode::Agent);
}
