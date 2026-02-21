use super::*;
use crate::Tool;
use futures::StreamExt;

#[test]
fn test_params_validation_valid() {
    let params = WebSearchParams {
        query: "Rust programming".to_string(),
        allowed_domains: vec![],
        blocked_domains: vec![],
    };
    assert!(params.validate().is_ok());
    assert!(params.validate_domain_exclusivity().is_ok());
}

#[test]
fn test_params_validation_query_too_short() {
    let params = WebSearchParams {
        query: "R".to_string(),
        allowed_domains: vec![],
        blocked_domains: vec![],
    };
    assert!(params.validate().is_err());
}

#[test]
fn test_params_validation_domain_exclusivity() {
    let params = WebSearchParams {
        query: "Rust programming".to_string(),
        allowed_domains: vec!["rust-lang.org".to_string()],
        blocked_domains: vec!["example.com".to_string()],
    };
    assert!(params.validate_domain_exclusivity().is_err());
}

#[test]
fn test_params_validation_allowed_only() {
    let params = WebSearchParams {
        query: "Rust programming".to_string(),
        allowed_domains: vec!["rust-lang.org".to_string()],
        blocked_domains: vec![],
    };
    assert!(params.validate().is_ok());
    assert!(params.validate_domain_exclusivity().is_ok());
}

#[test]
fn test_params_validation_blocked_only() {
    let params = WebSearchParams {
        query: "Rust programming".to_string(),
        allowed_domains: vec![],
        blocked_domains: vec!["example.com".to_string()],
    };
    assert!(params.validate().is_ok());
    assert!(params.validate_domain_exclusivity().is_ok());
}

#[test]
fn test_model_support_detection() {
    assert!(WebSearchTool::is_model_supported("claude-opus-4-6"));
    assert!(WebSearchTool::is_model_supported("claude-sonnet-4-6"));
    assert!(WebSearchTool::is_model_supported(
        "claude-haiku-4-5-20251001"
    ));
    assert!(!WebSearchTool::is_model_supported("claude-2.1"));
    assert!(!WebSearchTool::is_model_supported("gpt-4"));
}

#[test]
fn test_cache_key_building() {
    let params1 = WebSearchParams {
        query: "Rust".to_string(),
        allowed_domains: vec![],
        blocked_domains: vec![],
    };
    let key1 = WebSearchTool::build_cache_key(&params1);
    assert_eq!(key1, "Rust");

    let params2 = WebSearchParams {
        query: "Rust".to_string(),
        allowed_domains: vec!["rust-lang.org".to_string()],
        blocked_domains: vec![],
    };
    let key2 = WebSearchTool::build_cache_key(&params2);
    assert!(key2.contains("allow:"));
    assert!(key2.contains("rust-lang.org"));

    let params3 = WebSearchParams {
        query: "Rust".to_string(),
        allowed_domains: vec![],
        blocked_domains: vec!["example.com".to_string()],
    };
    let key3 = WebSearchTool::build_cache_key(&params3);
    assert!(key3.contains("block:"));
    assert!(key3.contains("example.com"));
}

#[tokio::test]
async fn test_cache_functionality() {
    let cache = WebSearchCache::new();
    let key = "test_query".to_string();
    let results = CachedSearchResults {
        results: vec![SearchResultBlock {
            tool_use_id: "tool_123".to_string(),
            content: vec![SearchHit {
                title: "Test Result".to_string(),
                url: "https://example.com".to_string(),
                snippet: Some("A test snippet".to_string()),
            }],
        }],
        count: 1,
    };

    // Insert and retrieve
    cache.insert(key.clone(), results.clone()).await;
    let cached = cache.get(&key).await;
    assert!(cached.is_some());
    let cached = cached.unwrap();
    assert_eq!(cached.count, 1);
    assert_eq!(cached.results.len(), 1);
    assert_eq!(cached.results[0].content.len(), 1);
}

#[test]
fn test_search_hit_serialization() {
    let hit = SearchHit {
        title: "Test Result".to_string(),
        url: "https://example.com".to_string(),
        snippet: Some("A test snippet".to_string()),
    };

    let json = serde_json::to_string(&hit).unwrap();
    assert!(json.contains("Test Result"));
    assert!(json.contains("https://example.com"));
    assert!(json.contains("A test snippet"));
}

#[test]
fn test_search_result_block_serialization() {
    let block = SearchResultBlock {
        tool_use_id: "tool_123".to_string(),
        content: vec![
            SearchHit {
                title: "Result 1".to_string(),
                url: "https://example.com/1".to_string(),
                snippet: None,
            },
            SearchHit {
                title: "Result 2".to_string(),
                url: "https://example.com/2".to_string(),
                snippet: Some("Result 2 snippet".to_string()),
            },
        ],
    };

    let json = serde_json::to_string(&block).unwrap();
    assert!(json.contains("tool_123"));
    assert!(json.contains("Result 1"));
    assert!(json.contains("Result 2"));
    assert!(json.contains("Result 2 snippet"));
}

#[test]
fn test_web_search_output_serialization() {
    let output = WebSearchOutput {
        query: "test query".to_string(),
        results: vec![SearchResultBlock {
            tool_use_id: "tool_123".to_string(),
            content: vec![SearchHit {
                title: "Test".to_string(),
                url: "https://example.com".to_string(),
                snippet: None,
            }],
        }],
        count: 1,
        duration_seconds: 1.23,
        cached: false,
    };

    let json = serde_json::to_string(&output).unwrap();
    assert!(json.contains("test query"));
    assert!(json.contains("1.23"));
    assert!(json.contains("false")); // cached: false
}

#[tokio::test]
async fn test_metadata() {
    let tool = WebSearchTool::new();
    let metadata = tool.metadata();
    assert_eq!(metadata.name, "WebSearch");
    assert!(metadata.description.contains("server-side"));
    assert!(metadata.description.contains("caching"));
}

#[tokio::test]
async fn test_tool_is_read_only() {
    let tool = WebSearchTool::new();
    assert!(tool.is_read_only());
}

#[tokio::test]
async fn test_tool_is_concurrency_safe() {
    let tool = WebSearchTool::new();
    assert!(tool.is_concurrency_safe());
}

#[tokio::test]
async fn test_query_too_short_error() {
    let tool = WebSearchTool::new();
    let params = WebSearchParams {
        query: "R".to_string(),
        allowed_domains: vec![],
        blocked_domains: vec![],
    };
    let ctx = ToolContext::default();

    let result = tool.execute(params, &ctx).await;
    assert!(result.is_err());
    if let Err(e) = result {
        let err_msg = e.to_string();
        assert!(err_msg.contains("at least 2 characters") || err_msg.contains("validation"));
    }
}

#[tokio::test]
async fn test_domain_exclusivity_error() {
    let tool = WebSearchTool::new();
    let params = WebSearchParams {
        query: "Test query".to_string(),
        allowed_domains: vec!["example.com".to_string()],
        blocked_domains: vec!["blocked.com".to_string()],
    };
    let ctx = ToolContext::default();

    let result = tool.execute(params, &ctx).await;
    assert!(result.is_err());
    if let Err(e) = result {
        let err_msg = e.to_string();
        assert!(err_msg.contains("mutually exclusive") || err_msg.contains("validation"));
    }
}

#[tokio::test]
#[ignore] // Requires API key and network
async fn test_web_search_phase2_integration() {
    // This test makes real API calls and requires:
    // 1. Valid API key in ~/.claude-msec-k
    // 2. Network connectivity
    // 3. Anthropic API access
    // Run with: cargo test test_web_search_phase2_integration -- --ignored --nocapture

    let tool = WebSearchTool::new();
    let params = WebSearchParams {
        query: "Rust programming language".to_string(),
        allowed_domains: vec![],
        blocked_domains: vec![],
    };
    let ctx = ToolContext {
        debug: true,
        ..Default::default()
    };

    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    // Should have progress events and a result
    assert!(events.len() > 1);

    // Find the result event
    let result = events.iter().find_map(|e| match e {
        ToolEvent::Result(output) => Some(output),
        _ => None,
    });

    if let Some(output) = result {
        assert_eq!(output.query, "Rust programming language");
        assert!(!output.cached); // First request should not be cached
        println!("Search returned {} results", output.count);
        println!("Duration: {:.2}s", output.duration_seconds);

        for (idx, block) in output.results.iter().enumerate() {
            println!("\nResult block {}: {} hits", idx + 1, block.content.len());
            for (i, hit) in block.content.iter().enumerate() {
                println!("  {}. {} - {}", i + 1, hit.title, hit.url);
                if let Some(snippet) = &hit.snippet {
                    println!("     {}", snippet);
                }
            }
        }
    } else {
        panic!("No result event found in stream");
    }
}

#[tokio::test]
#[ignore] // Requires API key
async fn test_web_search_phase2_caching() {
    let tool = WebSearchTool::new();
    let params = WebSearchParams {
        query: "Rust caching test".to_string(),
        allowed_domains: vec![],
        blocked_domains: vec![],
    };
    let ctx = ToolContext {
        debug: true,
        ..Default::default()
    };

    // First search
    let stream1 = tool.execute(params.clone(), &ctx).await.unwrap();
    let events1: Vec<_> = stream1.collect().await;
    let result1 = events1.iter().find_map(|e| match e {
        ToolEvent::Result(output) => Some(output),
        _ => None,
    });

    assert!(result1.is_some());
    let output1 = result1.unwrap();
    assert!(!output1.cached);

    // Second search (should be cached)
    let stream2 = tool.execute(params.clone(), &ctx).await.unwrap();
    let events2: Vec<_> = stream2.collect().await;
    let result2 = events2.iter().find_map(|e| match e {
        ToolEvent::Result(output) => Some(output),
        _ => None,
    });

    assert!(result2.is_some());
    let output2 = result2.unwrap();
    assert!(output2.cached);
    assert_eq!(output2.count, output1.count);
}

#[tokio::test]
#[ignore] // Requires API key
async fn test_web_search_phase2_allowed_domains() {
    let tool = WebSearchTool::new();
    let params = WebSearchParams {
        query: "Rust".to_string(),
        allowed_domains: vec!["rust-lang.org".to_string()],
        blocked_domains: vec![],
    };
    let ctx = ToolContext::default();

    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    let result = events.iter().find_map(|e| match e {
        ToolEvent::Result(output) => Some(output),
        _ => None,
    });

    if let Some(output) = result {
        // Verify all results are from allowed domain
        for block in &output.results {
            for hit in &block.content {
                assert!(
                    hit.url.contains("rust-lang.org"),
                    "URL {} should be from rust-lang.org",
                    hit.url
                );
            }
        }
    }
}

#[tokio::test]
#[ignore] // Requires API key
async fn test_web_search_phase2_blocked_domains() {
    let tool = WebSearchTool::new();
    let params = WebSearchParams {
        query: "programming".to_string(),
        allowed_domains: vec![],
        blocked_domains: vec!["example.com".to_string()],
    };
    let ctx = ToolContext::default();

    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    let result = events.iter().find_map(|e| match e {
        ToolEvent::Result(output) => Some(output),
        _ => None,
    });

    if let Some(output) = result {
        // Verify no results are from blocked domain
        for block in &output.results {
            for hit in &block.content {
                assert!(
                    !hit.url.contains("example.com"),
                    "URL {} should not be from example.com",
                    hit.url
                );
            }
        }
    }
}
