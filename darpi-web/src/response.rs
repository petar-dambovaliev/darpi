use hyper::{Body, Request, Response, StatusCode};

use bytes::BytesMut;
use http::header;
use std::convert::Infallible;
use std::io::Write;
use std::{fmt, io};

pub trait Responder<E>
where
    E: ResponderError,
{
    fn status_code(&self) -> StatusCode {
        StatusCode::OK
    }
    fn respond(self, _: &Request<Body>) -> Result<Response<Body>, E>;
}

impl Responder<Infallible> for &'static str {
    fn respond(self, _: &Request<Body>) -> Result<Response<Body>, Infallible> {
        Ok(Response::builder()
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .status(StatusCode::OK)
            .body(Body::from(self))
            .unwrap())
    }
}

impl Responder<Infallible> for &'static [u8] {
    fn respond(self, _: &Request<Body>) -> Result<Response<Body>, Infallible> {
        Ok(Response::builder()
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .status(StatusCode::OK)
            .body(Body::from(self))
            .unwrap())
    }
}

impl Responder<Infallible> for String {
    fn respond(self, _: &Request<Body>) -> Result<Response<Body>, Infallible> {
        Ok(Response::builder()
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .status(StatusCode::OK)
            .body(Body::from(self))
            .unwrap())
    }
}

impl<T, E> Responder<E> for Option<T>
where
    E: ResponderError,
    T: Responder<E>,
{
    fn respond(self, r: &Request<Body>) -> Result<Response<Body>, E> {
        match self {
            Some(t) => t.respond(r),
            None => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap()),
        }
    }
}

impl<T, E> Responder<E> for Result<T, E>
where
    E: ResponderError,
    T: Responder<E>,
{
    fn respond(self, b: &Request<Body>) -> Result<Response<Body>, E> {
        match self {
            Ok(t) => t.respond(b),
            Err(e) => Err(e),
        }
    }
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

impl ResponderError for Infallible {}

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
