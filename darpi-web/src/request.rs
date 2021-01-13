use crate::response::ResponderError;
use async_trait::async_trait;
use derive_more::{Display, From};
use hyper::{body::HttpBody, Body};
use serde::{de, Deserialize, Deserializer};
use serde_urlencoded;
use std::{fmt, ops};

pub struct ExtractBody<T>(pub T);

impl<T> ExtractBody<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> ops::Deref for ExtractBody<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> ops::DerefMut for ExtractBody<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

#[async_trait]
impl<T, E> FromRequestBody<T, E> for ExtractBody<T>
where
    T: FromRequestBody<T, E> + 'static,
    E: ResponderError + 'static,
{
    async fn extract(b: Body) -> Result<T, E> {
        T::extract(b).await
    }
}

#[async_trait]
pub trait FromRequestBody<T, E> {
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

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Query<T>(pub T);

impl<T> Query<T> {
    pub fn into_inner(self) -> T {
        self.0
    }

    pub fn from_query(query_str: &str) -> Result<Self, QueryPayloadError>
    where
        T: de::DeserializeOwned,
    {
        serde_urlencoded::from_str::<T>(query_str)
            .map(|val| Ok(Query(val)))
            .unwrap_or_else(move |e| Err(QueryPayloadError::Deserialize(e)))
    }
}

impl<T> ops::Deref for Query<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> ops::DerefMut for Query<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T: fmt::Debug> fmt::Debug for Query<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: fmt::Display> fmt::Display for Query<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

pub struct Path<T>(pub T);

impl<T> Path<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<'de, T> Deserialize<'de> for Path<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let t = T::deserialize(deserializer)?;
        Ok(Path(t))
    }
}

impl<T> AsRef<T> for Path<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> ops::Deref for Path<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> ops::DerefMut for Path<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> From<T> for Path<T> {
    fn from(inner: T) -> Path<T> {
        Path(inner)
    }
}

impl<T: fmt::Debug> fmt::Debug for Path<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: fmt::Display> fmt::Display for Path<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Display, From)]
pub enum PathError {
    #[display(fmt = "Path deserialize error: {}", _0)]
    Deserialize(String),
}

impl ResponderError for PathError {}

impl std::error::Error for PathError {}
