use cust_exec::SandboxProfile;
use cust_tools_api::{Tool, ToolSpec};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn specs(&self) -> Vec<ToolSpec> {
        self.tools.values().map(|t| t.spec()).collect()
    }

    /// Default tools with the default sandbox profile applied to `bash`.
    pub fn with_default_tools() -> Self {
        Self::with_default_tools_sandboxed(SandboxProfile::default())
    }

    /// Default tools, with `bash` confined to `sandbox`.
    pub fn with_default_tools_sandboxed(sandbox: SandboxProfile) -> Self {
        let mut reg = Self::new();
        reg.register(Arc::new(crate::read_file::ReadFileTool));
        reg.register(Arc::new(crate::write_file::WriteFileTool));
        reg.register(Arc::new(crate::list_dir::ListDirTool));
        reg.register(Arc::new(crate::search::SearchTool));
        reg.register(Arc::new(crate::bash::BashTool::new(sandbox)));
        reg.register(Arc::new(crate::exec_js::ExecJsTool));
        reg.register(Arc::new(crate::view_image::ViewImageTool));
        reg
    }
}
