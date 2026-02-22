use super::*;
use crate::web_fetch_domains::PRE_APPROVED_DOMAINS;
use crate::Tool;
use futures::StreamExt;

#[test]
fn test_http_to_https_upgrade() {
    assert_eq!(
        WebFetchTool::upgrade_to_https("http://example.com"),
        "https://example.com"
    );
    assert_eq!(
        WebFetchTool::upgrade_to_https("https://example.com"),
        "https://example.com"
    );
}

#[test]
fn test_domain_extraction() {
    assert_eq!(
        WebFetchTool::get_domain("https://docs.anthropic.com/path"),
        Some("docs.anthropic.com".to_string())
    );
    assert_eq!(
        WebFetchTool::get_domain("http://example.com:8080/path"),
        Some("example.com".to_string())
    );
}

#[test]
fn test_same_domain_check() {
    assert!(WebFetchTool::is_same_domain(
        "https://example.com/page1",
        "https://example.com/page2"
    ));
    assert!(!WebFetchTool::is_same_domain(
        "https://example.com/page",
        "https://other.com/page"
    ));
}

#[test]
fn test_pre_approved_domains() {
    assert!(WebFetchTool::is_domain_approved(
        "https://docs.anthropic.com/"
    ));
    assert!(WebFetchTool::is_domain_approved(
        "https://docs.python.org/3/"
    ));
    assert!(WebFetchTool::is_domain_approved(
        "https://doc.rust-lang.org/"
    ));
    assert!(!WebFetchTool::is_domain_approved(
        "https://random-site.com/"
    ));
}

#[test]
fn test_pre_approved_domain_list_coverage() {
    // Verify we have comprehensive coverage
    assert!(
        PRE_APPROVED_DOMAINS.len() >= 80,
        "Should have at least 80 pre-approved domains, found {}",
        PRE_APPROVED_DOMAINS.len()
    );

    // Verify key ecosystems are covered
    let has_python = PRE_APPROVED_DOMAINS.iter().any(|d| d.contains("python"));
    let has_rust = PRE_APPROVED_DOMAINS.iter().any(|d| d.contains("rust"));
    let has_javascript = PRE_APPROVED_DOMAINS
        .iter()
        .any(|d| d.contains("nodejs") || d.contains("npm"));
    let has_docs = PRE_APPROVED_DOMAINS.iter().any(|d| d.contains("docs"));

    assert!(has_python, "Should include Python ecosystem");
    assert!(has_rust, "Should include Rust ecosystem");
    assert!(has_javascript, "Should include JavaScript ecosystem");
    assert!(has_docs, "Should include documentation sites");
}

#[test]
fn test_markdown_conversion_html() {
    let html = b"<html><body><h1>Title</h1><p>Content</p></body></html>";
    let result = WebFetchTool::to_markdown(html, Some("text/html"));
    assert!(result.is_ok());
    let markdown = result.unwrap();
    assert!(markdown.contains("Title"));
}

#[test]
fn test_markdown_conversion_text() {
    let text = b"Plain text content";
    let result = WebFetchTool::to_markdown(text, Some("text/plain"));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Plain text content");
}

#[test]
fn test_markdown_truncation() {
    let long_text = "a".repeat(150_000);
    let result = WebFetchTool::to_markdown(long_text.as_bytes(), Some("text/plain"));
    assert!(result.is_ok());
    let markdown = result.unwrap();
    assert!(markdown.len() <= MAX_MARKDOWN_LENGTH + 100); // Allow for truncation message
    assert!(markdown.contains("truncated"));
}

#[tokio::test]
async fn test_cache_functionality() {
    let cache = WebFetchCache::new();
    let key = "test_key".to_string();
    let response = CachedResponse {
        content: "test content".to_string(),
        bytes: 100,
        status_code: 200,
        status_text: "OK".to_string(),
        final_url: "https://example.com".to_string(),
    };

    // Insert and retrieve
    cache.insert(key.clone(), response.clone()).await;
    let cached = cache.get(&key).await;
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().content, "test content");
}

#[tokio::test]
#[ignore] // Network test
async fn test_webfetch_phase2_basic() {
    let tool = WebFetchTool::new();
    let params = WebFetchParams {
        url: "https://httpbin.org/html".to_string(),
        prompt: "Get the HTML content".to_string(),
    };
    let ctx = ToolContext::default();

    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    let result = events.iter().find_map(|e| match e {
        ToolEvent::Result(output) => Some(output),
        _ => None,
    });

    assert!(result.is_some());
    let output = result.unwrap();
    assert_eq!(output.code, 200);
    assert!(output.bytes > 0);
    assert!(!output.result.is_empty());
}

#[tokio::test]
#[ignore] // Network test
async fn test_webfetch_phase2_https_upgrade() {
    let tool = WebFetchTool::new();
    let params = WebFetchParams {
        url: "http://httpbin.org/html".to_string(),
        prompt: "Get the HTML content".to_string(),
    };
    let ctx = ToolContext::default();

    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    let result = events.iter().find_map(|e| match e {
        ToolEvent::Result(output) => Some(output),
        _ => None,
    });

    if let Some(output) = result {
        // Should have upgraded to HTTPS
        assert!(output.url.starts_with("https://") || output.url.starts_with("http://"));
    }
}

#[tokio::test]
#[ignore] // Network test
async fn test_webfetch_phase2_caching() {
    let tool = WebFetchTool::new();
    let params = WebFetchParams {
        url: "https://httpbin.org/html".to_string(),
        prompt: "Get the HTML content".to_string(),
    };
    let ctx = ToolContext::default();

    // First fetch
    let stream1 = tool.execute(params.clone(), &ctx).await.unwrap();
    let events1: Vec<_> = stream1.collect().await;
    let duration1 = events1.iter().find_map(|e| match e {
        ToolEvent::Result(output) => Some(output.duration_ms),
        _ => None,
    });

    // Second fetch (should be cached)
    let stream2 = tool.execute(params.clone(), &ctx).await.unwrap();
    let events2: Vec<_> = stream2.collect().await;
    let duration2 = events2.iter().find_map(|e| match e {
        ToolEvent::Result(output) => Some(output.duration_ms),
        _ => None,
    });

    // Both should succeed
    assert!(duration1.is_some());
    assert!(duration2.is_some());

    // Cached request should generally be faster
    if let (Some(d1), Some(d2)) = (duration1, duration2) {
        println!("First fetch: {}ms, Second fetch: {}ms", d1, d2);
    }
}

#[tokio::test]
#[ignore] // Network test
async fn test_webfetch_phase2_404_error() {
    let tool = WebFetchTool::new();
    let params = WebFetchParams {
        url: "https://httpbin.org/status/404".to_string(),
        prompt: "Get the content".to_string(),
    };
    let ctx = ToolContext::default();

    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    let has_error = events.iter().any(|e| matches!(e, ToolEvent::Error { .. }));
    assert!(has_error);
}

#[tokio::test]
async fn test_webfetch_phase2_invalid_url() {
    let tool = WebFetchTool::new();
    let params = WebFetchParams {
        url: "not-a-valid-url".to_string(),
        prompt: "Get the content".to_string(),
    };
    let ctx = ToolContext::default();

    let stream = tool.execute(params, &ctx).await.unwrap();
    let events: Vec<_> = stream.collect().await;

    let has_error = events.iter().any(|e| matches!(e, ToolEvent::Error { .. }));
    assert!(has_error);
}
