#!/bin/bash
# Simple MCP test server
# Implements the Model Context Protocol over stdio (line-delimited JSON-RPC)
# Use for testing ZeroClaw's MCP client integration

set -e

# Counter for request IDs
REQ_ID=1

# Response functions
respond_initialize() {
    cat <<EOF
{"jsonrpc":"2.0","id":${REQ_ID},"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"test-mcp-server","version":"0.1.0"}}}
EOF
    ((REQ_ID++))
}

respond_tools_list() {
    cat <<'EOF'
{"jsonrpc":"2.0","id":2,"result":{"tools":[
  {"name":"echo","description":"Echo back the input text","inputSchema":{"type":"object","properties":{"text":{"type":"string","description":"Text to echo back"}},"required":["text"]}}},
  {"name":"add","description":"Add two numbers together","inputSchema":{"type":"object","properties":{"a":{"type":"number","description":"First number"},"b":{"type":"number","description":"Second number"}},"required":["a","b"]}}},
  {"name":"get_time","description":"Get current Unix timestamp","inputSchema":{"type":"object","properties":{}}},
  {"name":"random","description":"Generate a random number","inputSchema":{"type":"object","properties":{"max":{"type":"number","description":"Maximum value (default: 100)"}}}},
  {"name":"reverse","description":"Reverse a string","inputSchema":{"type":"object","properties":{"text":{"type":"string","description":"Text to reverse"}},"required":["text"]}}}
]}}
EOF
    ((REQ_ID++))
}

respond_tool_call() {
    local request="$1"
    local tool_name=$(echo "$request" | jq -r '.params.name // empty' 2>/dev/null || echo "")

    case "$tool_name" in
        echo)
            local text=$(echo "$request" | jq -r '.params.arguments.text // "empty"' 2>/dev/null)
            # Escape special characters for JSON
            text=$(echo "$text" | sed 's/\\/\\\\/g' | sed 's/"/\\"/g')
            cat <<EOF
{"jsonrpc":"2.0","id":${REQ_ID},"result":{"content":[{\"type\":\"text\",\"text\":\"${text}\"}]}}
EOF
            ;;
        add)
            local a=$(echo "$request" | jq -r '.params.arguments.a // 0' 2>/dev/null)
            local b=$(echo "$request" | jq -r '.params.arguments.b // 0' 2>/dev/null)
            local sum=$((a + b))
            cat <<EOF
{"jsonrpc":"2.0","id":${REQ_ID},"result":{"content":[{\"type\":\"text\",\"text\":\"${sum}\"}]}}
EOF
            ;;
        get_time)
            local time=$(date +%s)
            cat <<EOF
{"jsonrpc":"2.0","id":${REQ_ID},"result":{"content":[{\"type\":\"text\",\"text\":\"${time}\"}]}}
EOF
            ;;
        random)
            local max=$(echo "$request" | jq -r '.params.arguments.max // 100' 2>/dev/null)
            local rand=$((RANDOM % max))
            cat <<EOF
{"jsonrpc":"2.0","id":${REQ_ID},"result":{"content":[{\"type\":\"text\",\"text\":\"${rand}\"}]}}
EOF
            ;;
        reverse)
            local text=$(echo "$request" | jq -r '.params.arguments.text // ""' 2>/dev/null)
            local reversed=$(echo "$text" | rev)
            # Escape for JSON
            reversed=$(echo "$reversed" | sed 's/\\/\\\\/g' | sed 's/"/\\"/g')
            cat <<EOF
{"jsonrpc":"2.0","id":${REQ_ID},"result":{"content":[{\"type\":\"text\",\"text\":\"${reversed}\"}]}}
EOF
            ;;
        *)
            # Unknown tool
            cat <<EOF
{"jsonrpc":"2.0","id":${REQ_ID},"error":{"code":-32601,"message":"Tool not found: ${tool_name}"}}
EOF
            ;;
    esac
    ((REQ_ID++))
}

# Main request loop
main() {
    # Check if jq is available
    if ! command -v jq &> /dev/null; then
        echo "Error: jq is required but not installed. Please install jq." >&2
        exit 1
    fi

    echo "MCP Test Server starting..." >&2
    echo "Listening for JSON-RPC requests on stdin..." >&2

    # Read line by line from stdin
    while IFS= read -r line; do
        # Skip empty lines
        [[ -z "$line" ]] && continue

        # Extract method
        local method=$(echo "$line" | jq -r '.method // empty' 2>/dev/null)

        case "$method" in
            initialize)
                respond_initialize
                ;;
            tools/list)
                respond_tools_list
                ;;
            tools/call)
                respond_tool_call "$line"
                ;;
            ping)
                # Respond to ping
                cat <<EOF
{"jsonrpc":"2.0","id":${REQ_ID},"result":{}}
EOF
                ((REQ_ID++))
                ;;
            notifications/initialized)
                # Acknowledge initialized notification
                >&2 echo "Client initialized"
                ;;
            *)
                # Unknown method - return error
                cat <<EOF
{"jsonrpc":"2.0","id":${REQ_ID},"error":{"code":-32601,"message":"Method not found: ${method}"}}
EOF
                ((REQ_ID++))
                ;;
        esac
    done
}

# Run main function
main
