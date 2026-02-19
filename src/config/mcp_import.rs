// MCP configuration import from external sources
//
// This module handles importing and merging MCP server configurations from
// external tools like VSCode, Claude Code, Cursor, etc.

use crate::config::{Config, McpConfig, McpServerConfig};
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Import source locations for external MCP configurations
#[derive(Debug, Clone)]
pub struct ImportSource {
    pub name: &'static str,
    pub path: &'static str,
    pub format: ConfigFormat,
}

#[derive(Debug, Clone)]
pub enum ConfigFormat {
    VSCode,
    ClaudeCode,
    Cursor,
    StandardMCP,
}

/// Import sources to check (in priority order)
const IMPORT_SOURCES: &[ImportSource] = &[
    ImportSource {
        name: "Claude Code",
        path: "~/.config/claude-code/mcp.json",
        format: ConfigFormat::ClaudeCode,
    },
    ImportSource {
        name: "VSCode",
        path: "~/.config/Code/User/mcp.json",
        format: ConfigFormat::VSCode,
    },
    ImportSource {
        name: "Cursor",
        path: "~/.cursor/mcp.json",
        format: ConfigFormat::Cursor,
    },
    ImportSource {
        name: "Standard",
        path: "~/.config/mcp/config.json",
        format: ConfigFormat::StandardMCP,
    },
    ImportSource {
        name: "OpenCode",
        path: "~/.config/openrc/mcp.json",
        format: ConfigFormat::StandardMCP,
    },
];

/// Result of importing MCP configurations
#[derive(Debug, Clone, Default)]
pub struct ImportReport {
    pub sources_checked: usize,
    pub sources_found: usize,
    pub servers_imported: usize,
    pub sources: Vec<SourceReport>,
}

#[derive(Debug, Clone)]
pub struct SourceReport {
    pub name: String,
    pub found: bool,
    pub servers_count: usize,
    pub errors: Vec<String>,
}

/// Import MCP configurations from all external sources
pub fn import_external_mcp_configs(config: &mut Config) -> Result<ImportReport> {
    let mut report = ImportReport::default();
    let home_dir = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/home/user"));

    for source in IMPORT_SOURCES {
        let source_path = expand_tilde(source.path, &home_dir);
        let mut source_report = SourceReport {
            name: source.name.to_string(),
            found: false,
            servers_count: 0,
            errors: Vec::new(),
        };

        // Check if file exists
        if !source_path.exists() {
            report.sources.push(source_report);
            continue;
        }

        source_report.found = true;
        report.sources_found += 1;

        // Read and parse config
        let content = std::fs::read_to_string(&source_path)?;
        match import_from_str(&content, &source.format) {
            Ok(servers) => {
                source_report.servers_count = servers.len();
                report.servers_imported += servers.len();

                // Merge with existing config
                for server in servers {
                    // Check for name conflicts
                    if config.mcp.servers.iter().any(|s| s.name == server.name) {
                        let new_name = format!("{}_imported", server.name);
                        tracing::info!(
                            "MCP server '{}' already exists, importing as '{}'",
                            server.name,
                            new_name
                        );
                    }
                    config.mcp.servers.push(server);
                }
            }
            Err(e) => {
                source_report.errors.push(e.to_string());
            }
        }

        report.sources.push(source_report);
    }

    report.sources_checked = IMPORT_SOURCES.len();

    // Save config
    config.save()?;

    Ok(report)
}

/// Import from a specific source file
pub async fn import_from_source(
    path: &Path,
    format: &ConfigFormat,
) -> Result<Vec<McpServerConfig>> {
    let content = tokio::fs::read_to_string(path).await?;
    import_from_str(&content, format)
}

/// Import from a string
pub fn import_from_str(content: &str, format: &ConfigFormat) -> Result<Vec<McpServerConfig>> {
    match format {
        ConfigFormat::VSCode => import_vscode_format(content),
        ConfigFormat::ClaudeCode => import_claude_code_format(content),
        ConfigFormat::Cursor => import_cursor_format(content),
        ConfigFormat::StandardMCP => import_standard_mcp_format(content),
    }
}

/// Import VSCode format MCP configuration
fn import_vscode_format(content: &str) -> Result<Vec<McpServerConfig>> {
    let json: Value = serde_json::from_str(content)?;
    let mut servers = Vec::new();

    // VSCode format: { "servers": { "name": { "type": "stdio", "command": "...", "args": [...] } } }
    if let Some(servers_obj) = json.get("servers").and_then(|v| v.as_object()) {
        for (name, config) in servers_obj {
            let server = parse_vscode_server(name, config)?;
            servers.push(server);
        }
    }

    Ok(servers)
}

/// Parse a single VSCode server configuration
fn parse_vscode_server(name: &str, config: &Value) -> Result<McpServerConfig> {
    let transport_type = config
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("stdio")
        .to_string();

    let command = config
        .get("command")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let args = config
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let env = config
        .get("env")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    // For HTTP transport, get URL from command or config
    let url = if transport_type == "http" {
        config.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string()
    } else {
        String::new()
    };

    Ok(McpServerConfig {
        name: name.to_string(),
        transport_type,
        command,
        args,
        env,
        work_dir: None,
        url,
        auth_token: None,
        timeout_secs: 30,
        retry_policy: None,
        api_key: None,
    })
}

/// Import Claude Code format MCP configuration
fn import_claude_code_format(content: &str) -> Result<Vec<McpServerConfig>> {
    // Claude Code format is similar to VSCode but may have different structure
    import_vscode_format(content)
}

/// Import Cursor format MCP configuration
fn import_cursor_format(content: &str) -> Result<Vec<McpServerConfig>> {
    // Cursor format is similar to VSCode
    import_vscode_format(content)
}

/// Import standard MCP format configuration
fn import_standard_mcp_format(content: &str) -> Result<Vec<McpServerConfig>> {
    let json: Value = serde_json::from_str(content)?;
    let mut servers = Vec::new();

    // Standard format: { "mcpServers": { "name": { ... } } }
    if let Some(servers_obj) = json.get("mcpServers").and_then(|v| v.as_object()) {
        for (name, config) in servers_obj {
            // Convert standard format to ZeroClaw format
            let command = config
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let args = config
                .get("args")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default();

            let env = config
                .get("env")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default();

            servers.push(McpServerConfig {
                name: name.to_string(),
                transport_type: "stdio".to_string(),
                command,
                args,
                env,
                work_dir: None,
                url: String::new(),
                auth_token: None,
                timeout_secs: 30,
                retry_policy: None,
                api_key: None,
            });
        }
    }

    Ok(servers)
}

/// Expand tilde in path to home directory
fn expand_tilde(path: &str, home_dir: &Path) -> PathBuf {
    if path.starts_with("~/") {
        home_dir.join(&path[2..])
    } else if path == "~" {
        home_dir.to_path_buf()
    } else {
        PathBuf::from(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_vscode_format() {
        let config = r#"{
            "servers": {
                "test-server": {
                    "type": "stdio",
                    "command": "npx",
                    "args": ["-y", "@test/server"],
                    "env": {
                        "TEST_VAR": "value"
                    }
                }
            }
        }"#;

        let servers = import_vscode_format(config).unwrap();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "test-server");
        assert_eq!(servers[0].command, "npx");
        assert_eq!(servers[0].args, vec!["-y", "@test/server"]);
        assert_eq!(servers[0].env.get("TEST_VAR"), Some(&"value".to_string()));
    }

    #[test]
    fn test_import_standard_mcp_format() {
        let config = r#"{
            "mcpServers": {
                "test-server": {
                    "command": "npx",
                    "args": ["-y", "@test/server"]
                }
            }
        }"#;

        let servers = import_standard_mcp_format(config).unwrap();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "test-server");
        assert_eq!(servers[0].command, "npx");
    }

    #[test]
    fn test_expand_tilde() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
        let home_path = Path::new(&home);

        // Test ~/path expansion
        let result = expand_tilde("~/test/path", home_path);
        assert_eq!(result, home_path.join("test/path"));

        // Test ~ expansion
        let result = expand_tilde("~", home_path);
        assert_eq!(result, home_path);

        // Test regular path (no expansion)
        let result = expand_tilde("/absolute/path", home_path);
        assert_eq!(result, PathBuf::from("/absolute/path"));
    }
}
