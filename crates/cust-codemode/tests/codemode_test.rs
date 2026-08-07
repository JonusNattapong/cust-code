use cust_codemode::{CodeEvaluator, HostBridge};
use cust_tools_api::{Availability, PermissionRequest, Tool, ToolError, ToolResult, ToolSpec};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

struct MockReadTool;
impl Tool for MockReadTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "read_file".to_string(),
            description: "Mock read".to_string(),
            parameters: serde_json::json!({}),
        }
    }

    fn permission(&self, _args: &serde_json::Value) -> PermissionRequest {
        PermissionRequest::None
    }

    fn availability(&self) -> Availability {
        Availability::Both
    }

    fn call<'a>(
        &'a self,
        _args: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send + 'a>> {
        Box::pin(async move {
            Ok(ToolResult::success(
                "Mock read content",
                Some(serde_json::json!("file content")),
            ))
        })
    }
}

#[tokio::test]
async fn test_codemode_eval() {
    let mut map: HashMap<String, Arc<dyn Tool>> = HashMap::new();
    map.insert("read_file".to_string(), Arc::new(MockReadTool));

    let bridge = HostBridge::new(map);
    let evaluator = CodeEvaluator::new(bridge);

    let script = "let res = tools.read_file({}); res";
    let output = evaluator.eval_script(script).await.unwrap();

    assert!(output.contains("Mock read content"));
}
