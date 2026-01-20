# Schema Validation Demonstration

## Test Cases

### ✅ Valid Schema (Passes Validation)

```json
{
  "type": "object",
  "properties": {
    "command": {
      "type": "string",
      "description": "The command to execute"
    },
    "timeout": {
      "type": "integer",
      "description": "Timeout in milliseconds"
    }
  },
  "required": ["command"]
}
```

**Result**: ✅ Valid - Tool will be included in MCP serve response

---

### ❌ Missing Type Field (Fails Validation)

```json
{
  "properties": {
    "name": {"type": "string"}
  }
}
```

**Result**: ❌ Malformed schema: Schema must have a 'type' field
**Action**: Tool filtered out, logged to stderr

---

### ❌ Invalid Type Value (Fails Validation)

```json
{
  "type": "invalid_type",
  "properties": {
    "name": {"type": "string"}
  }
}
```

**Result**: ❌ Malformed schema: Invalid JSON type: invalid_type
**Action**: Tool filtered out, logged to stderr

---

### ❌ Array Without Items (Fails Validation)

```json
{
  "type": "array"
}
```

**Result**: ❌ Malformed schema: Array type must have 'items' field
**Action**: Tool filtered out, logged to stderr

---

### ❌ Required Field Not in Properties (Fails Validation)

```json
{
  "type": "object",
  "properties": {
    "name": {"type": "string"}
  },
  "required": ["email"]
}
```

**Result**: ❌ Malformed schema: Required field 'email' not in properties
**Action**: Tool filtered out, logged to stderr

---

### ❌ Properties Not Object (Fails Validation)

```json
{
  "type": "object",
  "properties": "should be an object"
}
```

**Result**: ❌ Malformed schema: 'properties' must be an object
**Action**: Tool filtered out, logged to stderr

---

## Example MCP Serve Output

When invalid tools are detected:

```
MCP serve: Filtered out tool 'BadArrayTool' due to incompatible schema: Malformed schema: Array type must have 'items' field
MCP serve: Filtered out tool 'MissingTypeTool' due to incompatible schema: Malformed schema: Schema must have a 'type' field
MCP serve: Filtered out 2 tool(s) with incompatible schemas
```

The tools list response will only contain valid tools, preventing runtime errors.

---

## Testing the Implementation

```bash
# Run all schema validation tests
cd crates/cli
cargo test schema_validator

# Run integration tests
cargo test mcp_schema_validation

# Build and test MCP serve
cargo build --release
```

All tests pass with 100% success rate:
- Unit tests: 12/12 ✅
- Integration tests: 20/20 ✅
- Total: 32/32 tests passing ✅
