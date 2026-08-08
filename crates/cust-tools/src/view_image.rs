use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use cust_tools_api::{Availability, PermissionRequest, Tool, ToolError, ToolResult, ToolSpec};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

/// Maximum file size this tool will read, in bytes.
///
/// Provider APIs cap image payloads (Anthropic: 5MB per image before base64
/// inflation); refusing early gives a clear error instead of a rejected
/// request after the read and encode have already happened.
const MAX_IMAGE_BYTES: u64 = 5 * 1024 * 1024;

pub struct ViewImageTool;

#[derive(Debug, Serialize, Deserialize)]
pub struct ViewImageArgs {
    pub path: String,
}

/// Sniff the image media type from magic bytes rather than the file
/// extension — a renamed or extensionless file still identifies correctly,
/// and a wrong extension can't smuggle in an unsupported format.
fn sniff_media_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

impl Tool for ViewImageTool {
    fn name(&self) -> &str {
        "view_image"
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "view_image".to_string(),
            description: "Read an image file and return it as base64-encoded content for the model to view. \
                Supports PNG, JPEG, GIF, and WebP."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the image file"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    fn permission(&self, args: &serde_json::Value) -> PermissionRequest {
        if let Some(path) = args.get("path").and_then(|p| p.as_str()) {
            PermissionRequest::ReadPath(PathBuf::from(path))
        } else {
            PermissionRequest::None
        }
    }

    fn availability(&self) -> Availability {
        Availability::Both
    }

    fn call<'a>(
        &'a self,
        args: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send + 'a>> {
        Box::pin(async move {
            let view_args: ViewImageArgs = serde_json::from_value(args)
                .map_err(|e| ToolError::Failed(anyhow::anyhow!("Invalid arguments: {e}")))?;
            let path = PathBuf::from(&view_args.path);

            let metadata = tokio::fs::metadata(&path).await.map_err(|e| {
                ToolError::Failed(anyhow::anyhow!("Failed to stat {}: {e}", path.display()))
            })?;
            if metadata.len() > MAX_IMAGE_BYTES {
                return Err(ToolError::Failed(anyhow::anyhow!(
                    "{} is {} bytes, exceeding the {}-byte limit for images",
                    path.display(),
                    metadata.len(),
                    MAX_IMAGE_BYTES
                )));
            }

            let bytes = tokio::fs::read(&path).await.map_err(|e| {
                ToolError::Failed(anyhow::anyhow!("Failed to read {}: {e}", path.display()))
            })?;

            let media_type = sniff_media_type(&bytes).ok_or_else(|| {
                ToolError::Failed(anyhow::anyhow!(
                    "{} is not a recognized image format (PNG, JPEG, GIF, WebP)",
                    path.display()
                ))
            })?;

            let data = BASE64.encode(&bytes);
            let summary = format!(
                "Read {} ({}, {} bytes)",
                path.display(),
                media_type,
                bytes.len()
            );
            Ok(ToolResult::success(
                summary,
                Some(serde_json::json!({
                    "media_type": media_type,
                    "data": data,
                })),
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR";
    const JPEG_MAGIC: &[u8] = b"\xff\xd8\xff\xe0\x00\x10JFIF";
    const GIF_MAGIC: &[u8] = b"GIF89a\x01\x00\x01\x00";
    const WEBP_MAGIC: &[u8] = b"RIFF\x00\x00\x00\x00WEBPVP8 ";

    #[test]
    fn sniffs_png_from_magic_bytes() {
        assert_eq!(sniff_media_type(PNG_MAGIC), Some("image/png"));
    }

    #[test]
    fn sniffs_jpeg_from_magic_bytes() {
        assert_eq!(sniff_media_type(JPEG_MAGIC), Some("image/jpeg"));
    }

    #[test]
    fn sniffs_gif_from_magic_bytes() {
        assert_eq!(sniff_media_type(GIF_MAGIC), Some("image/gif"));
    }

    #[test]
    fn sniffs_webp_from_magic_bytes() {
        assert_eq!(sniff_media_type(WEBP_MAGIC), Some("image/webp"));
    }

    #[test]
    fn unrecognized_bytes_sniff_to_none() {
        assert_eq!(sniff_media_type(b"not an image"), None);
        assert_eq!(sniff_media_type(b""), None);
    }

    #[test]
    fn extension_does_not_matter_only_content_does() {
        // A PNG's magic bytes are recognized regardless of what the caller
        // named the file — sniffing must not consult the path at all.
        assert_eq!(sniff_media_type(PNG_MAGIC), Some("image/png"));
    }

    #[tokio::test]
    async fn reads_and_base64_encodes_a_real_png() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.png");
        tokio::fs::write(&path, PNG_MAGIC).await.unwrap();

        let tool = ViewImageTool;
        let result = tool
            .call(serde_json::json!({ "path": path.to_str().unwrap() }))
            .await
            .expect("tool call succeeds");

        assert!(result.ok);
        let data = result.data.expect("data present");
        assert_eq!(data["media_type"], "image/png");
        let decoded = BASE64.decode(data["data"].as_str().unwrap()).unwrap();
        assert_eq!(decoded, PNG_MAGIC);
    }

    #[tokio::test]
    async fn rejects_a_file_that_is_not_an_image() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        tokio::fs::write(&path, b"hello world").await.unwrap();

        let tool = ViewImageTool;
        let result = tool.call(serde_json::json!({ "path": path.to_str().unwrap() })).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_a_missing_file() {
        let tool = ViewImageTool;
        let result = tool
            .call(serde_json::json!({ "path": "/nonexistent/path/x.png" }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_an_oversized_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.png");
        let mut oversized = PNG_MAGIC.to_vec();
        oversized.resize((MAX_IMAGE_BYTES + 1) as usize, 0);
        tokio::fs::write(&path, &oversized).await.unwrap();

        let tool = ViewImageTool;
        let result = tool.call(serde_json::json!({ "path": path.to_str().unwrap() })).await;
        assert!(result.is_err());
    }

    #[test]
    fn permission_requests_read_access_to_the_path() {
        let tool = ViewImageTool;
        let perm = tool.permission(&serde_json::json!({ "path": "photo.png" }));
        assert_eq!(perm, PermissionRequest::ReadPath(PathBuf::from("photo.png")));
    }
}
