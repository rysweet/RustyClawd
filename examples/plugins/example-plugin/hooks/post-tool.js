#!/usr/bin/env node

/**
 * PostToolUse Hook
 *
 * Executes after a tool has been used. Can log results, validate output, or trigger actions.
 *
 * Exit codes:
 * - 0: Success
 * - 1: Non-blocking error (log warning)
 *
 * Output JSON:
 * {
 *   "suppressOutput": true/false,
 *   "systemMessage": "Message to user",
 *   "additionalContext": "Context to add to conversation"
 * }
 */

// Read context from stdin or environment
const context = JSON.parse(process.env.HOOK_CONTEXT || '{}');

const toolName = context.tool_name || 'unknown';
const toolResult = context.tool_result || {};
const sessionId = context.session_id || 'unknown';

// Log tool completion
console.log(`[PostToolUse] Tool: ${toolName}, Session: ${sessionId}`);

// Example: Log all file writes
if (toolName === 'Write') {
  const filePath = context.tool_params?.file_path || 'unknown';
  console.log(`[Audit] File written: ${filePath}`);
}

// Example: Add context after certain operations
if (toolName === 'Bash' && toolResult.exit_code !== 0) {
  const output = {
    additionalContext: 'Note: The command failed. Consider checking error output.',
    systemMessage: 'Command execution failed'
  };
  console.log(JSON.stringify(output));
  process.exit(0);
}

// Default: no additional action
const output = {};
console.log(JSON.stringify(output));
process.exit(0);
