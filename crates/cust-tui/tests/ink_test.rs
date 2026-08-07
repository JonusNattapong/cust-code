use cust_tui::{InkColor, InkNode, InkRenderer};

#[test]
fn test_ink_component_rendering() {
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
