use darpi_code_gen::middleware;
use darpi_web::{middleware::Expect, request::PayloadError, Body};
use hyper::body::HttpBody;

#[middleware(Request)]
pub async fn body_size_limiter(b: &Body, size: Expect<u64>) -> Result<(), PayloadError> {
    if let Some(limit) = b.size_hint().upper() {
        if size < limit {
            return Err(PayloadError::Size(size.into_inner(), limit));
        }
    }
    Ok(())
}
