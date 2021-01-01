use async_trait::async_trait;
use darpi::header::AUTHORIZATION;
use darpi::middleware::Expect;
use darpi::RequestParts;
use darpi::{middleware::RequestMiddleware, response::ResponderError, Body, Request};
use darpi_web::request::FromRequestParts;
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

#[async_trait]
impl FromRequestParts<UserRole, Error> for UserExtractor {
    async fn extract(&self, p: &RequestParts) -> Result<UserRole, Error> {
        Ok(UserRole::Admin)
    }
}

struct AccessControl;

impl AccessControl {
    // this will be a function defined by the user and transformed by a "middleware" macro
    // try to figure out if all arguments without Arc or dyn can be considered for
    // getting from handler
    async fn access_control(
        expected_role: Expect<UserRole>,
        user_role_extractor: Arc<dyn FromRequestParts<UserRole, Error>>,
        p: &RequestParts,
    ) -> Result<(), Error> {
        let actual_role = user_role_extractor.extract(p).await?;

        if expected_role != actual_role {
            return Err(Error::AccessDenied);
        }
        Ok(())
    }
}

pub trait ExpectValue<T> {
    fn expect() -> T;
}

// impl ExpectValue<UserRole> for AccessControl {
//     fn expect() -> UserRole {
//         UserRole::Admin
//     }
// }

fn expect_func<T, R>() -> R
where
    T: ExpectValue<R>,
{
    T::expect()
}

// this implementation will be done by the "middleware" macro
// along with the user_defined_role variable
impl AccessControl {
    async fn call<T>(&self, p: &RequestParts) -> Result<(), Error>
    where
        T: ExpectValue<UserRole>,
    {
        let user_defined_role = expect_func::<T, UserRole>();
        Self::access_control(Expect(user_defined_role), Arc::new(UserExtractor), p).await?;
        Ok(())
    }
}

#[test]
fn main() {}
