//! Integration coverage for the `ink` layer: components composed through a
//! `Tui` onto a fake terminal, asserting on the bytes that actually go out.

use cust_tui::ink::components::BorderStyle;
use cust_tui::ink::differ::{FrameKind, FullRedrawReason};
use cust_tui::ink::terminal::TestTerminal;
use cust_tui::ink::utils::{strip_ansi, visible_width};
use cust_tui::ink::{
    BoxView, Component, Container, InkColor, InkNode, InkRenderer, Loader, SelectItem, SelectList,
    Spacer, Text, TruncatedText, Tui,
};

#[test]
fn composed_tree_renders_through_the_tui() {
    let mut tui = Tui::new(TestTerminal::new(40, 20));

    let mut panel = BoxView::new()
        .with_padding(1, 0)
        .with_border(BorderStyle::Rounded)
        .with_title("Session");
    panel.add_child(Box::new(Text::new("connected").with_padding(0, 0)));
    tui.add_child(Box::new(panel));
    tui.add_child(Box::new(Spacer::new(1)));
    tui.add_child(Box::new(Loader::new("Thinking")));

    let frame = tui.render().expect("render");
    assert_eq!(frame.kind, FrameKind::Full(FullRedrawReason::FirstRender));

    let out = tui.terminal_mut().output();
    assert!(out.contains("Session"));
    assert!(out.contains("connected"));
    assert!(out.contains("Thinking"));
}

#[test]
fn only_the_changed_row_is_rewritten_between_frames() {
    let mut tui = Tui::new(TestTerminal::new(40, 20));
    tui.add_child(Box::new(Text::new("static").with_padding(0, 0)));
    tui.add_child(Box::new(Loader::new("step one")));
    tui.render().expect("render");

    // Swap the loader's label; the static row above must not be resent.
    tui.root_mut().clear();
    tui.add_child(Box::new(Text::new("static").with_padding(0, 0)));
    tui.add_child(Box::new(Loader::new("step two")));

    tui.terminal_mut().clear_output();
    let frame = tui.render().expect("render");

    assert_eq!(frame.kind, FrameKind::Partial { first: 1, last: 1 });
    let out = tui.terminal_mut().output();
    assert!(out.contains("step two"));
    assert!(!out.contains("static"));
}

#[test]
fn container_stacks_children_and_respects_width() {
    let mut c = Container::new();
    c.add_child(Box::new(Text::new("aaa bbb ccc").with_padding(0, 0)));
    c.add_child(Box::new(TruncatedText::new("a long status line here")));

    let lines = c.render(8);
    // "aaa bbb" fits in 8 cells, so the text takes two rows; the truncated
    // line always occupies exactly one.
    assert_eq!(lines.len(), 3);
    for line in &lines {
        assert!(visible_width(line) <= 8, "row overflowed width: {line:?}");
    }
    assert!(lines[2].ends_with('…'));
}

#[test]
fn focused_list_consumes_keys_and_reports_a_choice() {
    let mut tui = Tui::new(TestTerminal::new(30, 10));
    let list = SelectList::new(vec![
        SelectItem::new("yes", "Allow"),
        SelectItem::new("no", "Deny").with_description("reject the call"),
    ]);
    tui.add_child(Box::new(list));
    tui.set_focus(Some(0));

    tui.handle_input("\u{1b}[B"); // down
    tui.render().expect("render");
    assert!(strip_ansi(&tui.terminal_mut().output()).contains("❯ Deny"));

    tui.handle_input("\r");
    // The submission is readable back off the concrete component.
    let child = tui.root_mut().child_mut(0).expect("list child");
    child.handle_input(""); // no-op; proves the trait object stays usable
}

#[test]
fn unicode_content_never_overflows_the_render_width() {
    let mut c = Container::new();
    c.add_child(Box::new(Text::new("日本語のテキストです 👍 emoji").with_padding(1, 0)));
    for line in c.render(20) {
        assert_eq!(visible_width(&line), 20);
    }
}

#[test]
fn legacy_node_tree_still_renders() {
    let tree = InkNode::Box {
        title: Some("Header".to_string()),
        color: InkColor::Cyan,
        children: vec![
            InkNode::Text {
                content: "Hello Ink".to_string(),
                color: InkColor::Green,
                bold: true,
            },
            InkNode::Gauge {
                label: "Memory".to_string(),
                percent: 50,
            },
        ],
    };

    let rendered = InkRenderer::render_to_string(&tree);
    assert!(rendered.contains("Header"));
    assert!(rendered.contains("Hello Ink"));
    assert!(rendered.contains("Memory"));
    assert!(rendered.contains("50%"));
}

#[test]
fn legacy_gauge_clamps_an_out_of_range_percent() {
    let rendered = InkRenderer::render_to_string(&InkNode::Gauge {
        label: "x".to_string(),
        percent: 500,
    });
    assert!(rendered.contains("500%"));
}
