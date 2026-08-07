use cust_tools::{McpToolSpec, McpToolWrapper};
use cust_tools_api::Tool;

#[tokio::test]
async fn test_mcp_tool_wrapper() {
    let spec = McpToolSpec {
        name: "test_mcp_tool".to_string(),
        description: "Test MCP Tool".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": { "query": { "type": "string" } }
        }),
    };

    let wrapper = McpToolWrapper::new(spec);
    assert_eq!(wrapper.name(), "test_mcp_tool");

    let res = wrapper
        .call(serde_json::json!({ "query": "hello" }))
        .await
        .unwrap();
    assert!(res.ok);
    assert!(res.summary.contains("test_mcp_tool"));
}
