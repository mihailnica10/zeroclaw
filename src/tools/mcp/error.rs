// MCP-specific error types

use thiserror::Error;

/// Errors that can occur during MCP client operations
#[derive(Error, Debug)]
pub enum McpError {
    #[error("Failed to spawn MCP server process '{server}': {reason}")]
    ProcessSpawn { server: String, reason: String },

    #[error("MCP server process '{server}' exited unexpectedly: {reason}")]
    ProcessExit { server: String, reason: String },

    #[error("Failed to send request to MCP server '{server}': {reason}")]
    RequestFailed { server: String, reason: String },

    #[error("MCP server '{server}' returned error: {reason}")]
    ServerError { server: String, reason: String },

    #[error("Failed to parse MCP response from '{server}': {reason}")]
    ParseError { server: String, reason: String },

    #[error("MCP server '{server}' timed out after {timeout_secs}s")]
    Timeout { server: String, timeout_secs: u64 },

    #[error("Tool '{tool}' not found on MCP server '{server}'")]
    ToolNotFound { server: String, tool: String },

    #[error("Invalid tool arguments for '{tool}': {reason}")]
    InvalidArguments { tool: String, reason: String },

    #[error("Connection to MCP server '{server}' lost")]
    ConnectionLost { server: String },

    #[error("Unknown transport type '{transport}': use 'stdio' or 'http'")]
    UnknownTransport { transport: String },

    #[error("Failed to initialize MCP server '{server}': {reason}")]
    InitializationFailed { server: String, reason: String },

    #[error("HTTP request to MCP server '{server}' failed: {reason}")]
    HttpError { server: String, reason: String },

    #[error("IO error communicating with MCP server '{server}': {source}")]
    IoError {
        server: String,
        #[source]
        source: std::io::Error,
    },

    #[error("JSON serialization/deserialization error: {reason}")]
    JsonError {
        reason: String,
        #[source]
        source: serde_json::Error,
    },
}

impl McpError {
    /// Create a process spawn error
    pub fn process_spawn(server: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ProcessSpawn {
            server: server.into(),
            reason: reason.into(),
        }
    }

    /// Create a process exit error
    pub fn process_exit(server: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ProcessExit {
            server: server.into(),
            reason: reason.into(),
        }
    }

    /// Create a request failed error
    pub fn request_failed(server: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::RequestFailed {
            server: server.into(),
            reason: reason.into(),
        }
    }

    /// Create a server error
    pub fn server_error(server: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ServerError {
            server: server.into(),
            reason: reason.into(),
        }
    }

    /// Create a parse error
    pub fn parse_error(server: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ParseError {
            server: server.into(),
            reason: reason.into(),
        }
    }

    /// Create a timeout error
    pub fn timeout(server: impl Into<String>, timeout_secs: u64) -> Self {
        Self::Timeout {
            server: server.into(),
            timeout_secs,
        }
    }

    /// Create a tool not found error
    pub fn tool_not_found(server: impl Into<String>, tool: impl Into<String>) -> Self {
        Self::ToolNotFound {
            server: server.into(),
            tool: tool.into(),
        }
    }

    /// Create an invalid arguments error
    pub fn invalid_arguments(tool: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidArguments {
            tool: tool.into(),
            reason: reason.into(),
        }
    }

    /// Create a connection lost error
    pub fn connection_lost(server: impl Into<String>) -> Self {
        Self::ConnectionLost {
            server: server.into(),
        }
    }

    /// Create an unknown transport error
    pub fn unknown_transport(transport: impl Into<String>) -> Self {
        Self::UnknownTransport {
            transport: transport.into(),
        }
    }

    /// Create an initialization failed error
    pub fn initialization_failed(server: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InitializationFailed {
            server: server.into(),
            reason: reason.into(),
        }
    }

    /// Create an HTTP error
    pub fn http_error(server: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::HttpError {
            server: server.into(),
            reason: reason.into(),
        }
    }

    /// Create an IO error
    pub fn io_error(server: impl Into<String>, source: std::io::Error) -> Self {
        Self::IoError {
            server: server.into(),
            source,
        }
    }

    /// Create a JSON error
    pub fn json_error(reason: impl Into<String>, source: serde_json::Error) -> Self {
        Self::JsonError {
            reason: reason.into(),
            source,
        }
    }

    /// Get the server name if this error is server-specific
    pub fn server_name(&self) -> Option<&str> {
        match self {
            Self::ProcessSpawn { server, .. }
            | Self::ProcessExit { server, .. }
            | Self::RequestFailed { server, .. }
            | Self::ServerError { server, .. }
            | Self::ParseError { server, .. }
            | Self::Timeout { server, .. }
            | Self::ToolNotFound { server, .. }
            | Self::ConnectionLost { server, .. }
            | Self::InitializationFailed { server, .. }
            | Self::HttpError { server, .. }
            | Self::IoError { server, .. } => Some(server),
            Self::UnknownTransport { .. }
            | Self::InvalidArguments { .. }
            | Self::JsonError { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = McpError::process_spawn("test-server", "command not found");
        assert!(err.to_string().contains("test-server"));
        assert!(err.to_string().contains("command not found"));
    }

    #[test]
    fn test_server_name() {
        assert_eq!(
            McpError::process_spawn("server1", "reason").server_name(),
            Some("server1")
        );
        assert_eq!(McpError::unknown_transport("unknown").server_name(), None);
    }
}
