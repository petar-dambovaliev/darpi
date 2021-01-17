use crate::response::ResponderError;
use async_trait::async_trait;
use derive_more::{Display, From};
use http::HeaderValue;
use hyper::Body;
use serde::de;
use serde_urlencoded;

#[async_trait]
pub trait FromRequestBody<T, E>
where
    T: de::DeserializeOwned + 'static,
    E: ResponderError + 'static,
{
    async fn assert_content_type(_content_type: Option<&HeaderValue>) -> Result<(), E> {
        Ok(())
    }
    async fn extract(b: Body) -> Result<T, E>;
}

#[derive(Debug, Display, From)]
pub enum RequestErr {
    #[display(fmt = "Not found")]
    NotFound,
}

impl ResponderError for RequestErr {}

/// A set of errors that can occur during parsing query strings
#[derive(Debug, Display, From)]
pub enum PayloadError {
    /// Deserialize error
    #[display(fmt = "Payload deserialize error: {}", _0)]
    Deserialize(serde::de::value::Error),
    #[display(fmt = "Empty Payload")]
    NotExist,
    #[display(fmt = "Payload maximum {} exceeded: received {} bytes", _0, _1)]
    Size(u64, u64),
}

impl ResponderError for PayloadError {}

/// A set of errors that can occur during parsing query strings
#[derive(Debug, Display, From)]
pub enum QueryPayloadError {
    /// Deserialize error
    #[display(fmt = "Query deserialize error: {}", _0)]
    Deserialize(serde::de::value::Error),
    #[display(fmt = "Empty query")]
    NotExist,
}

impl ResponderError for QueryPayloadError {}
impl std::error::Error for QueryPayloadError {}

pub trait FromQuery<T, E> {
    fn from_query(query_str: &str) -> Result<T, E>
    where
        T: de::DeserializeOwned,
        E: ResponderError;
}

impl<T> FromQuery<T, QueryPayloadError> for T {
    fn from_query(query_str: &str) -> Result<T, QueryPayloadError>
    where
        T: de::DeserializeOwned,
    {
        serde_urlencoded::from_str::<T>(query_str)
            .map(|val| Ok(val))
            .unwrap_or_else(move |e| Err(QueryPayloadError::Deserialize(e)))
    }
}

#[derive(Debug, Display, From)]
pub enum PathError {
    #[display(fmt = "Path deserialize error: {}", _0)]
    Deserialize(String),
}

impl ResponderError for PathError {}
impl std::error::Error for PathError {}
