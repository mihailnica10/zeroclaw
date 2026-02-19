# ZeroClaw Config Reference (Operator-Oriented)

This is a high-signal reference for common config sections and defaults.

Last verified: **February 18, 2026**.

Config file path:

- `~/.zeroclaw/config.toml`

## Core Keys

| Key | Default | Notes |
|---|---|---|
| `default_provider` | `openrouter` | provider ID or alias |
| `default_model` | `anthropic/claude-sonnet-4-6` | model routed through selected provider |
| `default_temperature` | `0.7` | model temperature |

## `[agent]`

| Key | Default | Purpose |
|---|---|---|
| `max_tool_iterations` | `10` | Maximum tool-call loop turns per user message across CLI, gateway, and channels |

Notes:

- Setting `max_tool_iterations = 0` falls back to safe default `10`.
- If a channel message exceeds this value, the runtime returns: `Agent exceeded maximum tool iterations (<value>)`.

## `[gateway]`

| Key | Default | Purpose |
|---|---|---|
| `host` | `127.0.0.1` | bind address |
| `port` | `3000` | gateway listen port |
| `require_pairing` | `true` | require pairing before bearer auth |
| `allow_public_bind` | `false` | block accidental public exposure |

## `[memory]`

| Key | Default | Purpose |
|---|---|---|
| `backend` | `sqlite` | `sqlite`, `lucid`, `markdown`, `none` |
| `auto_save` | `true` | automatic persistence |
| `embedding_provider` | `none` | `none`, `openai`, or custom endpoint |
| `vector_weight` | `0.7` | hybrid ranking vector weight |
| `keyword_weight` | `0.3` | hybrid ranking keyword weight |

## `[channels_config]`

Top-level channel options are configured under `channels_config`.

Examples:

- `[channels_config.telegram]`
- `[channels_config.discord]`
- `[channels_config.whatsapp]`
- `[channels_config.email]`

See detailed channel matrix and allowlist behavior in [channels-reference.md](channels-reference.md).

## `[mcp]` (Model Context Protocol)

MCP enables ZeroClaw to dynamically discover and use tools from external MCP servers.

| Key | Default | Purpose |
|---|---|---|
| `enabled` | `false` | Enable MCP integration |
| `default_timeout_secs` | `30` | Timeout for MCP tool execution |
| `max_connections` | `10` | Maximum concurrent MCP server connections |

### MCP Server Configuration

Each MCP server is configured as a separate entry:

```toml
[[mcp.servers]]
name = "filesystem"
transport_type = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/workspace"]
timeout_secs = 30
```

| Server Key | Purpose |
|---|---|
| `name` | Unique identifier (used for tool namespacing) |
| `transport_type` | `"stdio"` (local subprocess) or `"http"` (remote server) |
| `command` | Command to launch stdio MCP server |
| `args` | Arguments for the command |
| `env` | Environment variables (optional) |
| `work_dir` | Working directory for subprocess (optional) |
| `url` | URL for HTTP transport |
| `auth_token` | Bearer token for HTTP transport (optional) |
| `api_key` | API key (encrypted by secret store, optional) |
| `timeout_secs` | Request timeout |
| `retry_policy` | Retry configuration (optional) |

### Example Configurations

**Filesystem MCP server (stdio):**
```toml
[[mcp.servers]]
name = "filesystem"
transport_type = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/workspace"]
```

**GitHub MCP server (stdio):**
```toml
[[mcp.servers]]
name = "github"
transport_type = "stdio"
command = "uvx"
args = ["mcp-server-github"]
api_key = "github_pat_..."  # Will be encrypted
```

**Remote HTTP MCP server:**
```toml
[[mcp.servers]]
name = "remote-service"
transport_type = "http"
url = "https://mcp.example.com/sse"
auth_token = "bearer_token_..."
```

## Security-Relevant Defaults

- deny-by-default channel allowlists (`[]` means deny all)
- pairing required on gateway by default
- public bind disabled by default

## Validation Commands

After editing config:

```bash
zeroclaw status
zeroclaw doctor
zeroclaw channel doctor
```

## Related Docs

- [channels-reference.md](channels-reference.md)
- [providers-reference.md](providers-reference.md)
- [operations-runbook.md](operations-runbook.md)
- [troubleshooting.md](troubleshooting.md)
