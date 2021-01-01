use async_trait::async_trait;
use darpi::header::AUTHORIZATION;
use darpi::middleware::Expect;
use darpi::RequestParts;
use darpi::{middleware::RequestMiddleware, response::ResponderError, Body, Request};
use derive_more::{Display, From};
use futures_util::future::{err, ok, Ready};
use std::convert::Infallible;
use std::sync::Arc;

#[derive(Debug, Display, From)]
enum Error {
    #[display(fmt = "no auth header")]
    NoAuthHeaderError,
    #[display(fmt = "Access denied")]
    AccessDenied,
}

impl ResponderError for Error {}

#[derive(Eq, PartialEq)]
enum UserRole {
    Admin,
    Regular,
    None,
}

struct UserExtractor;

struct AccessControl;

#[test]
fn main() {}
