//! Terminal event loop: raw mode, key handling, and agent event pumping.
//!
//! Renders inline through [`crate::ink::Differ`] rather than into an
//! alternate screen (37e) — finished frames stay in the terminal's own
//! scrollback instead of being discarded when the TUI exits.
//!
//! Key mapping lives in [`map_key`], a pure function, so the interaction rules
//! are testable without a terminal attached.

use crate::app::{SlashOutcome, TuiState};
use crate::ink::terminal::ProcessTerminal;
use crate::ink::{Differ, Terminal};
use crossterm::{
    event::{Event as TermEvent, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use cust_core::{AgentLoop, ApprovalHandler, Event};
use futures_util::{Stream, StreamExt};
use std::pin::Pin;

/// What a keypress means, resolved against the current state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyAction {
    /// Leave the TUI.
    Quit,
    /// Abort the in-flight turn, or clear the input when idle.
    Cancel,
    /// Switch between agent and terminal-shell views.
    ToggleShell,
    /// Advance to the next permission mode.
    CyclePermissions,
    /// Send the buffered input as a prompt.
    Submit(String),
    Insert(char),
    Backspace,
    Ignore,
}

/// Resolve a key event into an action. Pure: no terminal, no side effects.
pub fn map_key(key: KeyEvent, state: &TuiState) -> KeyAction {
    // Windows reports both press and release; act on press only.
    if key.kind == KeyEventKind::Release {
        return KeyAction::Ignore;
    }

    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Char('d') if ctrl => KeyAction::Quit,
        KeyCode::Char('c') if ctrl => KeyAction::Cancel,
        KeyCode::Char('x') if ctrl => KeyAction::ToggleShell,
        KeyCode::Esc => KeyAction::Cancel,
        // Terminals report Shift+Tab as BackTab; some send Tab+SHIFT instead.
        KeyCode::BackTab => KeyAction::CyclePermissions,
        KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => KeyAction::CyclePermissions,
        KeyCode::Enter => {
            let prompt = state.input_buffer.trim();
            if prompt.is_empty() {
                KeyAction::Ignore
            } else {
                KeyAction::Submit(prompt.to_string())
            }
        }
        KeyCode::Backspace => KeyAction::Backspace,
        // Ctrl-modified characters are shortcuts, never literal input.
        KeyCode::Char(c) if !ctrl => KeyAction::Insert(c),
        _ => KeyAction::Ignore,
    }
}

/// Run a `!`-prefixed command through the same sandboxed executor the
/// agent's own tools use, and log its output.
async fn run_shell_command(state: &mut TuiState, cmd: &str) {
    if cmd.is_empty() {
        state.log_shell_error(&anyhow::anyhow!("usage: !<command>"));
        return;
    }
    state.log_shell_command(cmd);

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let profile = state.sandbox_profile();
    match cust_exec::CommandRunner::run_cmd(cmd, &cwd, profile).await {
        Ok(result) => state.log_shell_result(&result),
        Err(err) => state.log_shell_error(&err),
    }
}

/// Restores raw mode on drop, including on panic or early return.
///
/// Unlike the old ratatui guard, this does not enter/leave an alternate
/// screen — the frame is written inline, so there is nothing to restore
/// beyond the terminal mode itself.
struct RawModeGuard;

impl RawModeGuard {
    fn enter() -> anyhow::Result<Self> {
        enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

/// Run the interactive TUI until the user quits.
///
/// Key events and agent events are pumped from the same loop, so a running
/// turn stays cancellable and the display keeps updating while it streams.
pub async fn run(
    agent: &AgentLoop,
    approvals: &dyn ApprovalHandler,
    mut state: TuiState,
) -> anyhow::Result<()> {
    let _guard = RawModeGuard::enter()?;
    let mut terminal = ProcessTerminal::new();
    let mut differ = Differ::new();
    let mut keys = EventStream::new();
    // Some(stream) while a turn is in flight.
    let mut turn: Option<Pin<Box<dyn Stream<Item = Event> + Send + '_>>> = None;

    loop {
        let (width, height) = terminal.size();
        let lines = crate::ui::render(&state, width);
        let frame = differ.frame(lines, width, height);
        if !frame.output.is_empty() {
            terminal.write(&frame.output);
            terminal.flush()?;
        }

        tokio::select! {
            // Bias toward keys so Ctrl+C stays responsive under a busy stream.
            biased;

            maybe_key = keys.next() => {
                let key = match maybe_key {
                    Some(Ok(TermEvent::Key(key))) => key,
                    // Resize, mouse, focus, or a transient read error: redraw.
                    Some(_) => continue,
                    // Key stream closed — nothing left to drive the loop.
                    None => break,
                };
                match map_key(key, &state) {
                    KeyAction::Quit => break,
                    KeyAction::Cancel => {
                        if turn.take().is_some() {
                            state.status = "Cancelled".to_string();
                        } else {
                            state.on_input_change(String::new());
                        }
                    }
                    KeyAction::ToggleShell => { state.toggle_view_mode(); }
                    KeyAction::CyclePermissions => { state.cycle_permission_mode(); }
                    KeyAction::Submit(line) => {
                        state.on_input_change(String::new());
                        if let Some(cmd) = line.strip_prefix('!') {
                            run_shell_command(&mut state, cmd.trim()).await;
                            continue;
                        }
                        // Slash commands the TUI owns never reach the model.
                        match state.handle_slash(&line) {
                            SlashOutcome::Consumed => {}
                            SlashOutcome::Quit => break,
                            SlashOutcome::Prompt(prompt) => {
                                state.assistant_text.clear();
                                turn = Some(agent.run_turn_owned(prompt, approvals));
                            }
                        }
                    }
                    KeyAction::Insert(c) => {
                        let mut buf = state.input_buffer.clone();
                        buf.push(c);
                        state.on_input_change(buf);
                    }
                    KeyAction::Backspace => {
                        let mut buf = state.input_buffer.clone();
                        buf.pop();
                        state.on_input_change(buf);
                    }
                    KeyAction::Ignore => {}
                }
            }

            agent_event = async {
                match turn.as_mut() {
                    Some(stream) => stream.next().await,
                    // No turn running: never resolve, so only keys wake us.
                    None => std::future::pending().await,
                }
            } => {
                match agent_event {
                    Some(event) => state.handle_event(event),
                    // Stream exhausted: the turn is over.
                    None => turn = None,
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn control_shortcuts_map_to_actions() {
        let state = TuiState::default();
        assert_eq!(
            map_key(press(KeyCode::Char('d'), KeyModifiers::CONTROL), &state),
            KeyAction::Quit
        );
        assert_eq!(
            map_key(press(KeyCode::Char('c'), KeyModifiers::CONTROL), &state),
            KeyAction::Cancel
        );
        assert_eq!(
            map_key(press(KeyCode::Char('x'), KeyModifiers::CONTROL), &state),
            KeyAction::ToggleShell
        );
        assert_eq!(
            map_key(press(KeyCode::Esc, KeyModifiers::NONE), &state),
            KeyAction::Cancel
        );
    }

    #[test]
    fn plain_characters_are_input_but_control_ones_are_not() {
        let state = TuiState::default();
        assert_eq!(
            map_key(press(KeyCode::Char('x'), KeyModifiers::NONE), &state),
            KeyAction::Insert('x')
        );
        // Ctrl+Z is unbound — it must not land in the buffer as 'z'.
        assert_eq!(
            map_key(press(KeyCode::Char('z'), KeyModifiers::CONTROL), &state),
            KeyAction::Ignore
        );
    }

    #[test]
    fn enter_submits_only_non_empty_trimmed_input() {
        let mut state = TuiState::default();
        assert_eq!(
            map_key(press(KeyCode::Enter, KeyModifiers::NONE), &state),
            KeyAction::Ignore
        );

        state.on_input_change("   ".to_string());
        assert_eq!(
            map_key(press(KeyCode::Enter, KeyModifiers::NONE), &state),
            KeyAction::Ignore
        );

        state.on_input_change("  hello  ".to_string());
        assert_eq!(
            map_key(press(KeyCode::Enter, KeyModifiers::NONE), &state),
            KeyAction::Submit("hello".to_string())
        );
    }

    #[test]
    fn shift_tab_cycles_permission_modes() {
        let state = TuiState::default();
        assert_eq!(
            map_key(press(KeyCode::BackTab, KeyModifiers::SHIFT), &state),
            KeyAction::CyclePermissions
        );
        assert_eq!(
            map_key(press(KeyCode::Tab, KeyModifiers::SHIFT), &state),
            KeyAction::CyclePermissions
        );
        // A plain Tab is not a mode switch.
        assert_eq!(
            map_key(press(KeyCode::Tab, KeyModifiers::NONE), &state),
            KeyAction::Ignore
        );
    }

    #[test]
    fn key_releases_are_ignored() {
        let state = TuiState::default();
        let mut key = press(KeyCode::Char('a'), KeyModifiers::NONE);
        key.kind = KeyEventKind::Release;
        assert_eq!(map_key(key, &state), KeyAction::Ignore);
    }
}
