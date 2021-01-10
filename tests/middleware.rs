use async_trait::async_trait;
use darpi::{
    middleware::Expect, response::ResponderError, Body, Json, Method, Path, Query, RequestParts,
    Response,
};
use darpi_code_gen::{app, handler, middleware, path_type, query_type};
use derive_more::{Display, From};
use serde::{Deserialize, Serialize};
use shaku::{module, Component, Interface};
use std::sync::Arc;
use UserRole::Admin;

///////////// setup dependencies with shaku ///////////
trait Logger: Interface {
    fn log(&self, arg: &dyn std::fmt::Debug);
}

#[derive(Component)]
#[shaku(interface = Logger)]
struct MyLogger;
impl Logger for MyLogger {
    fn log(&self, arg: &dyn std::fmt::Debug) {
        println!("{:#?}", arg)
    }
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
        components = [UserExtractorImpl, MyLogger],
        providers = [],
    }
}

fn make_container() -> Container {
    Container::builder().build()
}

////////////////////////

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
    Regular,
    Admin,
}

// there are 2 types of middleware `Request` and `Response`
// the constant argument that needs to be present is &RequestParts
// everything else is up to the user
// Arc<dyn UserExtractor> types are injected from the shaku container
// Expect<UserRole> is a special type that is provided by the user when
// the middleware is linked to a handler. This allows the expected value
// to be different per handler + middleware
// middlewares are obgligated to return Result<(), impl ResponderErr>
// if a middleware returns an Err(e) all work is aborted and the coresponding
// response is sent to the user
#[middleware(Request)]
async fn access_control(
    user_role_extractor: Arc<dyn UserExtractor>,
    p: &RequestParts,
    expected_role: Expect<UserRole>,
) -> Result<(), Error> {
    let actual_role = user_role_extractor.extract(p).await?;

    if expected_role > actual_role {
        return Err(Error::AccessDenied);
    }
    Ok(())
}

#[path_type]
#[query_type]
#[derive(Deserialize, Serialize, Debug)]
pub struct Name {
    name: String,
}

// Path<Name> is extracted from the registered path "/hello_world/{name}"
// and it is always mandatory. A request without "{name}" will result
// in the request path not matching the handler. It will either match another
// handler or result in an 404
// Option<Query<Name>> is extracted from the url query "?name=jason"
// it is optional, as the type suggests. To make it mandatory, simply
// remove the Option type. If there is a Query<T> in the handler and
// an incoming request url does not contain the query parameters, it will
// result in an error response
#[handler(Container)]
async fn hello_world(p: Path<Name>, q: Option<Query<Name>>, logger: Arc<dyn Logger>) -> String {
    let other = q.map_or("nobody".to_owned(), |n| n.0.name);
    let response = format!("{} sends hello to {}", p.name, other);
    logger.log(&response);
    response
}

// the handler macro has 2 optional arguments
// the shaku container type and a collection of middlewares
// the enum variant `Admin` is coresponding to the middlewre `access_control`'s Expect<UserRole>
// Json<Name> is extracted from the request body
// failure to do so will result in an error response
#[handler(Container, [access_control(Admin)])]
async fn do_something(p: Path<Name>, payload: Json<Name>, logger: Arc<dyn Logger>) -> String {
    let response = format!("{} sends hello to {}", p.name, payload.name);
    logger.log(&response);
    response
}

// #[middleware(Response)]
// async fn log_response(r: &Response<Body>) -> Result<(), Error> {
//     println!("{:#?}", r);
//     Ok(())
// }

#[handler]
async fn do_something2() -> String {
    "response".to_owned()
}

#[tokio::test]
async fn main() {
    let address = format!("127.0.0.1:{}", 3000);
    app!({
        address: address,
        module: make_container => Container,
        bind: [
            {
                route: "/hello_world/{name}",
                //todo if user does not specify a method
                // let the user handle all methods on the same route
                // with a single handler
                method: Method::GET,
                handler: hello_world,
            },
            {
                route: "/hello_world/{name}",
                method: Method::POST,
                // the POST method allows this handler to have
                // Json<Name> as an argument
                handler: do_something
            },
            {
                route: "/hello_world/bla",
                method: Method::GET,
                // the POST method allows this handler to have
                // Json<Name> as an argument
                handler: do_something2
            },
        ],
    });
}
