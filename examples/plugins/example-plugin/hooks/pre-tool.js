#!/usr/bin/env node

/**
 * PreToolUse Hook
 *
 * Executes before any tool is used. Can validate, log, or block tool execution.
 *
 * Exit codes:
 * - 0: Success, continue
 * - 1: Non-blocking error (log but continue)
 * - 2: Blocking error (stop execution)
 *
 * Output JSON for advanced control:
 * {
 *   "continue": true/false,
 *   "permissionDecision": "allow"/"deny"/"ask",
 *   "systemMessage": "Message to user"
 * }
 */

// Read context from stdin or environment
const context = JSON.parse(process.env.HOOK_CONTEXT || '{}');

const toolName = context.tool_name || 'unknown';
const sessionId = context.session_id || 'unknown';

// Log tool usage
console.log(`[PreToolUse] Tool: ${toolName}, Session: ${sessionId}`);

// Example: Block dangerous operations in production
if (process.env.NODE_ENV === 'production' && toolName === 'Write') {
  const output = {
    continue: false,
    permissionDecision: 'deny',
    systemMessage: 'Write operations blocked in production mode'
  };
  console.log(JSON.stringify(output));
  process.exit(2); // Blocking error
}

// Allow by default
const output = {
  continue: true,
  permissionDecision: 'allow'
};
console.log(JSON.stringify(output));
process.exit(0);
