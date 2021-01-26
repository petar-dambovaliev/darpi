#![forbid(unsafe_code)]

pub use hyper::{body::HttpBody, Body, Request, Response, StatusCode};
pub use json::Json;

pub mod json;
pub mod logger;
pub mod request;
pub mod response;
pub mod xml;
pub mod yaml;

use http::header::FORWARDED;
use http::request::{Parts as RequestParts, Parts};
use std::time::Instant;

pub trait ReqFormatter {
    fn format_req(&self, b: &Body, rp: &RequestParts) -> String {
        let mut content = vec!["[darpi::request]".to_string()];

        if let Some(forwarded) = rp.headers.get(FORWARDED) {
            let forwarded = format!(
                "remote_ip: {}",
                forwarded.to_str().map_err(|_| "").expect("never to happen")
            );
            content.push(forwarded);
        }

        let now = format!("when: {:#?}", Instant::now());
        content.push(now);

        let uri = format!("uri: {:#?}", rp.uri);
        content.push(uri);

        let size = format!("body_size: {:#?}", b.size_hint());
        content.push(size);

        content.join(" ").into()
    }
}

pub trait RespFormatter {
    fn format_resp(&self, start: &Instant, r: &Response<Body>) -> String {
        let mut content = vec!["[darpi::response]".to_string()];

        if let Some(forwarded) = r.headers().get(FORWARDED) {
            let forwarded = format!(
                "remote_ip: {}",
                forwarded.to_str().map_err(|_| "").expect("never to happen")
            );
            content.push(forwarded);
        }

        let now = format!("when: {:#?}", Instant::now());
        content.push(now);

        let took = format!("took: {:#?}", start.elapsed());

        content.push(took);

        let size = format!("body_size: {:#?}", r.size_hint());
        content.push(size);

        content.join(" ").into()
    }
}
