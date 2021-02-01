use darpi::request::PayloadError;
use darpi::{
    app, handler, middleware, req_formatter, resp_formatter, Body, Error, Method, Path, Query,
};
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
#[handler(
    container = Container
    req_middleware = [first(1)]
    res_middleware = [second(req_middleware(0))]
)]
async fn hello_world(#[req_middleware(0)] m: u64) -> String {
    format!("{}", m)
}

#[resp_formatter("%a %t %T %s %b")]
#[req_formatter("%a %t %b")]
struct LogFormat;

use tokio::runtime::Runtime;

//RUST_LOG=darpi=info cargo test --test inject -- --nocapture
#[tokio::test]
async fn main() -> Result<(), Error> {
    env_logger::builder().is_test(true).try_init().unwrap();

    let rt = Runtime::new().unwrap();

    app!({
        address: "127.0.0.1:3000",
        container: make_container => Container,
        req_middleware: [log_request(LogFormat)],
        res_middleware: [log_response((LogFormat, req_middleware(0)))],
        bind: [
            {
                // When a path argument is defined in the route,
                // the handler is required to have Path<T> as an argument
                // if not present, it will result in a compilation error
                route: "/hello_world",
                method: Method::GET,
                // handlers bound with a GET method are not allowed
                // to request a body(payload) from the request.
                // Json<T> argument would result in a compilation error
                handler: hello_world
            }
        ],
    })
    .run()
    .await
}
