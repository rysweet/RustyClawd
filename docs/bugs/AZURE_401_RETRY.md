# Fix: Azure 401 Unauthorized Now Retryable

## Problem

Azure AD bearer tokens expire every hour. When a cached token expires mid-session,
in-flight requests receive HTTP 401 Unauthorized. The retry logic classified 401
as non-retryable, causing permanent failure instead of transparently refreshing
the token and retrying.

**Impact**: Long-running sessions using Azure AI Foundry backend would fail
after ~1 hour with no recovery path short of restarting the CLI.

## Root Cause

`ClientError::is_retryable()` did not include `Unauthorized` in its match arms.
The retry loop in `Client::with_retry()` saw 401 errors as terminal and
propagated them immediately.

## Solution

Three changes in `crates/core/src/client/`:

### 1. `error.rs` -- Classify 401 as retryable

Added `ClientError::Unauthorized(..)` to the `is_retryable()` match. Also added
`is_auth_error()` helper so callers can distinguish auth errors from other
retryable errors (e.g. to invalidate cached credentials before retry).

```rust
pub fn is_retryable(&self) -> bool {
    matches!(self,
        ClientError::RateLimited { .. }
        | ClientError::ServiceUnavailable { .. }
        | ClientError::ServerError(500..=599, _)
        | ClientError::Timeout(_)
        | ClientError::NetworkError(_)
        | ClientError::DnsError(_)
        | ClientError::ConnectionError(_)
        | ClientError::Unauthorized(_)   // <-- NEW
    )
}

pub fn is_auth_error(&self) -> bool {  // <-- NEW
    matches!(self, ClientError::Unauthorized(_))
}
```

### 2. `azure_foundry.rs` -- Token cache invalidation

Added `invalidate_cached_token()` to `AzureAuth` that clears the `RwLock<Option<CachedToken>>`.
This forces the next `get_token()` call to acquire a fresh token via `az account get-access-token`.

```rust
pub async fn invalidate_cached_token(&self) {
    let mut cache = self.cached_token.write().await;
    *cache = None;
}
```

### 3. `mod.rs` -- Wire auth retry into the retry loop

In `Client::with_retry()`, before sleeping for the retry delay, check
`is_auth_error()`. If true and the Azure backend is active, call
`invalidate_cached_token()` so the retry gets a fresh bearer token.

```rust
if e.is_auth_error() {
    if let Some(ref auth) = self.azure_auth {
        tracing::info!("Auth error on {label} — invalidating cached Azure token");
        auth.invalidate_cached_token().await;
    }
}
```

## Design Decisions

| Decision | Rationale | Alternatives Considered |
|---|---|---|
| 401 is retryable globally | Azure tokens expire predictably; retrying with a fresh token succeeds. For Anthropic/Copilot backends, a 401 retry is harmless (will fail again quickly). | Only retryable for Azure backend -- rejected because it complicates `is_retryable()` with backend awareness. |
| Separate `is_auth_error()` helper | Callers need to distinguish "invalidate credentials then retry" from "just retry". Keeps `is_retryable()` as a simple boolean. | Embedding invalidation logic inside `is_retryable()` -- rejected because error classification should be side-effect-free. |
| `invalidate_cached_token()` clears to `None` | Simple and correct. `get_token()` already handles the `None` case by acquiring a new token. | Setting `expires_at` to past -- rejected as unnecessarily indirect. |

## Testing

- `test_is_retryable`: Verifies `Unauthorized` is retryable
- `test_is_auth_error`: Verifies only `Unauthorized` returns true
- `test_invalidate_cached_token`: Injects a cached token, invalidates, confirms `None`

## Files Changed

- `crates/core/src/client/error.rs`
- `crates/core/src/client/azure_foundry.rs`
- `crates/core/src/client/mod.rs`
