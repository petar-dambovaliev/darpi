#![forbid(unsafe_code)]

pub use darpi_code_gen::{app, handler, path_type, query_type};
pub use darpi_web::{
    request, request::Path, request::Query, response, route, route::ReqRoute, route::Route, Json,
};
pub use futures;
pub use http::{Method, StatusCode};
pub use hyper::{service, Body, Error, Request, Response, Server};
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
