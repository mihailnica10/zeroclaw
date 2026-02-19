// MCP client trait and transport implementations

use crate::tools::mcp::error::McpError;
use crate::tools::mcp::protocol::{
    CallToolParams, InitializeParams, JsonRpcId, JsonRpcRequest, JsonRpcResponse, ListToolsResult,
    ServerCapabilities, ToolDefinition, ToolResult,
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Generic MCP client interface supporting both stdio and HTTP/SSE transports
#[async_trait]
pub trait McpClient: Send + Sync {
    /// Initialize MCP session (send initialize request, receive server capabilities)
    async fn initialize(&mut self) -> Result<ServerCapabilities, McpError>;

    /// List available tools from MCP server
    async fn list_tools(&self) -> Result<Vec<ToolDefinition>, McpError>;

    /// Call a specific tool on the MCP server
    async fn call_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolResult, McpError>;

    /// Health check: verify server is responsive
    async fn health_check(&self) -> Result<bool, McpError>;

    /// Graceful shutdown
    async fn shutdown(&self) -> Result<(), McpError>;

    /// Get server name (for logging/tool prefixing)
    fn server_name(&self) -> &str;
}

/// Stdio-based MCP client for local subprocess MCP servers
pub struct StdioMcpClient {
    server_name: String,
    command: String,
    args: Vec<String>,
    env: std::collections::HashMap<String, String>,
    work_dir: Option<String>,
    timeout_secs: u64,

    // Process and I/O handles
    #[allow(clippy::type_complexity)]
    child: Arc<Mutex<Option<tokio::process::Child>>>,
    stdin: Arc<Mutex<Option<tokio::process::ChildStdin>>>,
    stdout: Arc<Mutex<Option<tokio::io::BufReader<tokio::process::ChildStdout>>>>,
    request_id: Arc<Mutex<u64>>,

    // Cached capabilities
    capabilities: Arc<Mutex<Option<ServerCapabilities>>>,
}

impl StdioMcpClient {
    pub fn new(
        server_name: String,
        command: String,
        args: Vec<String>,
        env: std::collections::HashMap<String, String>,
        work_dir: Option<String>,
        timeout_secs: u64,
    ) -> Self {
        Self {
            server_name,
            command,
            args,
            env,
            work_dir,
            timeout_secs,
            child: Arc::new(Mutex::new(None)),
            stdin: Arc::new(Mutex::new(None)),
            stdout: Arc::new(Mutex::new(None)),
            request_id: Arc::new(Mutex::new(0)),
            capabilities: Arc::new(Mutex::new(None)),
        }
    }

    async fn ensure_process_running(&self) -> Result<(), McpError> {
        let mut child_guard = self.child.lock().await;
        if child_guard.is_some() {
            return Ok(());
        }

        let mut cmd = tokio::process::Command::new(&self.command);
        cmd.args(&self.args);
        for (key, value) in &self.env {
            cmd.env(key, value);
        }
        if let Some(wd) = &self.work_dir {
            cmd.current_dir(wd);
        }

        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| McpError::process_spawn(&self.server_name, e.to_string()))?;

        let stdin = child.stdin.take().ok_or_else(|| {
            McpError::process_exit(&self.server_name, "Failed to open stdin".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            McpError::process_exit(&self.server_name, "Failed to open stdout".to_string())
        })?;

        *child_guard = Some(child);
        *self.stdin.lock().await = Some(stdin);
        *self.stdout.lock().await = Some(tokio::io::BufReader::new(stdout));

        Ok(())
    }

    async fn send_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        self.ensure_process_running().await?;

        let id = {
            let mut req_id = self.request_id.lock().await;
            *req_id += 1;
            JsonRpcId::Number(*req_id as i64)
        };

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: id.clone(),
            method: method.to_string(),
            params: Some(params),
        };

        let request_str = serde_json::to_string(&request)
            .map_err(|e| McpError::json_error("Failed to serialize request", e))?;

        let mut stdin = self.stdin.lock().await;
        let stdin_ref = stdin
            .as_mut()
            .ok_or_else(|| McpError::connection_lost(&self.server_name))?;

        stdin_ref
            .write_all(request_str.as_bytes())
            .await
            .map_err(|e| McpError::io_error(&self.server_name, e))?;
        stdin_ref
            .write_all(b"\n")
            .await
            .map_err(|e| McpError::io_error(&self.server_name, e))?;
        stdin_ref
            .flush()
            .await
            .map_err(|e| McpError::io_error(&self.server_name, e))?;
        drop(stdin);

        let response_str = tokio::time::timeout(
            std::time::Duration::from_secs(self.timeout_secs),
            self.read_line(),
        )
        .await
        .map_err(|_| McpError::timeout(&self.server_name, self.timeout_secs))??;

        let response: JsonRpcResponse = serde_json::from_str(&response_str)
            .map_err(|e| McpError::parse_error(&self.server_name, e.to_string()))?;

        if let Some(err) = response.error {
            return Err(McpError::server_error(&self.server_name, err.message));
        }

        response.result.ok_or_else(|| {
            McpError::parse_error(
                &self.server_name,
                "Response missing result field".to_string(),
            )
        })
    }

    async fn read_line(&self) -> Result<String, McpError> {
        let mut stdout = self.stdout.lock().await;
        let stdout_ref = stdout
            .as_mut()
            .ok_or_else(|| McpError::connection_lost(&self.server_name))?;
        let mut line = String::new();
        stdout_ref
            .read_line(&mut line)
            .await
            .map_err(|e| McpError::io_error(&self.server_name, e))?;
        Ok(line)
    }
}

#[async_trait]
impl McpClient for StdioMcpClient {
    async fn initialize(&mut self) -> Result<ServerCapabilities, McpError> {
        self.ensure_process_running().await?;

        let params = serde_json::to_value(InitializeParams {
            protocolVersion: "2024-11-05".to_string(),
            capabilities: crate::tools::mcp::protocol::ClientCapabilities {
                roots: None,
                sampling: None,
            },
            clientInfo: crate::tools::mcp::protocol::ClientInfo {
                name: "zeroclaw".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        })
        .map_err(|e| McpError::json_error("Failed to serialize init params", e))?;

        let result = self.send_request("initialize", params).await?;
        let capabilities: ServerCapabilities = serde_json::from_value(result)
            .map_err(|e| McpError::parse_error(&self.server_name, e.to_string()))?;

        // Send initialized notification
        let _ = self
            .send_request("notifications/initialized", serde_json::json!(null))
            .await;

        *self.capabilities.lock().await = Some(capabilities.clone());
        Ok(capabilities)
    }

    async fn list_tools(&self) -> Result<Vec<ToolDefinition>, McpError> {
        let result = self
            .send_request("tools/list", serde_json::json!({}))
            .await?;
        let list_result: ListToolsResult = serde_json::from_value(result)
            .map_err(|e| McpError::parse_error(&self.server_name, e.to_string()))?;
        Ok(list_result.tools)
    }

    async fn call_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolResult, McpError> {
        let params = serde_json::to_value(CallToolParams {
            name: tool_name.to_string(),
            arguments,
        })
        .map_err(|e| McpError::json_error("Failed to serialize tool params", e))?;

        let result = self.send_request("tools/call", params).await?;
        let tool_result: ToolResult = serde_json::from_value(result)
            .map_err(|e| McpError::parse_error(&self.server_name, e.to_string()))?;
        Ok(tool_result)
    }

    async fn health_check(&self) -> Result<bool, McpError> {
        match self.send_request("ping", serde_json::json!({})).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn shutdown(&self) -> Result<(), McpError> {
        let mut child_guard = self.child.lock().await;
        if let Some(mut child) = child_guard.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        *self.stdin.lock().await = None;
        *self.stdout.lock().await = None;
        Ok(())
    }

    fn server_name(&self) -> &str {
        &self.server_name
    }
}

/// HTTP-based MCP client for remote MCP servers
pub struct HttpSseMcpClient {
    server_name: String,
    url: String,
    auth_token: Option<String>,
    timeout_secs: u64,
    http_client: reqwest::Client,
    capabilities: Arc<Mutex<Option<ServerCapabilities>>>,
}

impl HttpSseMcpClient {
    pub fn new(
        server_name: String,
        url: String,
        auth_token: Option<String>,
        timeout_secs: u64,
    ) -> Self {
        Self {
            server_name,
            url,
            auth_token,
            timeout_secs,
            http_client: reqwest::Client::new(),
            capabilities: Arc::new(Mutex::new(None)),
        }
    }

    async fn send_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: JsonRpcId::String(Uuid::new_v4().to_string()),
            method: method.to_string(),
            params: Some(params),
        };

        let mut req_builder = self
            .http_client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(self.timeout_secs));

        if let Some(token) = &self.auth_token {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", token));
        }

        let response = req_builder
            .json(&request)
            .send()
            .await
            .map_err(|e| McpError::http_error(&self.server_name, e.to_string()))?;

        if !response.status().is_success() {
            return Err(McpError::http_error(
                &self.server_name,
                format!("HTTP {}", response.status()),
            ));
        }

        let response_json: JsonRpcResponse = response
            .json()
            .await
            .map_err(|e| McpError::parse_error(&self.server_name, e.to_string()))?;

        if let Some(err) = response_json.error {
            return Err(McpError::server_error(&self.server_name, err.message));
        }

        response_json.result.ok_or_else(|| {
            McpError::parse_error(
                &self.server_name,
                "Response missing result field".to_string(),
            )
        })
    }
}

#[async_trait]
impl McpClient for HttpSseMcpClient {
    async fn initialize(&mut self) -> Result<ServerCapabilities, McpError> {
        let params = serde_json::to_value(InitializeParams {
            protocolVersion: "2024-11-05".to_string(),
            capabilities: crate::tools::mcp::protocol::ClientCapabilities {
                roots: None,
                sampling: None,
            },
            clientInfo: crate::tools::mcp::protocol::ClientInfo {
                name: "zeroclaw".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        })
        .map_err(|e| McpError::json_error("Failed to serialize init params", e))?;

        let result = self.send_request("initialize", params).await?;
        let capabilities: ServerCapabilities = serde_json::from_value(result)
            .map_err(|e| McpError::parse_error(&self.server_name, e.to_string()))?;

        *self.capabilities.lock().await = Some(capabilities.clone());
        Ok(capabilities)
    }

    async fn list_tools(&self) -> Result<Vec<ToolDefinition>, McpError> {
        let result = self
            .send_request("tools/list", serde_json::json!({}))
            .await?;
        let list_result: ListToolsResult = serde_json::from_value(result)
            .map_err(|e| McpError::parse_error(&self.server_name, e.to_string()))?;
        Ok(list_result.tools)
    }

    async fn call_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolResult, McpError> {
        let params = serde_json::to_value(CallToolParams {
            name: tool_name.to_string(),
            arguments,
        })
        .map_err(|e| McpError::json_error("Failed to serialize tool params", e))?;

        let result = self.send_request("tools/call", params).await?;
        let tool_result: ToolResult = serde_json::from_value(result)
            .map_err(|e| McpError::parse_error(&self.server_name, e.to_string()))?;
        Ok(tool_result)
    }

    async fn health_check(&self) -> Result<bool, McpError> {
        match self.send_request("ping", serde_json::json!({})).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    async fn shutdown(&self) -> Result<(), McpError> {
        // No-op for HTTP (stateless)
        Ok(())
    }

    fn server_name(&self) -> &str {
        &self.server_name
    }
}
