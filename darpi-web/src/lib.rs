pub use hyper::{Body, Request, Response};

use derive_more::{Display, From};
//use http::StatusCode;
use serde::de;
use serde_urlencoded;
use std::{fmt, ops};

pub trait Responder<R, S> {
    fn respond(&self, req: Request<R>) -> Response<S>;
}

pub trait ErrorResponder<R, S, E>
where
    E: std::error::Error,
{
    fn respond_to_error(req: Request<R>, e: E) -> Response<S>;
}

/// A set of errors that can occur during parsing query strings
#[derive(Debug, Display, From)]
pub enum QueryPayloadError {
    /// Deserialize error
    #[display(fmt = "Query deserialize error: {}", _0)]
    Deserialize(serde::de::value::Error),
    #[display(fmt = "Empty query")]
    NotExist,
}

impl std::error::Error for QueryPayloadError {}

/// Return `BadRequest` for `QueryPayloadError`
// impl ResponseError for QueryPayloadError {
//     fn status_code(&self) -> StatusCode {
//         StatusCode::BAD_REQUEST
//     }
// }

#[derive(PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Query<T>(pub T);

impl<T, R, S, E> ErrorResponder<R, S, E> for Query<T>
where
    T: ErrorResponder<R, S, E>,
    E: std::error::Error,
{
    fn respond_to_error(req: Request<R>, e: E) -> Response<S> {
        T::respond_to_error(req, e)
    }
}

impl<T> Query<T>
where
    T: Default,
{
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
