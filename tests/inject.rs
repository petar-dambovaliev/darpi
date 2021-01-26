use darpi::request::PayloadError;
use darpi::{app, handler, middleware, Body, Error, HttpBody, Json, Method, Path, Query};
use serde::{Deserialize, Serialize};
use shaku::module;

fn make_container() -> Container {
    let module = Container::builder().build();
    module
}

module! {
    Container {
        components = [],
        providers = [],
    }
}

#[derive(Deserialize, Serialize, Debug, Path, Query)]
pub struct Name {
    name: String,
}

#[middleware(Request)]
async fn first(#[handler] size: u64) -> Result<u64, PayloadError> {
    Ok(size + 1)
}

#[middleware(Response)]
async fn second(#[handler] size: u64) -> Result<u64, PayloadError> {
    Ok(size + 1)
}

//todo make sure the middleware index is a correct value within bounds of the current element
#[handler([first(1), second(middleware(0))])]
async fn do_something_else() -> String {
    "do something".to_string()
}

// #[path] tells the handler macro that it should decode the path arguments "/hello_world/{name}" into Name
// and it is always mandatory. A request without "{name}" will result
// in the request path not matching the handler. It will either match another
// handler or result in an 404
// #[query] Option<Name> is extracted from the url query "?name=jason"
// it is optional, as the type suggests. To make it mandatory, simply
// remove the Option type. If there is a query in the handler and
// an incoming request url does not contain the query parameters, it will
// result in an error response
#[handler]
async fn hello_world(#[path] p: Name, #[query] q: Name) -> String {
    format!("{} sends hello to {}", p.name, q.name)
}

#[tokio::test]
async fn main() -> Result<(), Error> {
    // the `app` macro creates a server and allows the user to call
    // the method `run` and await on that future
    app!({
       // the provided address is verified at compile time
        address: "127.0.0.1:3000",
        // via the container we inject our dependencies
        // in this case, MyLogger type
        // any handler that has the trait Logger as an argument
        // will be given MyLogger
        module: make_container => Container,
        // a set of global middleware that will be executed for every handler
        // it will assert that every request has a body size less than 128 bytes
        middleware: [],
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
                handler: do_something_else
            },
        ],
    })
    .run()
    .await
}
