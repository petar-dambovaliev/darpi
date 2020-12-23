#![forbid(unsafe_code)]

pub use darpi_code_gen::{handler, path_type, run, QueryType};
pub use darpi_web::{request, request::Path, response, route, route::ReqRoute, route::Route};
pub use futures;
pub use http::{Method, StatusCode};
pub use hyper::{service, Body, Request, Response, Server};
use serde::{de, Deserialize, Deserializer};
pub use serde_json;
use std::fmt::Display;
use std::str::FromStr;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

pub fn from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(de::Error::custom)
}
