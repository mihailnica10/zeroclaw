# MCP CLI Reference

ZeroClaw provides comprehensive CLI commands for managing MCP (Model Context Protocol) servers. This reference covers all available commands, their options, and usage examples.

## Table of Contents

- [Overview](#overview)
- [Commands](#commands)
  - [mcp list](#mcp-list)
  - [mcp add](#mcp-add)
  - [mcp remove](#mcp-remove)
  - [mcp test](#mcp-test)
  - [mcp status](#mcp-status)
  - [mcp import](#mcp-import)
  - [mcp export](#mcp-export)
- [Configuration](#configuration)
- [Examples](#examples)

## Overview

MCP (Model Context Protocol) is a JSON-RPC 2.0-based protocol for extending AI assistant capabilities with external tools and resources. ZeroClaw's MCP integration allows you to:

- Connect to stdio-based MCP servers (local processes)
- Connect to HTTP-based MCP servers (remote services)
- Import configurations from Claude Code, VSCode, Cursor, and standard MCP configs
- Monitor server health and connectivity
- Export ZeroClaw MCP configs to external formats

## Commands

### `mcp list`

List all configured MCP servers.

```bash
zeroclaw mcp list
```

**Output:**
```
Configured MCP servers:

  Name: filesystem
  Transport: stdio
  Command: npx -y @modelcontextprotocol/server-filesystem /path/to/allowed
  Timeout: 30s

  Name: remote-api
  Transport: http
  URL: https://api.example.com/mcp
  Timeout: 30s
```

### `mcp add`

Add a new MCP server configuration.

```bash
zeroclaw mcp add <NAME> <TRANSPORT> <TARGET> [OPTIONS]
```

**Arguments:**
- `NAME` - Unique identifier for the server (required)
- `TRANSPORT` - Transport type: `stdio` or `http` (required)
- `TARGET` - Command path (for stdio) or URL (for http) (required)

**Options:**
- `-a, --args <ARGS>` - Command arguments (for stdio transport)
- `--import <PATH>` - Import server from external config file instead of manual configuration

**Examples:**

Add a stdio server with arguments:
```bash
zeroclaw mcp add filesystem stdio npx -a "-y" -a "@modelcontextprotocol/server-filesystem" -a "/home/user/projects"
```

Add an HTTP server:
```bash
zeroclaw mcp add remote-api http https://api.example.com/mcp
```

Import from external config:
```bash
zeroclaw mcp add imported-server stdio dummy --import ~/.config/Code/User/mcp.json
```

### `mcp remove`

Remove an MCP server configuration.

```bash
zeroclaw mcp remove <NAME>
```

**Arguments:**
- `NAME` - Server name to remove (required)

**Example:**
```bash
zeroclaw mcp remove filesystem
```

### `mcp test`

Test MCP server connectivity.

```bash
zeroclaw mcp test <NAME>
```

**Arguments:**
- `NAME` - Server name to test (required)

**Example:**
```bash
zeroclaw mcp test filesystem
```

**Output:**
```
Testing MCP server 'filesystem'...
  Transport: stdio
  Command: npx -y @modelcontextprotocol/server-filesystem /home/user/projects
  Status: ✓ Configured (run 'zeroclaw daemon' for connectivity test)
```

### `mcp status`

Show MCP server status and health information.

```bash
zeroclaw mcp status [NAME]
```

**Arguments:**
- `NAME` - Server name (optional, shows all servers if not specified)

**Examples:**

Show status for all servers:
```bash
zeroclaw mcp status
```

**Output:**
```
MCP Status:

Enabled: true
Max connections: 10
Default timeout: 30s

Servers (2):
  - filesystem (stdio)
  - remote-api (http)
```

Show status for a specific server:
```bash
zeroclaw mcp status filesystem
```

**Output:**
```
Server: filesystem (stdio)
Timeout: 30s
Status: ✓ Configured

Note: Run 'zeroclaw daemon' to test actual connectivity
```

### `mcp import`

Import MCP server configurations from external sources.

```bash
zeroclaw mcp import --from=<SOURCES> [OPTIONS]
```

**Options:**
- `--from <SOURCES>` - Import sources (comma-separated): `claude-code`, `vscode`, `cursor`, `openrc`, `all`
- `--replace` - Replace existing config instead of merging
- `--preview` - Preview import without applying changes

**Examples:**

Preview import from VSCode:
```bash
zeroclaw mcp import --preview --from=vscode
```

Import from multiple sources:
```bash
zeroclaw mcp import --from=vscode,claude-code,cursor
```

Import from all sources and replace existing config:
```bash
zeroclaw mcp import --from=all --replace
```

**Import Sources:**
- `claude-code` - `~/.config/claude-code/mcp.json`
- `vscode` - `~/.config/Code/User/mcp.json`
- `cursor` - `~/.cursor/mcp.json`
- `openrc` - `~/.config/openrc/mcp.json`
- `standard` - `~/.config/mcp/config.json`
- `all` - All of the above

### `mcp export`

Export ZeroClaw MCP configuration to external format.

```bash
zeroclaw mcp export <FORMAT> [OPTIONS]
```

**Arguments:**
- `FORMAT` - Export format: `vscode`, `claude`, `standard` (required)

**Options:**
- `-o, --output <PATH>` - Output file path (writes to stdout if not specified)

**Examples:**

Export to VSCode format (stdout):
```bash
zeroclaw mcp export vscode
```

Export to Claude Code format and save to file:
```bash
zeroclaw mcp export claude -o ~/.config/claude-code/mcp.json
```

Export to standard MCP format:
```bash
zeroclaw mcp export standard -o mcp-config.json
```

## Configuration

MCP configuration is stored in `~/.zeroclaw/config.toml` under the `[mcp]` section:

```toml
[mcp]
enabled = true
default_timeout_secs = 30
max_connections = 10

[[mcp.servers]]
name = "filesystem"
transport_type = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/projects"]
timeout_secs = 30

[[mcp.servers]]
name = "remote-api"
transport_type = "http"
url = "https://api.example.com/mcp"
timeout_secs = 30
```

### Configuration Fields

**Top-level MCP settings:**
- `enabled` - Enable/disable MCP integration (default: `false`)
- `default_timeout_secs` - Default timeout for server connections (default: `30`)
- `max_connections` - Maximum concurrent server connections (default: `10`)

**Per-server settings:**
- `name` - Unique server identifier (required)
- `transport_type` - Transport type: `stdio` or `http` (required)
- `command` - Command to execute (for stdio transport)
- `args` - Command arguments (array, for stdio transport)
- `env` - Environment variables (object, optional)
- `url` - Server URL (for http transport)
- `timeout_secs` - Connection timeout (default: `30`)

## Examples

### Example 1: Setting up the Filesystem MCP Server

```bash
# Add the filesystem server
zeroclaw mcp add filesystem stdio npx \
  -a "-y" \
  -a "@modelcontextprotocol/server-filesystem" \
  -a "/home/user/projects"

# Verify it was added
zeroclaw mcp list

# Test connectivity
zeroclaw mcp test filesystem
```

### Example 2: Importing Configurations from Claude Code

```bash
# Preview what would be imported
zeroclaw mcp import --preview --from=claude-code

# Actually import
zeroclaw mcp import --from=claude-code

# List imported servers
zeroclaw mcp list
```

### Example 3: Setting up an HTTP-based MCP Server

```bash
# Add remote HTTP server
zeroclaw mcp add remote-api http https://api.example.com/mcp

# Check status
zeroclaw mcp status remote-api
```

### Example 4: Exporting Configuration for VSCode

```bash
# Export to VSCode format
zeroclaw mcp export vscode -o ~/.config/Code/User/mcp.json

# Or export to standard MCP format
zeroclaw mcp export standard -o mcp-export.json
```

## See Also

- [MCP Import Guide](./mcp-import-guide.md) - Detailed guide on importing external MCP configurations
- [Commands Reference](./commands-reference.md) - Complete ZeroClaw CLI reference
- [Configuration Reference](./config-reference.md) - Full configuration documentation
