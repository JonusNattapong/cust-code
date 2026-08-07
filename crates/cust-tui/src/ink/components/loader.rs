use crate::ink::Component;

/// The default braille spinner frames, matching `pi-tui`'s `Loader`.
pub const DEFAULT_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// An animated single-row spinner with a label.
///
/// The component holds no timer: the caller advances it with [`Loader::tick`]
/// on whatever cadence it already renders at, so animation stays in step with
/// the frame loop instead of racing it.
#[derive(Debug, Clone)]
pub struct Loader {
    frames: Vec<String>,
    index: usize,
    label: String,
    active: bool,
}

impl Loader {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            frames: DEFAULT_FRAMES.iter().map(|s| s.to_string()).collect(),
            index: 0,
            label: label.into(),
            active: true,
        }
    }

    pub fn with_frames(mut self, frames: Vec<String>) -> Self {
        if !frames.is_empty() {
            self.frames = frames;
            self.index = 0;
        }
        self
    }

    pub fn set_label(&mut self, label: impl Into<String>) {
        self.label = label.into();
    }

    /// Advance to the next spinner frame.
    pub fn tick(&mut self) {
        self.index = (self.index + 1) % self.frames.len();
    }

    /// Stop animating and render nothing.
    pub fn stop(&mut self) {
        self.active = false;
    }

    pub fn start(&mut self) {
        self.active = true;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl Component for Loader {
    fn render(&mut self, width: usize) -> Vec<String> {
        if !self.active {
            return Vec::new();
        }
        let frame = &self.frames[self.index];
        let line = if self.label.is_empty() {
            frame.clone()
        } else {
            format!("{frame} {}", self.label)
        };
        vec![crate::ink::utils::truncate_to_width(&line, width, Some("…")).text]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ink::utils::visible_width;

    #[test]
    fn renders_a_frame_and_the_label() {
        let mut l = Loader::new("Thinking");
        let out = l.render(40);
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("Thinking"));
        assert!(out[0].starts_with('⠋'));
    }

    #[test]
    fn tick_cycles_and_wraps() {
        let mut l = Loader::new("").with_frames(vec!["a".into(), "b".into()]);
        assert_eq!(l.render(10), vec!["a"]);
        l.tick();
        assert_eq!(l.render(10), vec!["b"]);
        l.tick();
        assert_eq!(l.render(10), vec!["a"]);
    }

    #[test]
    fn stopped_loader_renders_nothing() {
        let mut l = Loader::new("x");
        l.stop();
        assert!(l.render(10).is_empty());
        l.start();
        assert!(!l.render(10).is_empty());
    }

    #[test]
    fn long_labels_are_clipped_to_width() {
        let mut l = Loader::new("a very long status message indeed");
        let out = l.render(12);
        assert_eq!(visible_width(&out[0]), 12);
    }

    #[test]
    fn empty_frame_list_is_ignored() {
        let mut l = Loader::new("x").with_frames(vec![]);
        assert!(!l.render(20)[0].is_empty());
    }
}
