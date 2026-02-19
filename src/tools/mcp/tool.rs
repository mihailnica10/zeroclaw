// McpTool - Wrapper that exposes individual MCP tools as native ZeroClaw tools

use crate::security::SecurityPolicy;
use crate::tools::mcp::client::McpClient;
use crate::tools::mcp::error::McpError;
use crate::tools::mcp::protocol::{Content, ToolDefinition};
use crate::tools::traits::{Tool, ToolResult as ZToolResult};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// Wrapper that exposes a single MCP tool as a native ZeroClaw tool
pub struct McpTool {
    /// MCP server client (shared across tools from same server)
    client: Arc<dyn McpClient>,
    /// Tool definition from MCP server
    definition: ToolDefinition,
    /// Security policy reference
    security: Arc<SecurityPolicy>,
    /// Server name (for namespacing)
    server_name: String,
}

impl McpTool {
    pub fn new(
        client: Arc<dyn McpClient>,
        definition: ToolDefinition,
        security: Arc<SecurityPolicy>,
        server_name: String,
    ) -> Self {
        Self {
            client,
            definition,
            security,
            server_name,
        }
    }

    /// Format tool output from MCP content to string
    fn format_content(content: &[Content]) -> String {
        content
            .iter()
            .map(|c| match c {
                Content::Text { text } => text.clone(),
                Content::Image { data, media_type } => {
                    format!("[Image: {} bytes, type={}]", data.len(), media_type)
                }
                Content::Resource { uri } => format!("[Resource: {}]", uri),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[async_trait]
impl Tool for McpTool {
    fn name(&self) -> &str {
        &self.definition.name
    }

    fn description(&self) -> &str {
        &self.definition.description
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.definition.input_schema.clone()
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ZToolResult> {
        // Check rate limits
        if self.security.is_rate_limited() {
            return Ok(ZToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: too many actions in the last hour".into()),
            });
        }

        // Enforce autonomy level
        if let Err(reason) = self.security.enforce_tool_operation(
            crate::security::policy::ToolOperation::Act,
            &format!("mcp.{}.{}", self.server_name, self.definition.name),
        ) {
            return Ok(ZToolResult {
                success: false,
                output: String::new(),
                error: Some(reason),
            });
        }

        // Record action for rate limiting
        if !self.security.record_action() {
            return Ok(ZToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: action budget exhausted".into()),
            });
        }

        // Call MCP server
        match self.client.call_tool(&self.definition.name, args).await {
            Ok(mcp_result) => {
                let output = Self::format_content(&mcp_result.content);
                Ok(ZToolResult {
                    success: !mcp_result.is_error,
                    output,
                    error: if mcp_result.is_error {
                        Some("MCP server returned error flag".into())
                    } else {
                        None
                    },
                })
            }
            Err(McpError::ServerError { reason, .. }) => Ok(ZToolResult {
                success: false,
                output: String::new(),
                error: Some(reason),
            }),
            Err(e) => Ok(ZToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("MCP tool execution failed: {}", e)),
            }),
        }
    }
}
