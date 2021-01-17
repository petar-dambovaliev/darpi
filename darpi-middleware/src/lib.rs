use darpi_code_gen::middleware;
use darpi_web::{request::PayloadError, Body};
use hyper::body::HttpBody;

#[middleware(Request)]
pub async fn body_size_limit(#[body] b: &Body, #[expect] size: u64) -> Result<(), PayloadError> {
    if let Some(limit) = b.size_hint().upper() {
        if size < limit {
            return Err(PayloadError::Size(size, limit));
        }
    }
    Ok(())
}
