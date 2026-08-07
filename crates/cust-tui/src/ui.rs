use crate::app::{TuiState, ViewMode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Gauge, Paragraph},
};

pub fn render(frame: &mut Frame<'_>, state: &TuiState) {
    let area = frame.area();
    let banner_height = if state.show_banner {
        crate::banner::height_for(&state.banner, area.width)
    } else {
        0
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(banner_height), // Welcome Banner (0 once a turn starts)
            Constraint::Length(3),             // Header Dashboard
            Constraint::Length(3),             // Context Memory Gauge
            Constraint::Min(5),                // Assistant Body / Logs
            Constraint::Length(4),             // Prompt Input Box & Slash Menu
            Constraint::Length(1),             // Permission Mode Footer
        ])
        .split(area);

    if state.show_banner && chunks[0].height > 0 {
        crate::banner::render(frame, chunks[0], &state.banner);
    }
    let chunks = &chunks[1..];

    // 1. Header Dashboard
    let mode_str = match state.view_mode {
        ViewMode::Agent => "[Agent Mode]",
        ViewMode::TerminalShell => "[Shell Mode]",
    };
    let header_text = format!(
        " cust TUI — Status: {} | Model: {} ({}) | Mode: {}",
        state.status, state.active_model, state.active_provider, mode_str
    );
    let header = Paragraph::new(header_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Header Dashboard ")
            .style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(header, chunks[0]);

    // 2. Live Context Memory Gauge
    let percent = state.memory_percent();
    let memory_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Context Window Memory Budget "),
        )
        .gauge_style(
            Style::default()
                .fg(if percent > 80 {
                    Color::Red
                } else {
                    Color::Green
                })
                .add_modifier(Modifier::BOLD),
        )
        .percent(percent)
        .label(format!(
            "{}% ({}/{} tokens)",
            percent, state.current_tokens, state.max_context_tokens
        ));
    frame.render_widget(memory_gauge, chunks[1]);

    // 3. Body View
    let body_text = if state.assistant_text.is_empty() {
        state.logs.join("\n")
    } else {
        format!("{}\n\n--- Logs ---\n{}", state.assistant_text, state.logs.join("\n"))
    };
    let body_title = match state.view_mode {
        ViewMode::Agent => " Assistant & Tool Execution Stream ",
        ViewMode::TerminalShell => " Terminal Shell Output Stream ",
    };
    let body = Paragraph::new(body_text).block(Block::default().borders(Borders::ALL).title(body_title));
    frame.render_widget(body, chunks[2]);

    // 4. Input & Slash Menu Box
    let input_title = if state.slash_suggestions.is_empty() {
        " Input Prompt (Type / for Slash Commands) "
    } else {
        " Slash Commands Autocomplete Menu "
    };
    let input_display = if state.slash_suggestions.is_empty() {
        state.input_buffer.clone()
    } else {
        format!(
            "{}\nSuggestions:\n{}",
            state.input_buffer,
            state.slash_suggestions.join(" | ")
        )
    };
    let input_box = Paragraph::new(input_display).block(
        Block::default()
            .borders(Borders::ALL)
            .title(input_title)
            .style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(input_box, chunks[3]);

    // 5. Status line — model, workspace, branch, context, permission mode.
    crate::statusline::render(frame, chunks[4], state, &state.statusline);
}
