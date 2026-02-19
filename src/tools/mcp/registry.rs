// McpRegistry - Manages MCP server discovery and tool registration

use crate::config::McpConfig;
use crate::security::{SecretStore, SecurityPolicy};
use crate::tools::mcp::client::{HttpSseMcpClient, McpClient, StdioMcpClient};
use crate::tools::mcp::error::McpError;
use crate::tools::mcp::tool::McpTool;
use crate::tools::traits::Tool;
use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Registry for discovering and managing MCP tools
pub struct McpRegistry;

impl McpRegistry {
    /// Discover and register all MCP tools from configured servers
    pub async fn discover_tools(
        config: &McpConfig,
        security: &Arc<SecurityPolicy>,
        config_path: &Path,
    ) -> Result<Vec<Box<dyn Tool>>> {
        if !config.enabled {
            return Ok(Vec::new());
        }

        let mut all_tools = Vec::new();

        for server_config in &config.servers {
            match Self::register_server(server_config, security.clone(), config_path).await {
                Ok(mut tools) => {
                    tracing::info!(
                        "Discovered {} tools from MCP server '{}'",
                        tools.len(),
                        server_config.name
                    );
                    all_tools.append(&mut tools);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to register MCP server '{}': {}. Skipping.",
                        server_config.name,
                        e
                    );
                }
            }
        }

        Ok(all_tools)
    }

    async fn register_server(
        server_config: &crate::config::McpServerConfig,
        security: Arc<SecurityPolicy>,
        config_path: &Path,
    ) -> Result<Vec<Box<dyn Tool>>, McpError> {
        let retry_policy = server_config.retry_policy.clone().unwrap_or_default();

        // Create appropriate client based on transport type
        let client: Arc<dyn McpClient> = match server_config.transport_type.as_str() {
            "stdio" => {
                let mut stdio_client = StdioMcpClient::new(
                    server_config.name.clone(),
                    server_config.command.clone(),
                    server_config.args.clone(),
                    server_config.env.clone(),
                    server_config.work_dir.clone(),
                    server_config.timeout_secs,
                );

                // Initialize with retry logic
                let mut attempts = 0;
                loop {
                    match stdio_client.initialize().await {
                        Ok(_) => break,
                        Err(e) if attempts < retry_policy.max_attempts => {
                            attempts += 1;
                            tracing::warn!(
                                "MCP server '{}' initialization attempt {}/{} failed: {}. Retrying in {}ms...",
                                server_config.name,
                                attempts,
                                retry_policy.max_attempts,
                                e,
                                retry_policy.backoff_ms
                            );
                            sleep(Duration::from_millis(retry_policy.backoff_ms)).await;
                        }
                        Err(e) => return Err(e),
                    }
                }

                Arc::new(stdio_client)
            }
            "http" => {
                let auth_token = if let Some(token) = &server_config.auth_token {
                    Some(Self::resolve_secret(token, config_path)?)
                } else {
                    None
                };

                let mut http_client = HttpSseMcpClient::new(
                    server_config.name.clone(),
                    server_config.url.clone(),
                    auth_token,
                    server_config.timeout_secs,
                );

                // Initialize with retry logic
                let mut attempts = 0;
                loop {
                    match http_client.initialize().await {
                        Ok(_) => break,
                        Err(e) if attempts < retry_policy.max_attempts => {
                            attempts += 1;
                            tracing::warn!(
                                "MCP server '{}' initialization attempt {}/{} failed: {}. Retrying in {}ms...",
                                server_config.name,
                                attempts,
                                retry_policy.max_attempts,
                                e,
                                retry_policy.backoff_ms
                            );
                            sleep(Duration::from_millis(retry_policy.backoff_ms)).await;
                        }
                        Err(e) => return Err(e),
                    }
                }

                Arc::new(http_client)
            }
            _ => {
                return Err(McpError::unknown_transport(&server_config.transport_type));
            }
        };

        // List tools from server
        let tool_definitions = client.list_tools().await?;

        // Create McpTool wrapper for each definition
        let tools: Vec<Box<dyn Tool>> = tool_definitions
            .into_iter()
            .map(|def| {
                Box::new(McpTool::new(
                    client.clone(),
                    def,
                    security.clone(),
                    server_config.name.clone(),
                )) as Box<dyn Tool>
            })
            .collect();

        Ok(tools)
    }

    fn resolve_secret(secret: &str, config_path: &Path) -> Result<String, McpError> {
        let default_path = std::path::PathBuf::from(".");
        let zeroclaw_dir = config_path.parent().unwrap_or(&default_path);

        let secret_store = SecretStore::new(zeroclaw_dir, true);

        secret_store
            .decrypt(secret)
            .map_err(|e| McpError::initialization_failed("secret", e.to_string()))
    }
}
