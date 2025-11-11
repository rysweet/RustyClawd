//! Server-Sent Events (SSE) stream parser for Anthropic API
//!
//! Parses SSE format:
//! ```text
//! event: message_start
//! data: {"type":"message_start",...}
//!
//! event: content_block_delta
//! data: {"type":"content_block_delta",...}
//! ```

use bytes::Bytes;
use futures::stream::{Stream, StreamExt};
use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::error::{ClientError, ClientResult};
use super::types::StreamEvent;

/// SSE event parsed from stream
#[derive(Debug)]
pub struct SseEvent {
    pub event_type: Option<String>,
    pub data: String,
}

/// Stream adapter that parses SSE format from byte chunks
#[pin_project]
pub struct SseStream<S> {
    #[pin]
    inner: S,
    buffer: String,
}

impl<S> SseStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>>,
{
    pub fn new(stream: S) -> Self {
        Self {
            inner: stream,
            buffer: String::new(),
        }
    }
}

/// Parse buffered data into SSE events (free function to avoid borrow issues)
fn parse_sse_events(buffer: &mut String) -> Vec<SseEvent> {
    let mut events = Vec::new();
    let mut current_event: Option<String> = None;
    let mut current_data: Vec<String> = Vec::new();

    // Split by double newline (event separator)
    let parts: Vec<&str> = buffer.split("\n\n").collect();

    // Keep the last incomplete part in buffer
    if parts.len() > 1 {
        for part in &parts[..parts.len() - 1] {
            if part.is_empty() {
                continue;
            }

            // Parse individual lines in this event
            for line in part.lines() {
                if line.is_empty() {
                    continue;
                }

                if let Some(event_type) = line.strip_prefix("event: ") {
                    current_event = Some(event_type.trim().to_string());
                } else if let Some(data) = line.strip_prefix("data: ") {
                    current_data.push(data.to_string());
                }
                // Ignore other fields like "id:", "retry:", etc.
            }

            // Emit event if we have data
            if !current_data.is_empty() {
                events.push(SseEvent {
                    event_type: current_event.take(),
                    data: current_data.join("\n"),
                });
                current_data.clear();
            }
        }

        // Keep the last incomplete part
        *buffer = parts[parts.len() - 1].to_string();
    }

    events
}

impl<S> Stream for SseStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>>,
{
    type Item = ClientResult<SseEvent>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            // Try to parse events from buffer first
            let events = parse_sse_events(&mut this.buffer);
            if !events.is_empty() {
                return Poll::Ready(Some(Ok(events.into_iter().next().unwrap())));
            }

            // Need more data, poll the inner stream
            match this.inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    // Add new data to buffer
                    match std::str::from_utf8(&bytes) {
                        Ok(text) => {
                            this.buffer.push_str(text);
                            // Continue loop to try parsing again
                        }
                        Err(e) => {
                            return Poll::Ready(Some(Err(ClientError::Stream(format!(
                                "Invalid UTF-8: {}",
                                e
                            )))));
                        }
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(ClientError::Request(e))));
                }
                Poll::Ready(None) => {
                    // Stream ended, check if there's remaining data
                    if !this.buffer.is_empty() {
                        let remaining = this.buffer.clone();
                        this.buffer.clear();
                        if !remaining.trim().is_empty() {
                            return Poll::Ready(Some(Err(ClientError::InvalidSSE(format!(
                                "Stream ended with incomplete data: {}",
                                remaining
                            )))));
                        }
                    }
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Stream adapter that parses SSE events into StreamEvent types
#[pin_project]
pub struct EventStream<S> {
    #[pin]
    inner: SseStream<S>,
}

impl<S> EventStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>>,
{
    pub fn new(stream: S) -> Self {
        Self {
            inner: SseStream::new(stream),
        }
    }
}

impl<S> Stream for EventStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>>,
{
    type Item = ClientResult<StreamEvent>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match this.inner.poll_next(cx) {
            Poll::Ready(Some(Ok(sse_event))) => {
                // Parse the JSON data into StreamEvent
                match serde_json::from_str::<StreamEvent>(&sse_event.data) {
                    Ok(event) => Poll::Ready(Some(Ok(event))),
                    Err(e) => Poll::Ready(Some(Err(ClientError::JsonParse(e)))),
                }
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Helper to extract text from stream events
pub async fn collect_text_chunks<S>(mut stream: S) -> ClientResult<Vec<String>>
where
    S: Stream<Item = ClientResult<StreamEvent>> + Unpin,
{
    let mut chunks = Vec::new();

    while let Some(result) = stream.next().await {
        let event = result?;
        match event {
            StreamEvent::ContentBlockDelta { delta, .. } => {
                let super::types::ContentDelta::TextDelta { text } = delta;
                chunks.push(text);
            }
            StreamEvent::Error { error } => {
                return Err(ClientError::Api(error.message));
            }
            _ => {
                // Ignore other event types for text collection
            }
        }
    }

    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    #[tokio::test]
    async fn test_sse_parsing() {
        let data = "event: message_start\ndata: {\"type\":\"message_start\"}\n\n";
        let bytes = Bytes::from(data);
        let stream = stream::iter(vec![Ok(bytes)]);

        let mut sse_stream = SseStream::new(stream);
        let event = sse_stream.next().await.unwrap().unwrap();

        assert_eq!(event.event_type, Some("message_start".to_string()));
        assert!(event.data.contains("message_start"));
    }

    #[tokio::test]
    async fn test_multiline_data() {
        let data = "event: test\ndata: line1\ndata: line2\n\n";
        let bytes = Bytes::from(data);
        let stream = stream::iter(vec![Ok(bytes)]);

        let mut sse_stream = SseStream::new(stream);
        let event = sse_stream.next().await.unwrap().unwrap();

        assert_eq!(event.data, "line1\nline2");
    }
}
