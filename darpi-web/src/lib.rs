use derive_more::{Display, From};
//use http::StatusCode;
use serde::de;
use serde_urlencoded;
use std::{fmt, ops};

/// A set of errors that can occur during parsing query strings
#[derive(Debug, Display, From)]
pub enum QueryPayloadError {
    /// Deserialize error
    #[display(fmt = "Query deserialize error: {}", _0)]
    Deserialize(serde::de::value::Error),
}

impl std::error::Error for QueryPayloadError {}

/// Return `BadRequest` for `QueryPayloadError`
// impl ResponseError for QueryPayloadError {
//     fn status_code(&self) -> StatusCode {
//         StatusCode::BAD_REQUEST
//     }
// }

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
