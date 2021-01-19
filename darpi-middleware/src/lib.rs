pub mod auth;

use darpi::{middleware, request::PayloadError, Body, HttpBody};

#[middleware(Request)]
pub async fn body_size_limit(#[body] b: &Body, #[handler] size: u64) -> Result<(), PayloadError> {
    if let Some(limit) = b.size_hint().upper() {
        if size < limit {
            return Err(PayloadError::Size(size, limit));
        }
    }
    Ok(())
}
