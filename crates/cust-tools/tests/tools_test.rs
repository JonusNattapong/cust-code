use cust_tools::{ListDirTool, ReadFileTool, ToolRegistry, WriteFileTool};
use cust_tools_api::{PermissionRequest, Tool};

#[tokio::test]
async fn test_read_file_tool() {
    let tool = ReadFileTool;
    assert_eq!(tool.name(), "read_file");

    let args = serde_json::json!({
        "path": "Cargo.toml"
    });

    let perm = tool.permission(&args);
    assert!(matches!(perm, PermissionRequest::ReadPath(_)));

    let result = tool.call(args).await.unwrap();
    assert!(result.ok);
    assert!(result.summary.contains("Cargo.toml"));
}

#[tokio::test]
async fn test_write_file_tool_permission() {
    let tool = WriteFileTool;
    assert_eq!(tool.name(), "write_file");

    let args = serde_json::json!({
        "path": "target/test_output.txt",
        "content": "hello world"
    });

    let perm = tool.permission(&args);
    assert!(matches!(perm, PermissionRequest::WritePath(_)));

    let result = tool.call(args).await.unwrap();
    assert!(result.ok);

    let content = tokio::fs::read_to_string("target/test_output.txt")
        .await
        .unwrap();
    assert_eq!(content, "hello world");
}

#[tokio::test]
async fn test_list_dir_tool() {
    let tool = ListDirTool;
    let args = serde_json::json!({ "path": "." });
    let result = tool.call(args).await.unwrap();
    assert!(result.ok);
    assert!(result.data.unwrap().to_string().contains("Cargo.toml"));
}

#[tokio::test]
async fn test_tool_registry() {
    let registry = ToolRegistry::with_default_tools();
    assert!(registry.get("read_file").is_some());
    assert!(registry.get("write_file").is_some());
    assert!(registry.get("list_dir").is_some());
    assert!(registry.get("search").is_some());
    assert!(registry.get("bash").is_some());
    assert!(registry.get("exec").is_some());
    assert!(registry.get("view_image").is_some());
    assert_eq!(registry.specs().len(), 7);
}
