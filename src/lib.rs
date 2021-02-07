#![forbid(unsafe_code)]

pub use darpi_code_gen::{
    app, handler, job, middleware, req_formatter, resp_formatter, Path, Query,
};
pub use darpi_web::{
    handler::Args, handler::Handler, job, job::RequestJob, job::ResponseJob, logger,
    logger::ReqFormatter, logger::RespFormatter, middleware::RequestMiddleware,
    middleware::ResponseMiddleware, request, response, xml::Xml, yaml::Yaml, Json,
};

pub use async_trait::async_trait;
pub use chrono;
pub use darpi_route::{ReqRoute, Route};
pub use futures;
pub use http::{header, request::Parts as RequestParts, Method, StatusCode};
pub use hyper::{self, body, body::HttpBody, service, Body, Error, Request, Response, Server};
use serde::{de, Deserialize, Deserializer};
pub use serde_json;
use std::fmt::Display;
use std::str::FromStr;

pub fn from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(de::Error::custom)
}
