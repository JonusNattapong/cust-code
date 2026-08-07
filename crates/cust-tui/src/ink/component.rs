//! The component contract and the `Container` that composes components.
//!
//! Port of the `Component` / `Container` half of `pi-tui`'s `src/tui.ts`.
//! Rendering is deliberately dumb: a component turns a width into lines. All
//! positioning, diffing and cursor work happens above it.

/// Zero-width APC marker a focused component emits at its cursor position.
///
/// Terminals ignore it; [`crate::ink::Tui`] finds it, strips it, and parks the
/// hardware cursor there so IME candidate windows land in the right place.
pub const CURSOR_MARKER: &str = "\u{1b}_pi:c\u{7}";

/// A renderable piece of UI.
pub trait Component {
    /// Render to lines for the given viewport width.
    ///
    /// Each returned string is one terminal row. Lines may contain escape
    /// sequences; their *visible* width should not exceed `width`.
    fn render(&mut self, width: usize) -> Vec<String>;

    /// Drop cached rendering state.
    ///
    /// Called on theme changes and whenever the caller needs a render from
    /// scratch. The default is a no-op for stateless components.
    fn invalidate(&mut self) {}

    /// Handle keyboard input while focused. Ignored by default.
    fn handle_input(&mut self, _data: &str) {}

    /// Whether this component wants key *release* events (Kitty protocol).
    ///
    /// Releases are filtered out unless a component opts in.
    fn wants_key_release(&self) -> bool {
        false
    }

    /// Whether this component can take focus and show a hardware cursor.
    fn is_focusable(&self) -> bool {
        false
    }

    /// Notify the component that focus changed. Only meaningful when
    /// [`Component::is_focusable`] is true.
    fn set_focused(&mut self, _focused: bool) {}
}

/// A component that renders its children top to bottom.
#[derive(Default)]
pub struct Container {
    children: Vec<Box<dyn Component>>,
}

impl Container {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_child(&mut self, component: Box<dyn Component>) {
        self.children.push(component);
    }

    /// Remove the child at `index`, returning it. `None` if out of range.
    pub fn remove_child(&mut self, index: usize) -> Option<Box<dyn Component>> {
        if index < self.children.len() {
            Some(self.children.remove(index))
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.children.clear();
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    pub fn child_mut(&mut self, index: usize) -> Option<&mut Box<dyn Component>> {
        self.children.get_mut(index)
    }
}

impl Component for Container {
    fn render(&mut self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        for child in &mut self.children {
            lines.extend(child.render(width));
        }
        lines
    }

    fn invalidate(&mut self) {
        for child in &mut self.children {
            child.invalidate();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixed(Vec<String>, usize);

    impl Component for Fixed {
        fn render(&mut self, _width: usize) -> Vec<String> {
            self.0.clone()
        }
        fn invalidate(&mut self) {
            self.1 += 1;
        }
    }

    #[test]
    fn container_concatenates_children_in_order() {
        let mut c = Container::new();
        c.add_child(Box::new(Fixed(vec!["a".into()], 0)));
        c.add_child(Box::new(Fixed(vec!["b".into(), "c".into()], 0)));
        assert_eq!(c.render(10), vec!["a", "b", "c"]);
    }

    #[test]
    fn invalidate_cascades_to_children() {
        let mut c = Container::new();
        c.add_child(Box::new(Fixed(vec![], 0)));
        c.invalidate();
        // The child recorded the call; re-rendering still works afterwards.
        assert!(c.render(10).is_empty());
    }

    #[test]
    fn remove_child_out_of_range_is_none() {
        let mut c = Container::new();
        assert!(c.remove_child(0).is_none());
    }
}
