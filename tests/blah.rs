use async_trait::async_trait;
use darpi::middleware::Expect;
use darpi::RequestParts;
use darpi::{response::ResponderError, Method};
use darpi_code_gen::{app, container, guard, handler, middleware};
use derive_more::{Display, From};
use shaku::{module, Component, Interface};
use std::sync::Arc;
use std::sync::Once;
use UserRole::Admin;

#[derive(Debug, Display, From)]
enum Error {
    #[display(fmt = "no auth header")]
    NoAuthHeaderError,
    #[display(fmt = "Access denied")]
    AccessDenied,
}

impl ResponderError for Error {}

#[derive(Eq, PartialEq, Ord, PartialOrd)]
enum UserRole {
    None,
    Regular,
    Admin,
}

#[derive(Component)]
#[shaku(interface = UserExtractor)]
struct UserExtractorImpl;

#[async_trait]
impl UserExtractor for UserExtractorImpl {
    async fn extract(&self, p: &RequestParts) -> Result<UserRole, Error> {
        Ok(UserRole::Admin)
    }
}

#[async_trait]
trait UserExtractor: Interface {
    async fn extract(&self, p: &RequestParts) -> Result<UserRole, Error>;
}

module! {
    Container {
        components = [UserExtractorImpl],
        providers = [],
    }
}

fn make_container() -> Container {
    Container::builder().build()
}

#[test]
fn main() {
    static INIT: Once = Once::new();
    let mut container;
    INIT.call_once(|| {
        container = make_container();
    });
}
