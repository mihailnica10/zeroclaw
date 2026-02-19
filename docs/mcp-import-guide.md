# MCP Import Guide

This guide explains how to import and merge MCP server configurations from external sources into ZeroClaw.

## Table of Contents

- [Overview](#overview)
- [Supported Sources](#supported-sources)
- [Config Formats](#config-formats)
- [Import Workflow](#import-workflow)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)

## Overview

ZeroClaw can automatically discover and import MCP server configurations from:

- **Claude Code** - `~/.config/claude-code/mcp.json`
- **VSCode** - `~/.config/Code/User/mcp.json`
- **Cursor** - `~/.cursor/mcp.json`
- **OpenCode** - `~/.config/openrc/mcp.json`
- **Standard MCP** - `~/.config/mcp/config.json`

This makes it easy to migrate existing MCP setups to ZeroClaw without manually reconfiguring each server.

## Supported Sources

### Claude Code

**Location:** `~/.config/claude-code/mcp.json`

Claude Code uses a standard MCP configuration format with server definitions.

**Example:**
```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/files"]
    }
  }
}
```

### VSCode

**Location:** `~/.config/Code/User/mcp.json`

VSCode uses an extended format with explicit `type` field and additional metadata.

**Example:**
```json
{
  "servers": {
    "filesystem": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/files"],
      "env": {
        "NODE_ENV": "production"
      }
    }
  }
}
```

### Cursor

**Location:** `~/.cursor/mcp.json`

Cursor uses the VSCode-compatible format.

**Example:**
```json
{
  "servers": {
    "brave-search": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-brave-search"]
    }
  }
}
```

### OpenCode

**Location:** `~/.config/openrc/mcp.json`

Uses the standard MCP format.

**Example:**
```json
{
  "mcpServers": {
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"]
    }
  }
}
```

### Standard MCP

**Location:** `~/.config/mcp/config.json`

Standard MCP configuration format.

**Example:**
```json
{
  "mcpServers": {
    "postgres": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-postgres"]
    }
  }
}
```

## Config Formats

### VSCode Format

Used by VSCode and Cursor. Key fields:

- `servers` - Object containing server definitions
- `type` - Transport type (`stdio` or `http`)
- `command` - Command to execute (for stdio)
- `args` - Command arguments array
- `env` - Environment variables (optional)
- `url` - Server URL (for http transport)

### Standard MCP Format

Used by Claude Code, OpenCode, and standard MCP config. Key fields:

- `mcpServers` - Object containing server definitions
- `command` - Command to execute
- `args` - Command arguments array
- `env` - Environment variables (optional)

### Format Conversion

ZeroClaw automatically converts between formats:

| VSCode Field | Standard MCP Field | ZeroClaw Field |
|--------------|-------------------|----------------|
| `type` | (assumed `stdio`) | `transport_type` |
| `command` | `command` | `command` |
| `args` | `args` | `args` |
| `env` | `env` | `env` |
| `url` | (n/a) | `url` |

## Import Workflow

### Step 1: Preview Import

Always preview before importing to see what will be added:

```bash
zeroclaw mcp import --preview --from=vscode
```

**Output:**
```
Preview mode - no changes will be made

Checking sources: ["vscode"]

VSCode:
  Found: ✓
  Servers: 3

Would import:
  - filesystem (stdio)
  - brave-search (stdio)
  - github (stdio)
```

### Step 2: Import from Specific Sources

Import from one or more specific sources:

```bash
# Single source
zeroclaw mcp import --from=vscode

# Multiple sources
zeroclaw mcp import --from=vscode,claude-code

# All sources
zeroclaw mcp import --from=all
```

### Step 3: Verify Import

Check that servers were imported correctly:

```bash
zeroclaw mcp list
```

### Step 4: Test Connectivity

Test each imported server:

```bash
zeroclaw mcp test filesystem
```

## Examples

### Example 1: Import from VSCode

```bash
# Preview import
zeroclaw mcp import --preview --from=vscode

# Import
zeroclaw mcp import --from=vscode

# Verify
zeroclaw mcp list
```

### Example 2: Import from All Sources

```bash
# Preview all sources
zeroclaw mcp import --preview --from=all

# Import from all sources
zeroclaw mcp import --from=all

# Check status
zeroclaw mcp status
```

### Example 3: Replace Existing Config

If you want to replace your existing MCP configuration instead of merging:

```bash
# WARNING: This will clear existing MCP servers
zeroclaw mcp import --from=claude-code --replace
```

### Example 4: Selective Import

Import from specific sources only:

```bash
zeroclaw mcp import --from=vscode,cursor
```

## Name Conflicts

When importing, if a server name already exists, ZeroClaw automatically appends `_imported` to avoid conflicts:

```bash
# Original server: "filesystem"
# Imported server: "filesystem_imported"

zeroclaw mcp list
```

**Output:**
```
Configured MCP servers:

  Name: filesystem
  Transport: stdio
  Command: npx -y @modelcontextprotocol/server-filesystem /path1

  Name: filesystem_imported
  Transport: stdio
  Command: npx -y @modelcontextprotocol/server-filesystem /path2
```

## Troubleshooting

### Import Shows "No Servers Found"

**Problem:** Import command completes but no servers are imported.

**Solutions:**
1. Check that the source file exists:
   ```bash
   ls -la ~/.config/Code/User/mcp.json
   ```

2. Verify the file is valid JSON:
   ```bash
   cat ~/.config/Code/User/mcp.json | jq .
   ```

3. Check the file has expected structure:
   ```bash
   cat ~/.config/Code/User/mcp.json | jq 'keys'
   ```

### "Command Not Found" Errors

**Problem:** Server was imported but `mcp test` shows command not found.

**Solutions:**
1. Verify the command is available:
   ```bash
   which npx
   ```

2. Check command path is absolute or in PATH:
   ```bash
   echo $PATH
   ```

3. For npm packages, ensure npm/npx is installed:
   ```bash
   npm --version
   npx --version
   ```

### Permission Errors

**Problem:** Import fails with permission denied.

**Solutions:**
1. Check file permissions:
   ```bash
   ls -la ~/.config/Code/User/mcp.json
   ```

2. Ensure config directory is readable:
   ```bash
   chmod +r ~/.config/Code/User/mcp.json
   ```

### Merge Conflicts

**Problem:** Want to keep manual changes but also import.

**Solutions:**
1. Export current config first:
   ```bash
   zeroclaw mcp export standard -o mcp-backup.json
   ```

2. Import with preview to see conflicts:
   ```bash
   zeroclaw mcp import --preview --from=vscode
   ```

3. Manually edit `~/.zeroclaw/config.toml` if needed:
   ```toml
   [[mcp.servers]]
   name = "my-custom-server"
   transport_type = "stdio"
   command = "custom-command"
   ```

### Servers Not Starting in Daemon

**Problem:** Servers imported but not available when running `zeroclaw daemon`.

**Solutions:**
1. Verify MCP is enabled:
   ```bash
   zeroclaw mcp status
   ```

2. Check the daemon is running:
   ```bash
   zeroclaw service status
   ```

3. Review daemon logs for errors:
   ```bash
   journalctl -u zeroclaw -n 50
   ```

## Exporting and Backups

Before making major changes, export your current configuration:

```bash
# Export to standard MCP format
zeroclaw mcp export standard -o mcp-backup-$(date +%Y%m%d).json

# Export to VSCode format
zeroclaw mcp export vscode -o vscode-backup.json
```

## See Also

- [MCP CLI Reference](./mcp-cli-reference.md) - Complete MCP command reference
- [Configuration Reference](./config-reference.md) - Full configuration documentation
- [Troubleshooting](./troubleshooting.md) - General troubleshooting guide
