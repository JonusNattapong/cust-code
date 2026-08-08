//! Message and content block types for multimodal support.
//!
//! Replaces the simple `&str` prompt with a structured message that can
//! contain text, images, and other content types.

use serde::{Deserialize, Serialize};

/// One piece of content in a message (text, image, etc).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ContentBlock {
    /// Plain text, possibly with markdown.
    Text(String),
    /// Image: media type + base64-encoded bytes.
    Image { media_type: String, data: String },
}

impl ContentBlock {
    pub fn text(s: impl Into<String>) -> Self {
        Self::Text(s.into())
    }

    pub fn image_base64(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Image {
            media_type: media_type.into(),
            data: data.into(),
        }
    }
}

/// A message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentBlock>,
}

/// The role of a message sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: vec![ContentBlock::Text(content.into())],
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: vec![ContentBlock::Text(content.into())],
        }
    }

    /// Create a user message with both text and image.
    pub fn user_with_image(text: impl Into<String>, media_type: impl Into<String>, image_data: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: vec![
                ContentBlock::Text(text.into()),
                ContentBlock::Image {
                    media_type: media_type.into(),
                    data: image_data.into(),
                },
            ],
        }
    }

    pub fn with_content(role: Role, content: Vec<ContentBlock>) -> Self {
        Self { role, content }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_message_shorthand() {
        let m = Message::user("hello");
        assert_eq!(m.role, Role::User);
        assert_eq!(m.content.len(), 1);
    }

    #[test]
    fn user_with_image_has_two_blocks() {
        let m = Message::user_with_image("describe this", "image/png", "abc123");
        assert_eq!(m.content.len(), 2);
    }

    #[test]
    fn serializes_to_json() {
        let m = Message::user("test");
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"type\":\"Text\""));
    }
}
