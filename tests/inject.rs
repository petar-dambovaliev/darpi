use darpi::request::PayloadError;
use darpi::{app, handler, logger::DefaulFormat, middleware, Body, Error, Method, Path, Query};
use darpi_middleware::{log_request, log_response};
use env_logger;
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

//todo implement the ... operator for middleware slicing
// #[handler(
//     container = Container,
//     request = [first(1)],
//     response = [second(request(0))]
// )]
// async fn do_something_else(#[middleware(1)] m: u64) -> String {
//     "do something".to_string()
// }

#[handler]
async fn hello_world(#[path] p: Name, #[query] q: Name) -> String {
    format!("{} sends hello to {}", p.name, q.name)
}

#[tokio::test]
async fn main() -> Result<(), Error> {
    env_logger::builder().is_test(true).try_init().unwrap();

    app!({
        address: "127.0.0.1:3000",
        module: make_container => Container,
        middleware: [log_request(DefaulFormat), log_response(DefaulFormat, middleware(0))],
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
