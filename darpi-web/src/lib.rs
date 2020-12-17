#![forbid(unsafe_code)]
mod json;
mod responder;

pub use hyper::{Body, Request, Response, StatusCode};

use bytes::BytesMut;
use derive_more::{Display, From};
use futures::Future;
use http::header;
use serde::de;
use serde::de::DeserializeOwned;
use serde_urlencoded;
use std::convert::Infallible;
use std::io::Write;
use std::{fmt, io, ops};

pub trait FromRequest<T, E>
where
    T: DeserializeOwned,
    E: ResponderError,
{
    type Future: Future<Output = Result<T, E>>;
    fn extract(_: Body) -> Self::Future;
}

pub trait Responder<E>
where
    E: ResponderError,
{
    fn status_code(&self) -> StatusCode {
        StatusCode::OK
    }
    fn respond(self, _: &Request<Body>) -> Result<Response<Body>, E>;
}

pub trait ResponderError: fmt::Display {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
    fn respond_err(&self, _: &Request<Body>) -> Response<Body> {
        let mut buf = BytesMut::new();
        let _ = write!(ByteWriter(&mut buf), "{}", self);

        Response::builder()
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .status(self.status_code())
            .body(Body::from(buf.to_vec()))
            .expect("this cannot happen")
    }
}

struct ByteWriter<'a>(pub &'a mut BytesMut);

impl<'a> Write for ByteWriter<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
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

impl ResponderError for Infallible {}
impl ResponderError for QueryPayloadError {}
impl std::error::Error for QueryPayloadError {}

/// Return `BadRequest` for `QueryPayloadError`
// impl ResponseError for QueryPayloadError {
//     fn status_code(&self) -> StatusCode {
//         StatusCode::BAD_REQUEST
//     }
// }

pub trait ErrResponder<E, B>
where
    E: std::error::Error,
{
    fn respond_err(e: E) -> Response<B>;
}

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
