//! Bidirectional transport for the Claude Agent SDK hook callback protocol.
//!
//! When the SDK configures hooks during the initialize handshake, the CLI must
//! send `control_request` messages (with subtype `hook_callback`) to stdout and
//! read `control_response` messages from stdin. This module encapsulates that
//! protocol so the rest of the codebase only sees `send_hook_callback`.
//!
//! Also contains [`SdkHookConfig`] which stores the hook matchers and callback
//! IDs received during the initialize handshake.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use anyhow::{Context, Result};
use serde_json::Value;

// ---------------------------------------------------------------------------
// SDK hook configuration
// ---------------------------------------------------------------------------

/// Hook configuration received from the SDK during the initialize handshake.
///
/// The SDK sends hook matchers and callback IDs so the CLI can call back when
/// hook events fire. When a tool event matches a configured hook, the CLI
/// sends a `hook_callback` control_request via [`SdkTransport`] and reads the
/// SDK's response to decide whether to allow or deny the action.
#[derive(Debug, Clone, Default)]
pub struct SdkHookConfig {
    /// Map of event name (e.g. "PreToolUse") to a list of (matcher_pattern, callback_ids).
    pub events: HashMap<String, Vec<(String, Vec<String>)>>,
}

impl SdkHookConfig {
    /// Parse the `hooks` object from the SDK initialize request.
    ///
    /// Expected shape:
    /// ```json
    /// {
    ///   "PreToolUse": [{"matcher": "Bash", "hookCallbackIds": ["hook_0"]}],
    ///   "PostToolUse": [{"matcher": "*", "hookCallbackIds": ["hook_1"]}]
    /// }
    /// ```
    pub fn from_json(hooks_value: &Value) -> Self {
        let mut events: HashMap<String, Vec<(String, Vec<String>)>> = HashMap::new();

        if let Some(obj) = hooks_value.as_object() {
            for (event_name, entries) in obj {
                let mut matchers = Vec::new();
                if let Some(arr) = entries.as_array() {
                    for entry in arr {
                        let matcher = entry
                            .get("matcher")
                            .and_then(|m| m.as_str())
                            .unwrap_or("*")
                            .to_string();
                        let callback_ids: Vec<String> = entry
                            .get("hookCallbackIds")
                            .and_then(|ids| ids.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default();
                        if !callback_ids.is_empty() {
                            matchers.push((matcher, callback_ids));
                        }
                    }
                }
                if !matchers.is_empty() {
                    events.insert(event_name.clone(), matchers);
                }
            }
        }

        Self { events }
    }

    /// Check if any hooks were configured.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Return all callback IDs that match a given event and tool name.
    ///
    /// A matcher pattern of `"*"` matches any tool name; otherwise the pattern
    /// must exactly equal `tool_name`. Returns a vec of `(callback_id, matcher_pattern)`.
    pub fn get_matching_callbacks(&self, event: &str, tool_name: &str) -> Vec<(String, String)> {
        let mut matches = Vec::new();
        if let Some(matchers) = self.events.get(event) {
            for (pattern, callback_ids) in matchers {
                if pattern == "*" || pattern == tool_name {
                    for cb_id in callback_ids {
                        matches.push((cb_id.clone(), pattern.clone()));
                    }
                }
            }
        }
        matches
    }
}

// ---------------------------------------------------------------------------
// SDK transport
// ---------------------------------------------------------------------------

/// Bidirectional transport for Claude Agent SDK protocol.
///
/// Sends `control_request` messages via stdout and reads `control_response`
/// messages from stdin. Thread-safe: both reader and writer are behind mutexes.
pub struct SdkTransport {
    reader: Mutex<Box<dyn BufRead + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    next_request_id: AtomicU64,
}

impl SdkTransport {
    /// Create transport from stdin/stdout.
    ///
    /// **Important**: Call this only after the handshake has completed and the
    /// stdin lock from `read_stream_json_stdin` has been dropped. The transport
    /// takes a fresh stdin lock for reading responses.
    pub fn from_stdio() -> Self {
        Self {
            reader: Mutex::new(Box::new(std::io::BufReader::new(std::io::stdin()))),
            writer: Mutex::new(Box::new(std::io::stdout())),
            next_request_id: AtomicU64::new(1),
        }
    }

    /// Create from explicit reader/writer (for testing).
    pub fn from_rw(reader: Box<dyn BufRead + Send>, writer: Box<dyn Write + Send>) -> Self {
        Self {
            reader: Mutex::new(reader),
            writer: Mutex::new(writer),
            next_request_id: AtomicU64::new(1),
        }
    }

    /// Send a hook callback to the SDK and wait for the response.
    ///
    /// Builds a `control_request` with subtype `hook_callback`, writes it as a
    /// single JSON line to the writer, then reads one JSON line from the reader
    /// and extracts `response.output`.
    ///
    /// NOTE: The `read_line` call blocks until the SDK responds. If the SDK
    /// crashes or disconnects, the read returns an error (broken pipe).
    /// A proper timeout mechanism (via spawn_blocking + tokio::time::timeout)
    /// is a future improvement. The SDK itself uses 60s timeouts for callbacks.
    pub fn send_hook_callback(
        &self,
        callback_id: &str,
        event: &str,
        input: &Value,
    ) -> Result<Value> {
        let req_id = format!(
            "hook_req_{}",
            self.next_request_id.fetch_add(1, Ordering::SeqCst)
        );

        let request = serde_json::json!({
            "type": "control_request",
            "request_id": req_id,
            "request": {
                "subtype": "hook_callback",
                "callback_id": callback_id,
                "event": event,
                "input": input
            }
        });

        // Send request as a single newline-delimited JSON line.
        {
            let mut writer = self
                .writer
                .lock()
                .map_err(|e| anyhow::anyhow!("Writer lock poisoned: {}", e))?;
            let json = serde_json::to_string(&request)?;
            writeln!(writer, "{}", json)?;
            writer.flush()?;
        }

        // Read response (one JSON line).
        {
            let mut reader = self
                .reader
                .lock()
                .map_err(|e| anyhow::anyhow!("Reader lock poisoned: {}", e))?;
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .context("Failed to read SDK hook response")?;

            if line.trim().is_empty() {
                return Err(anyhow::anyhow!(
                    "Empty response from SDK for hook callback {}",
                    callback_id
                ));
            }

            let response: Value = serde_json::from_str(line.trim())
                .context("Failed to parse SDK hook response JSON")?;

            // Extract output from response.response.output, falling back to empty object.
            Ok(response
                .get("response")
                .and_then(|r| r.get("output"))
                .cloned()
                .unwrap_or(serde_json::json!({})))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Build a mock SDK response for a hook callback.
    fn mock_response(output: Value) -> String {
        let resp = serde_json::json!({
            "type": "control_response",
            "response": {
                "request_id": "hook_req_1",
                "output": output
            }
        });
        format!("{}\n", serde_json::to_string(&resp).unwrap())
    }

    #[test]
    fn test_send_hook_callback_roundtrip() {
        let response_json = mock_response(serde_json::json!({"decision": "allow"}));
        let reader: Box<dyn BufRead + Send> = Box::new(Cursor::new(response_json.into_bytes()));
        let writer_buf: Vec<u8> = Vec::new();
        let writer: Box<dyn Write + Send> = Box::new(Cursor::new(writer_buf));

        let transport = SdkTransport::from_rw(reader, writer);

        let input = serde_json::json!({"tool_name": "Bash", "tool_input": {}});
        let result = transport
            .send_hook_callback("hook_0", "PreToolUse", &input)
            .unwrap();

        assert_eq!(result["decision"], "allow");
    }

    #[test]
    fn test_send_hook_callback_deny() {
        let response_json =
            mock_response(serde_json::json!({"decision": "deny", "reason": "blocked"}));
        let reader: Box<dyn BufRead + Send> = Box::new(Cursor::new(response_json.into_bytes()));
        let writer: Box<dyn Write + Send> = Box::new(Cursor::new(Vec::new()));

        let transport = SdkTransport::from_rw(reader, writer);

        let input = serde_json::json!({"tool_name": "Write"});
        let result = transport
            .send_hook_callback("hook_1", "PreToolUse", &input)
            .unwrap();

        assert_eq!(result["decision"], "deny");
        assert_eq!(result["reason"], "blocked");
    }

    #[test]
    fn test_send_hook_callback_empty_output() {
        // Response without output field returns empty object.
        let resp = serde_json::json!({
            "type": "control_response",
            "response": {
                "request_id": "hook_req_1"
            }
        });
        let response_json = format!("{}\n", serde_json::to_string(&resp).unwrap());
        let reader: Box<dyn BufRead + Send> = Box::new(Cursor::new(response_json.into_bytes()));
        let writer: Box<dyn Write + Send> = Box::new(Cursor::new(Vec::new()));

        let transport = SdkTransport::from_rw(reader, writer);
        let result = transport
            .send_hook_callback("hook_2", "PostToolUse", &serde_json::json!({}))
            .unwrap();

        assert!(result.is_object());
        assert_eq!(result, serde_json::json!({}));
    }

    #[test]
    fn test_send_hook_callback_verifies_request_format() {
        let response_json = mock_response(serde_json::json!({}));
        let reader: Box<dyn BufRead + Send> = Box::new(Cursor::new(response_json.into_bytes()));
        let writer_buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));

        // Use a shared buffer so we can inspect what was written.
        struct SharedWriter(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
        impl Write for SharedWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let writer: Box<dyn Write + Send> = Box::new(SharedWriter(writer_buf.clone()));
        let transport = SdkTransport::from_rw(reader, writer);

        let input = serde_json::json!({"tool_name": "Read", "tool_input": {"file_path": "/tmp/x"}});
        transport
            .send_hook_callback("cb_42", "PreToolUse", &input)
            .unwrap();

        let written = writer_buf.lock().unwrap();
        let sent: Value = serde_json::from_slice(&written).unwrap();
        assert_eq!(sent["type"], "control_request");
        assert_eq!(sent["request"]["subtype"], "hook_callback");
        assert_eq!(sent["request"]["callback_id"], "cb_42");
        assert_eq!(sent["request"]["event"], "PreToolUse");
        assert_eq!(sent["request"]["input"]["tool_name"], "Read");
    }

    #[test]
    fn test_send_hook_callback_empty_stdin_errors() {
        let reader: Box<dyn BufRead + Send> = Box::new(Cursor::new(Vec::new()));
        let writer: Box<dyn Write + Send> = Box::new(Cursor::new(Vec::new()));

        let transport = SdkTransport::from_rw(reader, writer);
        let result = transport.send_hook_callback("hook_0", "PreToolUse", &serde_json::json!({}));

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Empty response"),
            "Expected empty response error, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_send_hook_callback_invalid_json_errors() {
        let reader: Box<dyn BufRead + Send> = Box::new(Cursor::new(b"not valid json\n".to_vec()));
        let writer: Box<dyn Write + Send> = Box::new(Cursor::new(Vec::new()));

        let transport = SdkTransport::from_rw(reader, writer);
        let result = transport.send_hook_callback("hook_0", "PreToolUse", &serde_json::json!({}));

        assert!(result.is_err());
    }

    #[test]
    fn test_request_ids_increment() {
        // Two responses for two calls.
        let resp1 = mock_response(serde_json::json!({}));
        let resp2 = mock_response(serde_json::json!({}));
        let both = format!("{}{}", resp1, resp2);

        let reader: Box<dyn BufRead + Send> = Box::new(Cursor::new(both.into_bytes()));

        struct CapturingWriter(std::sync::Arc<std::sync::Mutex<Vec<String>>>);
        impl Write for CapturingWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                if let Ok(s) = std::str::from_utf8(buf) {
                    let trimmed = s.trim();
                    if !trimmed.is_empty() {
                        self.0.lock().unwrap().push(trimmed.to_string());
                    }
                }
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let lines = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let writer: Box<dyn Write + Send> = Box::new(CapturingWriter(lines.clone()));
        let transport = SdkTransport::from_rw(reader, writer);

        transport
            .send_hook_callback("a", "PreToolUse", &serde_json::json!({}))
            .unwrap();
        transport
            .send_hook_callback("b", "PostToolUse", &serde_json::json!({}))
            .unwrap();

        let captured = lines.lock().unwrap();
        assert_eq!(captured.len(), 2);
        let req1: Value = serde_json::from_str(&captured[0]).unwrap();
        let req2: Value = serde_json::from_str(&captured[1]).unwrap();
        assert_eq!(req1["request_id"], "hook_req_1");
        assert_eq!(req2["request_id"], "hook_req_2");
    }
}
