//! Python bindings for RustyClawd - claude_agent_sdk compatible API
//!
//! Provides drop-in replacement for the claude_agent_sdk Python module.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::path::PathBuf;

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
fn query(
    py: Python,
    prompt: String,
    options: Option<ClaudeAgentOptions>,
) -> PyResult<Py<PyAny>> {
    // Block on async Rust code
    py.allow_threads(|| {
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

            // Return response as Python string
            Ok(response)
        })
    })?;

    // Convert to Python object
    Python::with_gil(|py| {
        Ok(PyDict::new(py).into())
    })
}

/// Python module initialization
#[pymodule]
fn claude_agent_sdk(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(query, m)?)?;
    m.add_class::<ClaudeAgentOptions>()?;
    Ok(())
}
