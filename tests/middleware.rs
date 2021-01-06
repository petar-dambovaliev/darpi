use async_trait::async_trait;
use darpi::middleware::Expect;
use darpi::RequestParts;
use darpi::{response::ResponderError, Method};
use darpi_code_gen::{app, guard, handler, middleware};
use derive_more::{Display, From};
use shaku::{module, Component, HasComponent, Interface};
use std::sync::Arc;
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

#[middleware(Request)]
async fn access_control(
    expected_role: Expect<UserRole>,
    user_role_extractor: Arc<dyn UserExtractor>,
    p: &RequestParts,
) -> Result<(), Error> {
    if expected_role == UserRole::None {
        return Ok(());
    }
    let actual_role = user_role_extractor.extract(p).await?;

    if expected_role > actual_role {
        return Err(Error::AccessDenied);
    }
    Ok(())
}

#[guard([access_control(Admin)])]
#[handler(Container)]
async fn hello_world(logger: Arc<dyn UserExtractor>) {
    //do something
}

#[tokio::test]
async fn main() {
    app!({
        address: "127.0.0.1:3000",
        module: make_container => Container,
        bind: [
            {
                route: "/hello_world",
                method: Method::GET,
                handler: hello_world,
            },
        ],
    });
}
