#!/usr/bin/env bash
# Manual test script for `rusty mcp serve` command
#
# This script sends JSON-RPC requests to the MCP server and validates responses.
# Run with: ./tests/mcp_serve_manual_test.sh

set -e

echo "🧪 Testing MCP Serve Command"
echo "=============================="

# Build the binary first
echo "📦 Building rusty..."
cargo build --release 2>&1 | grep -E "(Compiling|Finished)" || true

RUSTY_BIN="./target/release/rusty"

if [ ! -f "$RUSTY_BIN" ]; then
    echo "❌ Binary not found at $RUSTY_BIN"
    exit 1
fi

echo "✅ Binary found"
echo ""

# Test 1: Initialize request
echo "Test 1: Initialize Request"
echo "--------------------------"

INIT_REQUEST='{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "1.0",
    "capabilities": {},
    "clientInfo": {
      "name": "test-client",
      "version": "1.0.0"
    }
  }
}'

echo "📤 Sending initialize request..."
RESPONSE=$(echo "$INIT_REQUEST" | timeout 5 "$RUSTY_BIN" mcp serve 2>&1 | head -1)

if echo "$RESPONSE" | jq -e '.result.serverInfo.name == "rustyclawd"' > /dev/null 2>&1; then
    echo "✅ Initialize response valid"
    echo "   Server: $(echo "$RESPONSE" | jq -r '.result.serverInfo.name')"
    echo "   Version: $(echo "$RESPONSE" | jq -r '.result.serverInfo.version')"
else
    echo "❌ Initialize response invalid"
    echo "   Response: $RESPONSE"
    exit 1
fi

echo ""

# Test 2: tools/list request
echo "Test 2: Tools List Request"
echo "--------------------------"

LIST_REQUEST='{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list",
  "params": {}
}'

echo "📤 Sending tools/list request..."

# Send both initialize and tools/list
COMBINED_INPUT=$(cat <<EOF
$INIT_REQUEST
$LIST_REQUEST
EOF
)

RESPONSE=$(echo "$COMBINED_INPUT" | timeout 5 "$RUSTY_BIN" mcp serve 2>&1 | tail -1)

TOOL_COUNT=$(echo "$RESPONSE" | jq -r '.result.tools | length' 2>/dev/null || echo "0")

if [ "$TOOL_COUNT" -gt 0 ]; then
    echo "✅ Tools list response valid"
    echo "   Found $TOOL_COUNT tools"
    echo "   Sample tools:"
    echo "$RESPONSE" | jq -r '.result.tools[0:3] | .[] | "   - \(.name): \(.description)"' 2>/dev/null || true
else
    echo "❌ Tools list response invalid"
    echo "   Response: $RESPONSE"
    exit 1
fi

echo ""

# Test 3: Verify schema compliance (type: object at root)
echo "Test 3: Schema Compliance Check"
echo "-------------------------------"

echo "📤 Checking all tool schemas have 'type: object'..."

SCHEMA_CHECK=$(echo "$COMBINED_INPUT" | timeout 5 "$RUSTY_BIN" mcp serve 2>&1 | tail -1)

INVALID_SCHEMAS=$(echo "$SCHEMA_CHECK" | jq -r '.result.tools[] | select(.inputSchema.type != "object") | .name' 2>/dev/null || echo "")

if [ -z "$INVALID_SCHEMAS" ]; then
    echo "✅ All tool schemas have 'type: object' at root"
else
    echo "❌ Some tool schemas missing 'type: object':"
    echo "$INVALID_SCHEMAS"
    exit 1
fi

echo ""

# Test 4: tools/call request (simple command)
echo "Test 4: Tools Call Request"
echo "-------------------------"

CALL_REQUEST='{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "Bash",
    "arguments": {
      "command": "echo '\''test output'\''"
    }
  }
}'

FULL_INPUT=$(cat <<EOF
$INIT_REQUEST
$CALL_REQUEST
EOF
)

echo "📤 Sending tools/call request (Bash tool)..."

RESPONSE=$(echo "$FULL_INPUT" | timeout 10 "$RUSTY_BIN" mcp serve 2>&1 | tail -1)

if echo "$RESPONSE" | jq -e '.result.content[0].text' > /dev/null 2>&1; then
    OUTPUT=$(echo "$RESPONSE" | jq -r '.result.content[0].text')
    echo "✅ Tools call response valid"
    echo "   Output: $OUTPUT"
else
    echo "❌ Tools call response invalid"
    echo "   Response: $RESPONSE"
    exit 1
fi

echo ""

# Summary
echo "==============================="
echo "✅ All MCP Serve tests passed!"
echo "==============================="
echo ""
echo "The 'rusty mcp serve' command:"
echo "  ✅ Responds to initialize requests"
echo "  ✅ Lists all available tools"
echo "  ✅ Has valid JSON schemas (type: object)"
echo "  ✅ Executes tools successfully"
echo ""
echo "Ready for integration with MCP clients!"
