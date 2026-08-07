use crate::bridge::{HostBridge, HostReply, HostRequest};
use serde_json::Value;
use std::sync::Arc;

pub struct CodeEvaluator {
    bridge: Arc<HostBridge>,
}

impl CodeEvaluator {
    pub fn new(bridge: HostBridge) -> Self {
        Self {
            bridge: Arc::new(bridge),
        }
    }

    pub async fn eval_script(&self, script: &str) -> Result<String, anyhow::Error> {
        let bridge = self.bridge.clone();
        let script = script.to_string();

        let handle = tokio::runtime::Handle::current();

        tokio::task::spawn_blocking(move || {
            let runtime = rquickjs::Runtime::new()?;
            let context = rquickjs::Context::full(&runtime)?;

            context.with(|ctx| {
                let global = ctx.globals();
                let tools_obj = rquickjs::Object::new(ctx.clone())?;

                for tool_name in bridge.available_tools() {
                    let b = bridge.clone();
                    let t_name = tool_name.clone();
                    let h = handle.clone();

                    let raw_fn_name = format!("__raw_tool_{t_name}");

                    let func = rquickjs::Function::new(
                        ctx.clone(),
                        move |_ctx: rquickjs::Ctx<'_>,
                              args_str: String|
                              -> rquickjs::Result<String> {
                            let json_args: Value = serde_json::from_str(&args_str)
                                .unwrap_or_else(|_| serde_json::json!({}));

                            let req = HostRequest::CallTool {
                                name: t_name.clone(),
                                args: json_args,
                            };

                            let reply = h.block_on(b.handle_request(req));
                            match reply {
                                HostReply::ToolResult { ok, summary, data } => {
                                    Ok(serde_json::json!({
                                        "ok": ok,
                                        "summary": summary,
                                        "data": data
                                    })
                                    .to_string())
                                }
                                HostReply::Error(_err) => Err(rquickjs::Error::Exception),
                            }
                        },
                    )?;

                    global.set(&raw_fn_name, func)?;

                    let wrapper_code = format!(
                        "(args) => {raw_fn_name}(typeof args === 'string' ? args : JSON.stringify(args || {{}}))"
                    );
                    let wrapper_fn: rquickjs::Function = ctx.eval(wrapper_code)?;
                    tools_obj.set(tool_name, wrapper_fn)?;
                }

                global.set("tools", tools_obj)?;

                let res: rquickjs::Value = ctx.eval(script)?;
                let output_str = if let Some(s) = res.as_string() {
                    s.to_string()?
                } else {
                    format!("{res:?}")
                };
                Ok::<String, anyhow::Error>(output_str)
            })
        })
        .await?
    }
}
