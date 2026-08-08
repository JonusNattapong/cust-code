use crate::{Message, ProviderClient, TextStream};
use futures_util::StreamExt;
use std::pin::Pin;

pub struct ProviderFailoverGroup {
    pub primary: ProviderClient,
    pub fallbacks: Vec<ProviderClient>,
}

impl ProviderFailoverGroup {
    pub fn new(primary: ProviderClient, fallbacks: Vec<ProviderClient>) -> Self {
        Self { primary, fallbacks }
    }

    pub async fn stream_chat_with_failover(&self, messages: Vec<Message>) -> Result<TextStream, anyhow::Error> {
        // 1. Attempt primary provider
        let primary_stream = self.primary.stream_chat(messages.clone());
        let mut peekable = primary_stream.peekable();

        // Peek first item to check if primary stream returns an immediate error
        if let Some(first_item) = Pin::new(&mut peekable).peek().await {
            if first_item.is_ok() {
                return Ok(Box::pin(peekable));
            }
        } else {
            return Ok(Box::pin(peekable));
        }

        // 2. Failover to secondary fallbacks if primary failed
        for fallback in &self.fallbacks {
            let fb_stream = fallback.stream_chat(messages.clone());
            let mut fb_peekable = fb_stream.peekable();
            if let Some(item) = Pin::new(&mut fb_peekable).peek().await {
                if item.is_ok() {
                    return Ok(Box::pin(fb_peekable));
                }
            } else {
                return Ok(Box::pin(fb_peekable));
            }
        }

        // If all fallbacks failed, return primary stream
        Ok(self.primary.stream_chat(messages))
    }
}
