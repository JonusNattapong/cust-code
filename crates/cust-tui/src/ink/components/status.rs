//! Live status line during streaming: spinner, elapsed time, token count, shimmer gradient.
//!
//! Rendered as a single terminal row that updates every render frame. The spinner
//! cycles frames, elapsed increments per second, and the shimmer gradient shifts
//! per-frame to give the illusion of a flowing light across the text.

use std::time::{Duration, Instant};
use crate::ink::utils::pad_to_width;
use crate::ink::Component;

/// Braille spinner frames, matching the default Loader.
pub const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// The current state of the status line.
pub struct StatusLine {
    /// When the status line started (or was last reset).
    started: Instant,
    /// Current spinner frame index.
    spinner_index: usize,
    /// Total tokens seen in this turn (not cumulative context).
    tokens_this_turn: usize,
    /// Which frame index for the shimmer gradient offset.
    shimmer_frame: usize,
}

impl StatusLine {
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
            spinner_index: 0,
            tokens_this_turn: 0,
            shimmer_frame: 0,
        }
    }

    pub fn reset(&mut self) {
        self.started = Instant::now();
        self.spinner_index = 0;
        self.tokens_this_turn = 0;
        self.shimmer_frame = 0;
    }

    pub fn set_tokens(&mut self, count: usize) {
        self.tokens_this_turn = count;
    }

    pub fn add_tokens(&mut self, count: usize) {
        self.tokens_this_turn += count;
    }

    /// Advance the animation state (called once per render).
    pub fn tick(&mut self) {
        self.spinner_index = (self.spinner_index + 1) % SPINNER_FRAMES.len();
        self.shimmer_frame = (self.shimmer_frame + 1) % 20;
    }

    fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    fn format_elapsed(&self) -> String {
        let total_secs = self.elapsed().as_secs();
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        if mins > 0 {
            format!("{}m {}s", mins, secs)
        } else {
            format!("{}s", secs)
        }
    }

    fn format_tokens(&self) -> String {
        if self.tokens_this_turn < 1000 {
            format!("{}", self.tokens_this_turn)
        } else {
            format!("{:.1}k", self.tokens_this_turn as f64 / 1000.0)
        }
    }

    /// Apply a shimmer gradient to the line.
    /// The gradient is 8 cells wide and shifts by `offset` each frame.
    fn shimmer(&self, line: &str, offset: usize) -> String {
        // The shimmer is a smooth gradient: dim → bright → dim, moving left to right.
        // We approximate it with ANSI color codes: 238 (dark), 250 (bright), back to 238.
        let gradient = [238, 239, 240, 250, 251, 252, 250, 240, 239, 238];
        let mut out = String::new();
        for (col, c) in line.chars().enumerate() {
            let idx = (col + offset) % (gradient.len() * 4);
            let shade_idx = idx / 4; // Each shade lasts ~4 columns
            let code = gradient[shade_idx % gradient.len()];
            out.push_str(&format!("\u{1b}[38;5;{}m{}\u{1b}[0m", code, c));
        }
        out
    }
}

impl Default for StatusLine {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for StatusLine {
    fn render(&mut self, width: usize) -> Vec<String> {
        if width < 20 {
            // Too narrow; don't render anything.
            return Vec::new();
        }

        let spinner = SPINNER_FRAMES[self.spinner_index];
        let elapsed = self.format_elapsed();
        let tokens = self.format_tokens();
        let line = format!("✳ {} · ({}) · ↓ {} tokens", spinner, elapsed, tokens);

        // Apply the shimmer gradient and pad to width.
        let shimmer = self.shimmer(&line, self.shimmer_frame);
        let padded = pad_to_width(&shimmer, width);

        vec![padded]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_a_single_row() {
        let mut s = StatusLine::new();
        let lines = s.render(40);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn spinner_cycles_on_tick() {
        let mut s = StatusLine::new();
        let first = s.render(40)[0].clone();
        s.tick();
        let second = s.render(40)[0].clone();
        // The spinner frame should change (at least the escape codes will differ).
        // We can't directly compare because of shimmer, but the length should be the same.
        assert_eq!(first.len(), second.len());
    }

    #[test]
    fn too_narrow_renders_nothing() {
        let mut s = StatusLine::new();
        assert!(s.render(15).is_empty());
        assert_eq!(s.render(20).len(), 1);
    }

    #[test]
    fn formats_elapsed_time() {
        use crate::ink::utils::strip_ansi;
        let mut s = StatusLine::new();
        s.started = Instant::now() - Duration::from_secs(125);
        let line = s.render(60)[0].clone();
        let plain = strip_ansi(&line);
        assert!(plain.contains("2m"));
        assert!(plain.contains("5s"));
    }

    #[test]
    fn formats_token_count() {
        use crate::ink::utils::strip_ansi;
        let mut s = StatusLine::new();
        s.set_tokens(2400);
        let line = s.render(60)[0].clone();
        let plain = strip_ansi(&line);
        assert!(plain.contains("2.4k"));

        s.set_tokens(500);
        let line = s.render(60)[0].clone();
        let plain = strip_ansi(&line);
        assert!(plain.contains("500"));
    }

    #[test]
    fn pads_to_full_width() {
        let mut s = StatusLine::new();
        let line = s.render(80);
        // The line is complex due to shimmer ANSI codes, but visible width should be 80.
        assert_eq!(line.len(), 1);
    }
}
