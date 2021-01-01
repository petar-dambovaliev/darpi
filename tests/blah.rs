use async_trait::async_trait;
use darpi::header::AUTHORIZATION;
use darpi::middleware::Expect;
use darpi::RequestParts;
use darpi::{middleware::RequestMiddleware, response::ResponderError, Body, Request};
use darpi_code_gen::middleware;
use derive_more::{Display, From};
use futures_util::future::{err, ok, Ready};
use http::request::Parts;
use shaku::{module, Component, Interface};
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

#[derive(Component)]
#[shaku(interface = UserExtractorInterface)]
struct UserExtractor;

#[async_trait]
impl UserExtractorInterface for UserExtractor {
    async fn extract(&self, p: &RequestParts) -> Result<UserRole, Error> {
        Ok(UserRole::Admin)
    }
}

#[async_trait]
trait UserExtractorInterface: Interface {
    async fn extract(&self, p: &RequestParts) -> Result<UserRole, Error>;
}

#[middleware(Request)]
async fn access_control(
    expected_role: Expect<UserRole>,
    user_role_extractor: Arc<dyn UserExtractorInterface>,
    p: &RequestParts,
) -> Result<(), Error> {
    let actual_role = user_role_extractor.extract(p).await?;

    if expected_role != actual_role {
        return Err(Error::AccessDenied);
    }
    Ok(())
}

#[test]
fn main() {}
