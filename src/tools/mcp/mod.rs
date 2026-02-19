// MCP (Model Context Protocol) tool integration for ZeroClaw
//
// This module enables ZeroClaw to dynamically discover and use tools from
// external MCP-compliant servers via stdio or HTTP/SSE transports.

pub mod client;
pub mod error;
pub mod protocol;
pub mod registry;
pub mod tool;

pub use client::McpClient;
pub use error::McpError;
pub use registry::McpRegistry;
pub use tool::McpTool;

use crate::config::McpConfig;
use crate::security::SecurityPolicy;
use anyhow::Result;
use std::sync::Arc;

/// Discover and register all MCP tools from configured servers
pub async fn discover_tools(
    config: &McpConfig,
    security: &Arc<SecurityPolicy>,
    config_path: &std::path::Path,
) -> Result<Vec<Box<dyn super::Tool>>> {
    if !config.enabled {
        return Ok(Vec::new());
    }

    McpRegistry::discover_tools(config, security, config_path).await
}

#[cfg(test)]
mod tests;
