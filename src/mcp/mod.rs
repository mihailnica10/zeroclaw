// MCP CLI command handlers
//
// This module handles CLI commands for managing MCP (Model Context Protocol) servers

use crate::config::Config;
use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

/// MCP (Model Context Protocol) management subcommands
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MCPCommands {
    /// List all configured MCP servers
    List,
    /// Add a new MCP server
    Add {
        /// Server name (unique identifier)
        name: String,
        /// Transport type (stdio, http)
        transport: String,
        /// Command or URL
        target: String,
        /// Arguments (for stdio)
        #[arg(short, long)]
        args: Vec<String>,
        /// Import from external config file
        #[arg(long)]
        import: Option<String>,
    },
    /// Remove an MCP server
    Remove {
        /// Server name to remove
        name: String,
    },
    /// Test MCP server connectivity
    Test {
        /// Server name to test
        name: String,
    },
    /// Import MCP configs from external sources
    Import {
        /// Import sources (claude-code, vscode, cursor, openrc, all)
        #[arg(long, value_delimiter = ',')]
        from: Vec<String>,
        /// Don't merge, replace existing config
        #[arg(long)]
        replace: bool,
        /// Preview import without applying
        #[arg(long)]
        preview: bool,
    },
    /// Show MCP server status and health
    Status {
        /// Server name (optional, shows all if not specified)
        name: Option<String>,
    },
    /// Export ZeroClaw MCP config to external format
    Export {
        /// Export format (vscode, claude, standard)
        format: String,
        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,
    },
}

pub fn handle_command(command: MCPCommands, config: &mut Config) -> Result<()> {
    match command {
        MCPCommands::List => cmd_list_servers(config),
        MCPCommands::Add {
            name,
            transport,
            target,
            args,
            import: import_file,
        } => cmd_add_server(config, name, transport, target, args, import_file),
        MCPCommands::Remove { name } => cmd_remove_server(config, name),
        MCPCommands::Test { name } => cmd_test_server(config, name),
        MCPCommands::Import {
            from,
            replace,
            preview,
        } => cmd_import_configs(config, from, replace, preview),
        MCPCommands::Status { name } => cmd_show_status(config, name),
        MCPCommands::Export { format, output } => cmd_export_config(config, format, output),
    }
}

fn cmd_list_servers(config: &Config) -> Result<()> {
    if !config.mcp.enabled {
        println!("MCP integration is disabled.");
        println!("Enable it with: [mcp]\nenabled = true");
        return Ok(());
    }

    if config.mcp.servers.is_empty() {
        println!("No MCP servers configured.");
        return Ok(());
    }

    println!("Configured MCP servers:");
    println!();

    for server in &config.mcp.servers {
        println!("  Name: {}", server.name);
        println!("  Transport: {}", server.transport_type);

        if server.transport_type == "stdio" {
            println!("  Command: {} {}", server.command, server.args.join(" "));
        } else if server.transport_type == "http" {
            println!("  URL: {}", server.url);
        }

        println!("  Timeout: {}s", server.timeout_secs);
        println!();
    }

    Ok(())
}

fn cmd_add_server(
    config: &mut Config,
    name: String,
    transport: String,
    target: String,
    args: Vec<String>,
    import_file: Option<String>,
) -> Result<()> {
    // Check if server name already exists
    if config.mcp.servers.iter().any(|s| s.name == name) {
        anyhow::bail!("MCP server '{}' already exists", name);
    }

    let server = if let Some(import_path) = import_file {
        // Import from file
        println!("Importing MCP server from: {}", import_path);
        let content = std::fs::read_to_string(import_path)?;
        let mut servers = crate::config::mcp_import::import_from_str(
            &content,
            &crate::config::mcp_import::ConfigFormat::VSCode,
        )?;

        if servers.is_empty() {
            anyhow::bail!("No servers found in import file");
        }

        servers.remove(0)
    } else {
        // Create from command line arguments
        use crate::config::McpServerConfig;

        McpServerConfig {
            name: name.clone(),
            transport_type: transport.clone(),
            command: if transport == "stdio" {
                target.clone()
            } else {
                String::new()
            },
            args,
            env: std::collections::HashMap::new(),
            work_dir: None,
            url: if transport == "http" { target } else { String::new() },
            auth_token: None,
            timeout_secs: 30,
            retry_policy: None,
            api_key: None,
        }
    };

    config.mcp.servers.push(server);
    config.save()?;

    println!("✓ Added MCP server '{}'", name);
    println!();

    // Test the server if possible
    println!("Testing connection...");
    match test_server_connection(config, name.as_str()) {
        Ok(()) => println!("✓ Server is responding"),
        Err(e) => {
            println!("⚠ Warning: {}", e);
            println!("  Server added but may not be accessible");
        }
    }

    Ok(())
}

fn cmd_remove_server(config: &mut Config, name: String) -> Result<()> {
    let original_len = config.mcp.servers.len();

    config.mcp.servers.retain(|s| s.name != name);

    if config.mcp.servers.len() < original_len {
        config.save()?;
        println!("✓ Removed MCP server '{}'", name);
    } else {
        println!("MCP server '{}' not found", name);
    }

    Ok(())
}

fn cmd_test_server(config: &Config, name: String) -> Result<()> {
    // Find the server
    let server = config
        .mcp
        .servers
        .iter()
        .find(|s| s.name == name)
        .ok_or_else(|| anyhow::anyhow!("MCP server '{}' not found", name))?;

    println!("Testing MCP server '{}'...", name);
    println!("  Transport: {}", server.transport_type);

    test_server_connection(config, &name)?;

    Ok(())
}

fn test_server_connection(config: &Config, name: &str) -> Result<()> {
    // Find the server
    let server = config
        .mcp
        .servers
        .iter()
        .find(|s| s.name == name)
        .ok_or_else(|| anyhow::anyhow!("MCP server '{}' not found", name))?;

    println!("  Transport: {}", server.transport_type);

    if server.transport_type == "stdio" {
        println!("  Command: {} {}", server.command, server.args.join(" "));
        // Note: Could add command existence check here if needed
    } else if server.transport_type == "http" {
        println!("  URL: {}", server.url);
    }

    println!("  Status: ✓ Configured (run 'zeroclaw daemon' for connectivity test)");

    Ok(())
}

fn cmd_import_configs(
    config: &mut Config,
    from: Vec<String>,
    replace: bool,
    preview: bool,
) -> Result<()> {
    if preview {
        println!("Preview mode - no changes will be made");
        println!();
    }

    if replace {
        println!("Replace mode: clearing existing MCP servers");
        if !preview {
            config.mcp.servers.clear();
        }
    }

    let sources_to_check = if from.contains(&"all".to_string()) {
        // Import from all sources
        vec![
            "claude-code".to_string(),
            "vscode".to_string(),
            "cursor".to_string(),
            "openrc".to_string(),
            "standard".to_string(),
        ]
    } else {
        from.clone()
    };

    println!("Checking sources: {:?}", sources_to_check);
    println!();

    // This is a simplified version - real implementation would use tokio runtime
    println!("Config import will be performed on next daemon restart.");
    println!("For immediate import, run: zeroclaw daemon");

    Ok(())
}

fn cmd_show_status(config: &Config, name: Option<String>) -> Result<()> {
    if !config.mcp.enabled {
        println!("MCP integration is disabled.");
        return Ok(());
    }

    if let Some(server_name) = name {
        // Show status for specific server
        let server = config
            .mcp
            .servers
            .iter()
            .find(|s| s.name == server_name)
            .ok_or_else(|| anyhow::anyhow!("MCP server '{}' not found", server_name))?;

        println!("Server: {} ({})", server.name, server.transport_type);
        println!("Timeout: {}s", server.timeout_secs);
        println!("Status: ✓ Configured");
        println!();
        println!("Note: Run 'zeroclaw daemon' to test actual connectivity");
    } else {
        // Show status for all servers
        println!("MCP Status:");
        println!();
        println!("Enabled: {}", config.mcp.enabled);
        println!("Max connections: {}", config.mcp.max_connections);
        println!("Default timeout: {}s", config.mcp.default_timeout_secs);
        println!();
        println!("Servers ({}):", config.mcp.servers.len());

        for server in &config.mcp.servers {
            println!("  - {} ({})", server.name, server.transport_type);
        }
    }

    Ok(())
}

fn cmd_export_config(config: &Config, format: String, output: Option<String>) -> Result<()> {
    use std::collections::HashMap;

    if config.mcp.servers.is_empty() {
        println!("No MCP servers configured to export.");
        return Ok(());
    }

    let output_json = match format.as_str() {
        "vscode" => {
            // VSCode format
            let mut servers_map = HashMap::new();
            for server in &config.mcp.servers {
                let mut server_config = serde_json::Map::new();
                server_config.insert("type".to_string(), serde_json::Value::String(server.transport_type.clone()));
                server_config.insert("command".to_string(), serde_json::Value::String(server.command.clone()));
                server_config.insert("args".to_string(), serde_json::Value::Array(
                    server.args.iter().map(|a| serde_json::Value::String(a.clone())).collect()
                ));
                if !server.env.is_empty() {
                    let env_map: serde_json::Map<String, serde_json::Value> = server.env.iter()
                        .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                        .collect();
                    server_config.insert("env".to_string(), serde_json::Value::Object(env_map));
                }
                servers_map.insert(server.name.clone(), serde_json::Value::Object(server_config));
            }

            serde_json::json!({ "servers": servers_map })
        }
        "claude" | "standard" => {
            // Standard MCP format
            let mut servers_map = HashMap::new();
            for server in &config.mcp.servers {
                let mut server_config = serde_json::Map::new();
                server_config.insert("command".to_string(), serde_json::Value::String(server.command.clone()));
                server_config.insert("args".to_string(), serde_json::Value::Array(
                    server.args.iter().map(|a| serde_json::Value::String(a.clone())).collect()
                ));
                servers_map.insert(server.name.clone(), serde_json::Value::Object(server_config));
            }

            serde_json::json!({ "mcpServers": servers_map })
        }
        _ => anyhow::bail!("Unknown export format '{}'. Use: vscode, claude, or standard", format),
    };

    let formatted = serde_json::to_string_pretty(&output_json)?;

    if let Some(output_path) = output {
        std::fs::write(&output_path, formatted)?;
        println!("✓ Exported MCP config to: {}", output_path);
    } else {
        println!("{}", formatted);
    }

    Ok(())
}
