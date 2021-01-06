use hyper::{Body, Response, StatusCode};

use bytes::BytesMut;
use http::header;
use serde::export::Formatter;
use std::convert::Infallible;
use std::io::Write;
use std::{fmt, io};

pub trait Responder {
    fn status_code(&self) -> StatusCode {
        StatusCode::OK
    }
    fn respond(self) -> Response<Body>;
}

impl Responder for &'static str {
    fn respond(self) -> Response<Body> {
        Response::builder()
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .status(StatusCode::OK)
            .body(Body::from(self))
            .unwrap()
    }
}

impl Responder for &'static [u8] {
    fn respond(self) -> Response<Body> {
        Response::builder()
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .status(StatusCode::OK)
            .body(Body::from(self))
            .unwrap()
    }
}

impl Responder for String {
    fn respond(self) -> Response<Body> {
        Response::builder()
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .status(StatusCode::OK)
            .body(Body::from(self))
            .unwrap()
    }
}

impl Responder for () {
    fn respond(self) -> Response<Body> {
        Response::builder()
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap()
    }
}

impl<T> Responder for Option<T>
where
    T: Responder,
{
    fn respond(self) -> Response<Body> {
        match self {
            Some(t) => t.respond(),
            None => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap(),
        }
    }
}

impl<T, E> Responder for Result<T, E>
where
    E: ResponderError,
    T: Responder,
{
    fn respond(self) -> Response<Body> {
        match self {
            Ok(t) => t.respond(),
            Err(e) => e.respond_err(),
        }
    }
}

pub trait ResponderError: fmt::Display {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
    fn respond_err(&self) -> Response<Body> {
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
