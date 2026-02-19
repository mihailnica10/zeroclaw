// MCP integration tests

use crate::config::{McpConfig, McpServerConfig};
use crate::security::SecurityPolicy;
use crate::tools::mcp::discover_tools;
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_mcp_config_default_disabled() {
    let config = McpConfig::default();
    assert!(!config.enabled);
    assert!(config.servers.is_empty());
}

#[tokio::test]
async fn test_mcp_config_serialization() {
    let config = McpConfig {
        enabled: true,
        default_timeout_secs: 60,
        max_connections: 5,
        servers: vec![McpServerConfig {
            name: "test-server".to_string(),
            transport_type: "stdio".to_string(),
            command: "echo".to_string(),
            args: vec!["test".to_string()],
            env: std::collections::HashMap::new(),
            work_dir: None,
            url: String::new(),
            auth_token: None,
            timeout_secs: 30,
            retry_policy: None,
            api_key: None,
        }],
    };

    let toml_str = toml::to_string(&config).unwrap();
    assert!(toml_str.contains("enabled = true"));
    assert!(toml_str.contains("name = \"test-server\""));

    let parsed: McpConfig = toml::from_str(&toml_str).unwrap();
    assert!(parsed.enabled);
    assert_eq!(parsed.servers.len(), 1);
    assert_eq!(parsed.servers[0].name, "test-server");
}

#[tokio::test]
async fn test_mcp_disabled_returns_empty_tools() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.toml");
    let config = McpConfig {
        enabled: false,
        ..Default::default()
    };
    let security = Arc::new(SecurityPolicy::default());

    let tools = discover_tools(&config, &security, &config_path)
        .await
        .unwrap();

    assert!(tools.is_empty());
}

#[tokio::test]
async fn test_mcp_error_handling() {
    // Test that MCP errors don't crash the system
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.toml");
    let config = McpConfig {
        enabled: true,
        servers: vec![McpServerConfig {
            name: "invalid-server".to_string(),
            transport_type: "stdio".to_string(),
            command: "nonexistent-command-xyz-123".to_string(),
            args: vec![],
            env: std::collections::HashMap::new(),
            work_dir: None,
            url: String::new(),
            auth_token: None,
            timeout_secs: 1,
            retry_policy: None,
            api_key: None,
        }],
        ..Default::default()
    };
    let security = Arc::new(SecurityPolicy::default());

    // This should fail gracefully without panicking
    let tools = discover_tools(&config, &security, &config_path)
        .await
        .unwrap_or_default();

    // Failed servers should be skipped
    assert!(tools.is_empty());
}

#[tokio::test]
async fn test_mcp_unknown_transport() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.toml");
    let config = McpConfig {
        enabled: true,
        servers: vec![McpServerConfig {
            name: "bad-transport".to_string(),
            transport_type: "unknown-transport".to_string(),
            command: String::new(),
            args: vec![],
            env: std::collections::HashMap::new(),
            work_dir: None,
            url: String::new(),
            auth_token: None,
            timeout_secs: 30,
            retry_policy: None,
            api_key: None,
        }],
        ..Default::default()
    };
    let security = Arc::new(SecurityPolicy::default());

    let tools = discover_tools(&config, &security, &config_path)
        .await
        .unwrap_or_default();

    assert!(tools.is_empty());
}
