use crate::ink::Component;

/// Renders `lines` blank rows. Port of `pi-tui`'s `Spacer`.
#[derive(Debug, Clone)]
pub struct Spacer {
    lines: usize,
}

impl Default for Spacer {
    fn default() -> Self {
        Self::new(1)
    }
}

impl Spacer {
    pub fn new(lines: usize) -> Self {
        Self { lines }
    }

    pub fn set_lines(&mut self, lines: usize) {
        self.lines = lines;
    }
}

impl Component for Spacer {
    fn render(&mut self, _width: usize) -> Vec<String> {
        vec![String::new(); self.lines]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_requested_blank_rows() {
        assert_eq!(Spacer::new(3).render(80).len(), 3);
        assert_eq!(Spacer::new(0).render(80).len(), 0);
    }
}
