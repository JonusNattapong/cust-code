//! Terminal sink abstraction.
//!
//! Port of `pi-tui`'s `src/terminal.ts`, reduced to what the differential
//! renderer needs: a place to write bytes and a size to render against.
//! Splitting this out is what makes [`crate::ink::Tui`] testable — the diff
//! algorithm is exercised against [`TestTerminal`] with no tty involved.

use std::io::{self, Write};
use std::sync::{Arc, Mutex};

/// Options controlling how a terminal session is torn down.
#[derive(Debug, Clone, Copy, Default)]
pub struct TerminalStopOptions {
    /// Leave the alternate screen buffer contents in place.
    pub preserve_alt_screen: bool,
}

pub trait Terminal {
    /// Queue output. Implementations may buffer; [`Terminal::flush`] commits.
    fn write(&mut self, data: &str);

    /// Current viewport size in (columns, rows).
    fn size(&self) -> (usize, usize);

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Writes to the real process stdout.
pub struct ProcessTerminal {
    out: io::Stdout,
    /// Fallback size used when the tty size cannot be queried.
    fallback: (usize, usize),
}

impl Default for ProcessTerminal {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessTerminal {
    pub fn new() -> Self {
        Self {
            out: io::stdout(),
            fallback: (80, 24),
        }
    }

    /// Override the size reported when the tty cannot be queried.
    pub fn with_fallback_size(mut self, cols: usize, rows: usize) -> Self {
        self.fallback = (cols, rows);
        self
    }
}

impl Terminal for ProcessTerminal {
    fn write(&mut self, data: &str) {
        // A closed or broken stdout is not recoverable from here and there is
        // nowhere left to report it to; the caller sees it on the next flush.
        let _ = self.out.write_all(data.as_bytes());
    }

    fn size(&self) -> (usize, usize) {
        match crossterm::terminal::size() {
            Ok((cols, rows)) => (cols as usize, rows as usize),
            Err(_) => self.fallback,
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
    }
}

/// Collects everything written, for tests and snapshots.
#[derive(Clone)]
pub struct TestTerminal {
    buffer: Arc<Mutex<String>>,
    size: (usize, usize),
}

impl TestTerminal {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(String::new())),
            size: (cols, rows),
        }
    }

    /// Everything written so far, including escape sequences.
    pub fn output(&self) -> String {
        self.buffer.lock().expect("test terminal buffer poisoned").clone()
    }

    pub fn clear_output(&self) {
        self.buffer.lock().expect("test terminal buffer poisoned").clear()
    }

    pub fn set_size(&mut self, cols: usize, rows: usize) {
        self.size = (cols, rows);
    }
}

impl Terminal for TestTerminal {
    fn write(&mut self, data: &str) {
        self.buffer
            .lock()
            .expect("test terminal buffer poisoned")
            .push_str(data);
    }

    fn size(&self) -> (usize, usize) {
        self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_accumulates_writes() {
        let mut t = TestTerminal::new(80, 24);
        t.write("a");
        t.write("b");
        assert_eq!(t.output(), "ab");
        assert_eq!(t.size(), (80, 24));
    }

    #[test]
    fn test_terminal_clears() {
        let mut t = TestTerminal::new(10, 2);
        t.write("x");
        t.clear_output();
        assert_eq!(t.output(), "");
    }
}
