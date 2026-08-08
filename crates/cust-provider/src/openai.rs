use crate::message::{ContentBlock, Message};
use cust_config_types::ModelCapabilities;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::pin::Pin;

pub struct OpenAIProvider {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String, model: String, base_url: Option<String>) -> Self {
        let default_url = "https://api.openai.com/v1".to_string();
        Self {
            api_key,
            model,
            base_url: base_url.unwrap_or(default_url),
        }
    }

    pub fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities {
            chat: true,
            vision: true,
            tool_calling: true,
            streaming: true,
            max_context: Some(128000),
        }
    }

    pub fn stream_chat(
        &self,
        messages: Vec<Message>,
    ) -> Pin<Box<dyn futures_util::Stream<Item = Result<String, anyhow::Error>> + Send>> {
        let client = reqwest::Client::new();
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let api_key = self.api_key.clone();
        let model = self.model.clone();

        let stream = async_stream::stream! {
            // Convert messages to OpenAI format
            let messages_value: Vec<Value> = messages
                .into_iter()
                .map(|msg| {
                    let role = match msg.role {
                        crate::message::Role::User => "user",
                        crate::message::Role::Assistant => "assistant",
                    };
                    // OpenAI expects an array of content objects
                    let content: Vec<Value> = msg
                        .content
                        .into_iter()
                        .map(|block| match block {
                            ContentBlock::Text(text) => json!({
                                "type": "text",
                                "text": text
                            }),
                            ContentBlock::Image { media_type, data } => {
                                // OpenAI uses data: URIs for images
                                let data_uri = format!("data:{};base64,{}", media_type, data);
                                json!({
                                    "type": "image_url",
                                    "image_url": {
                                        "url": data_uri,
                                        "detail": "auto"
                                    }
                                })
                            }
                        })
                        .collect();
                    json!({
                        "role": role,
                        "content": content
                    })
                })
                .collect();

            let body = json!({
                "model": model,
                "messages": messages_value,
                "stream": true
            });

            let res = match client
                .post(&url)
                .bearer_auth(&api_key)
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    yield Err(anyhow::anyhow!("HTTP request failed: {e}"));
                    return;
                }
            };

            if !res.status().is_success() {
                let status = res.status();
                let text = res.text().await.unwrap_or_default();
                yield Err(anyhow::anyhow!("API error (status {status}): {text}"));
                return;
            }

            let mut es = res.bytes_stream().eventsource();
            while let Some(event_res) = es.next().await {
                match event_res {
                    Ok(event) => {
                        if event.data == "[DONE]" {
                            break;
                        }
                        if let Ok(v) = serde_json::from_str::<Value>(&event.data) {
                            if let Some(content) = v.pointer("/choices/0/delta/content").and_then(|c| c.as_str()) {
                                yield Ok(content.to_string());
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(anyhow::anyhow!("Stream read error: {e}"));
                        break;
                    }
                }
            }
        };

        Box::pin(stream)
    }
}
