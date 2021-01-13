### darpi

A web api framework with speed and safety in mind.
One of the big goals is to catch all errors at `compile time`, if possible.
The framework uses [hyper](https://github.com/hyperium/hyper) and ideally, the performance should be as if you were using `hyper` yourself.
The framework also uses [shaku](https://github.com/Mcat12/shaku) for `compile time` verifiable dependency injection.

The framework is in early development.
All feedback is appreciated.

An example of a simple app

`Cargo.toml`
```
[dependencies]
darpi = {git = "https://github.com/petar-dambovaliev/darpi.git", branch = "master"}
serde = { version = "1.0", features = ["derive"] }
tokio = {version = "0.2.11", features = ["full"]}
shaku = {version = "0.5.0", features = ["thread_safe"]}
```
`main.rs`
```rust
use async_trait::async_trait;
use darpi::{
    app, handler, middleware, middleware::Expect, path_type, query_type, request::ExtractBody,
    response::ResponderError, Body, Json, Method, Path, Query, RequestParts,
};
use derive_more::{Display, From};
use serde::{Deserialize, Serialize};
use shaku::{module, Component, Interface};
use std::sync::Arc;
use UserRole::Admin;
 
///////////// setup dependencies with shaku ///////////
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
#[handler]
async fn hello_world(p: Path<Name>, q: Option<Query<Name>>) -> String {
    let other = q.map_or("nobody".to_owned(), |n| n.0.name);
    format!("{} sends hello to {}", p.name, other)
}

// the handler macro has 2 optional arguments
// the shaku container type and a collection of middlewares
// the enum variant `Admin` is corresponding to the middlewre `access_control`'s Expect<UserRole>
// ExtractBody<T<Name>> is extracted from the request body
// where T is the format and it has to implement the trait FromRequestBody
// Json, Yaml and Xml are supported out of the box
// failure to extract will result in an error response
#[handler(Container, [access_control(Admin)])]
async fn do_something(p: Path<Name>, payload: ExtractBody<Json<Name>>) -> String {
    format!("{} sends hello to {}", p.name, payload.name)
}
 
 #[tokio::main]
 async fn main() -> Result<(), darpi::Error> {
    // the `app` macro creates a server and allows the user to call
    // the method `run` and await on that future
     app!({
        // if the address is a string literal, it is verified at compile time
        // if it is a string variable, obviously, it won't
         address: "127.0.0.1:3000",
         // via the container we inject our dependencies
         // in this case, MyLogger type
         // any handler that has the trait Logger as an argument
         // will be given MyLogger
         module: make_container => Container,
         bind: [
             {
                 // When a path argument is defined in the route,
                 // the handler is required to have Path<T> as an argument
                 // if not present, it will result in a compilation error
                 route: "/hello_world/{name}",
                 method: Method::GET,
                 // handlers bound with a GET method are not allowed
                 // to request a body(payload) from the request.
                 // Json<T> argument would result in a compilation error
                 handler: hello_world
             },
             {
                 route: "/hello_world/{name}",
                 method: Method::POST,
                 // the POST method allows this handler to have
                 // Json<Name> as an argument
                 handler: do_something
             },
         ],
     })
    .run()
    .await
 }
```
