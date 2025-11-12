//! Python bindings for RustyClawd - claude_agent_sdk compatible API
//!
//! Provides drop-in replacement for the claude_agent_sdk Python module.

use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Claude Agent options matching TypeScript SDK
#[pyclass]
#[derive(Clone)]
struct ClaudeAgentOptions {
    #[pyo3(get, set)]
    model: Option<String>,
    #[pyo3(get, set)]
    max_tokens: Option<u32>,
    #[pyo3(get, set)]
    temperature: Option<f32>,
}

#[pymethods]
impl ClaudeAgentOptions {
    #[new]
    fn new() -> Self {
        Self {
            model: None,
            max_tokens: None,
            temperature: None,
        }
    }
}

/// Main query function - matches claude_agent_sdk.query() API
#[pyfunction]
#[pyo3(signature = (prompt, options=None))]
fn query(
    py: Python,
    prompt: String,
    options: Option<ClaudeAgentOptions>,
) -> PyResult<PyObject> {
    // Extract response outside of GIL
    let response = py.allow_threads(|| {
        // Create Tokio runtime
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to create runtime: {}", e)
            ))?;

        // Run async operation
        rt.block_on(async {
            // Load API configuration
            let config = claude_code_core::client::Config::from_default_location()
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    format!("Failed to load config: {}", e)
                ))?;

            // Create client
            let client = claude_code_core::client::Client::new(config);

            // Build request
            let model = options.as_ref()
                .and_then(|o| o.model.clone())
                .unwrap_or_else(|| "claude-sonnet-4-5-20250929".to_string());

            let max_tokens = options.as_ref()
                .and_then(|o| o.max_tokens)
                .unwrap_or(4096);

            let request = claude_code_core::client::CreateMessageRequest::new(
                &model,
                vec![claude_code_core::client::Message::user(&prompt)],
                max_tokens,
            );

            // Send request and get response
            let response = client.create_message(request)
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    format!("API call failed: {}", e)
                ))?;

            Ok::<_, PyErr>(response)
        })
    })?;

    // Convert response to Python dict
    let dict = PyDict::new_bound(py);
    dict.set_item("id", response.id)?;
    dict.set_item("model", response.model)?;
    dict.set_item("role", format!("{:?}", response.role))?;

    // Extract text content from content blocks
    let mut text_content = String::new();
    for block in &response.content {
        if let claude_code_core::client::ContentBlock::Text { text } = block {
            if !text_content.is_empty() {
                text_content.push('\n');
            }
            text_content.push_str(text);
        }
    }
    dict.set_item("content", text_content)?;

    if let Some(stop_reason) = response.stop_reason {
        dict.set_item("stop_reason", stop_reason)?;
    }

    // Add usage info
    let usage_dict = PyDict::new_bound(py);
    usage_dict.set_item("input_tokens", response.usage.input_tokens)?;
    usage_dict.set_item("output_tokens", response.usage.output_tokens)?;
    dict.set_item("usage", usage_dict)?;

    Ok(dict.into())
}

/// Python module initialization
#[pymodule]
fn claude_agent_sdk(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(query, m)?)?;
    m.add_class::<ClaudeAgentOptions>()?;
    Ok(())
}
