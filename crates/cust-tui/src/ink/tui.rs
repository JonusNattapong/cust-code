//! The top-level TUI driver.
//!
//! Wires a [`Container`] of components to a [`Terminal`] through the
//! [`Differ`]. Port of the orchestration half of `pi-tui`'s `src/tui.ts`,
//! minus overlays, fullscreen and image protocols (not yet ported).

use super::component::Component;
use super::differ::{Differ, Frame};
use super::terminal::Terminal;
use super::Container;

/// Drives render cycles and routes input to the focused component.
pub struct Tui<T: Terminal> {
    terminal: T,
    root: Container,
    differ: Differ,
    /// Index into the root container's children, if any child has focus.
    focused: Option<usize>,
}

impl<T: Terminal> Tui<T> {
    pub fn new(terminal: T) -> Self {
        Self {
            terminal,
            root: Container::new(),
            differ: Differ::new(),
            focused: None,
        }
    }

    pub fn root_mut(&mut self) -> &mut Container {
        &mut self.root
    }

    pub fn terminal_mut(&mut self) -> &mut T {
        &mut self.terminal
    }

    pub fn add_child(&mut self, component: Box<dyn Component>) {
        self.root.add_child(component);
    }

    /// Give focus to the child at `index`, clearing focus from the previous
    /// holder. Passing `None` clears focus entirely.
    pub fn set_focus(&mut self, index: Option<usize>) {
        if let Some(prev) = self.focused {
            if let Some(child) = self.root.child_mut(prev) {
                child.set_focused(false);
            }
        }
        self.focused = index.filter(|i| {
            self.root
                .child_mut(*i)
                .map(|c| c.is_focusable())
                .unwrap_or(false)
        });
        if let Some(i) = self.focused {
            if let Some(child) = self.root.child_mut(i) {
                child.set_focused(true);
            }
        }
    }

    pub fn focused(&self) -> Option<usize> {
        self.focused
    }

    /// Route a chunk of stdin to the focused component.
    ///
    /// Input is dropped when nothing has focus — there is no sensible default
    /// recipient, and silently feeding it to child zero would be worse.
    pub fn handle_input(&mut self, data: &str) {
        let Some(i) = self.focused else { return };
        if let Some(child) = self.root.child_mut(i) {
            child.handle_input(data);
        }
    }

    /// Drop every component's cached render, forcing a rebuild next frame.
    pub fn invalidate(&mut self) {
        self.root.invalidate();
        self.differ.reset();
    }

    /// Render one frame and write the diff to the terminal.
    pub fn render(&mut self) -> std::io::Result<Frame> {
        let (width, height) = self.terminal.size();
        let lines = self.root.render(width);
        let frame = self.differ.frame(lines, width, height);
        if !frame.output.is_empty() {
            self.terminal.write(&frame.output);
            self.terminal.flush()?;
        }
        Ok(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ink::components::{SelectItem, SelectList, Text};
    use crate::ink::differ::{FrameKind, FullRedrawReason};
    use crate::ink::terminal::TestTerminal;

    fn tui() -> Tui<TestTerminal> {
        Tui::new(TestTerminal::new(40, 10))
    }

    #[test]
    fn renders_children_to_the_terminal() {
        let mut t = tui();
        t.add_child(Box::new(Text::new("hello").with_padding(0, 0)));
        let frame = t.render().expect("render");
        assert_eq!(frame.kind, FrameKind::Full(FullRedrawReason::FirstRender));
        assert!(t.terminal_mut().output().contains("hello"));
    }

    #[test]
    fn a_second_identical_frame_writes_nothing() {
        let mut t = tui();
        t.add_child(Box::new(Text::new("hello").with_padding(0, 0)));
        t.render().expect("render");
        t.terminal_mut().clear_output();
        let frame = t.render().expect("render");
        assert_eq!(frame.kind, FrameKind::NoChange);
        assert_eq!(t.terminal_mut().output(), "");
    }

    #[test]
    fn focus_only_lands_on_focusable_children() {
        let mut t = tui();
        t.add_child(Box::new(Text::new("x")));
        t.add_child(Box::new(SelectList::new(vec![SelectItem::new("v", "l")])));
        t.set_focus(Some(0));
        assert_eq!(t.focused(), None); // Text is not focusable
        t.set_focus(Some(1));
        assert_eq!(t.focused(), Some(1));
    }

    #[test]
    fn input_reaches_the_focused_child_only() {
        let mut t = tui();
        t.add_child(Box::new(SelectList::new(vec![
            SelectItem::new("a", "a"),
            SelectItem::new("b", "b"),
        ])));
        // Unfocused: the keypress is dropped.
        t.handle_input("j");
        t.set_focus(Some(0));
        t.handle_input("j");
        t.render().expect("render");
        assert!(t.terminal_mut().output().contains("❯ b"));
    }

    #[test]
    fn invalidate_forces_a_full_repaint() {
        let mut t = tui();
        t.add_child(Box::new(Text::new("hello").with_padding(0, 0)));
        t.render().expect("render");
        t.invalidate();
        let frame = t.render().expect("render");
        assert_eq!(frame.kind, FrameKind::Full(FullRedrawReason::FirstRender));
    }

    #[test]
    fn a_resize_triggers_a_full_repaint() {
        let mut t = tui();
        t.add_child(Box::new(Text::new("hello").with_padding(0, 0)));
        t.render().expect("render");
        t.terminal_mut().set_size(60, 10);
        let frame = t.render().expect("render");
        assert_eq!(frame.kind, FrameKind::Full(FullRedrawReason::WidthChanged));
    }
}
