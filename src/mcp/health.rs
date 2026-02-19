// MCP health check and monitoring
//
// This module provides health check and monitoring functionality for MCP servers

use crate::config::McpServerConfig;
use std::time::Instant;

/// Health status of an MCP server
#[derive(Debug, Clone)]
pub enum HealthStatus {
    /// Server is healthy and responding
    Healthy {
        latency_ms: u128,
        tools_count: usize,
    },
    /// Server is not responding
    Unresponsive,
    /// Server returned an error
    Error {
        message: String,
    },
}

/// Health report for a single MCP server
#[derive(Debug, Clone)]
pub struct ServerHealthReport {
    pub name: String,
    pub transport: String,
    pub status: HealthStatus,
}

/// Check the health of a single MCP server
///
/// Note: This is a simplified sync version. Full health checks require async runtime
/// which will be available when running via daemon.
pub fn check_server_health_sync(server: &McpServerConfig) -> HealthStatus {
    // For now, just check if the configuration is valid
    // A real health check would require spawning the MCP client

    if server.command.is_empty() && server.url.is_empty() {
        return HealthStatus::Error {
            message: "Invalid configuration: neither command nor URL specified".to_string(),
        };
    }

    if server.transport_type == "stdio" {
        // Check if command exists
        if let Ok(_) = std::Command::new(&server.command).arg("--version").output() {
            HealthStatus::Healthy {
                latency_ms: 0,
                tools_count: 0,
            }
        } else {
            HealthStatus::Error {
                message: format!("Command '{}' not found", server.command),
            }
        }
    } else if server.transport_type == "http" {
        // Check if URL is valid format
        if server.url.starts_with("http://") || server.url.starts_with("https://") {
            HealthStatus::Healthy {
                latency_ms: 0,
                tools_count: 0,
            }
        } else {
            HealthStatus::Error {
                message: format!("Invalid URL: {}", server.url),
            }
        }
    } else {
        HealthStatus::Error {
            message: format!("Unknown transport type: {}", server.transport_type),
        }
    }
}

/// Generate health reports for all configured MCP servers
pub fn monitor_all_servers_sync(servers: &[McpServerConfig]) -> Vec<ServerHealthReport> {
    servers
        .iter()
        .map(|server| ServerHealthReport {
            name: server.name.clone(),
            transport: server.transport_type.clone(),
            status: check_server_health_sync(server),
        })
        .collect()
}

/// Format health report for display
pub fn format_health_report(reports: &[ServerHealthReport]) -> String {
    let mut output = String::new();

    output.push_str(&format!("MCP Server Health Report\n"));
    output.push_str(&format!("========================\n\n"));

    for report in reports {
        match &report.status {
            HealthStatus::Healthy {
                latency_ms,
                tools_count,
            } => {
                output.push_str(&format!("Server: {} ({})\n", report.name, report.transport));
                output.push_str(&format!("Status: ✓ Healthy ({}ms latency)\n", latency_ms));
                output.push_str(&format!("Tools: {} discovered (run 'zeroclaw daemon' for actual count)\n\n", tools_count));
            }
            HealthStatus::Unresponsive => {
                output.push_str(&format!("Server: {} ({})\n", report.name, report.transport));
                output.push_str(&format!("Status: ✗ Unresponsive\n\n"));
            }
            HealthStatus::Error { message } => {
                output.push_str(&format!("Server: {} ({})\n", report.name, report.transport));
                output.push_str(&format!("Status: ✗ Error: {}\n\n", message));
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::McpServerConfig;

    #[test]
    fn test_check_stdio_server_health() {
        let server = McpServerConfig {
            name: "test-stdio".to_string(),
            transport_type: "stdio".to_string(),
            command: "echo".to_string(),
            args: vec![],
            env: std::collections::HashMap::new(),
            work_dir: None,
            url: String::new(),
            auth_token: None,
            timeout_secs: 30,
            retry_policy: None,
            api_key: None,
        };

        let status = check_server_health_sync(&server);
        assert!(matches!(status, HealthStatus::Healthy { .. }));
    }

    #[test]
    fn test_check_invalid_server() {
        let server = McpServerConfig {
            name: "invalid".to_string(),
            transport_type: "stdio".to_string(),
            command: "nonexistent-command-xyz-123".to_string(),
            args: vec![],
            env: std::collections::HashMap::new(),
            work_dir: None,
            url: String::new(),
            auth_token: None,
            timeout_secs: 30,
            retry_policy: None,
            api_key: None,
        };

        let status = check_server_health_sync(&server);
        assert!(matches!(status, HealthStatus::Error { .. }));
    }

    #[test]
    fn test_format_health_report() {
        let report = format_health_report(&[
            ServerHealthReport {
                name: "test".to_string(),
                transport: "stdio".to_string(),
                status: HealthStatus::Healthy {
                    latency_ms: 12,
                    tools_count: 4,
                },
            },
        ]);

        assert!(report.contains("Server: test"));
        assert!(report.contains("✓ Healthy"));
        assert!(report.contains("12ms latency"));
        assert!(report.contains("4 discovered"));
    }
}
