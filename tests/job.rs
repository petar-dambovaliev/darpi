use darpi::{app, handler, job, Error, Method, Path, Query, RequestJob, ResponseJob};
use env_logger;
use futures_util::future::{BoxFuture, LocalBoxFuture};
use futures_util::FutureExt;
use serde::{Deserialize, Serialize};
use shaku::module;
use std::convert::Infallible;

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

#[job(Request)]
async fn first_job() -> LocalBoxFuture<'static, ()> {
    Box::pin(async { println!("first job in the background.") }.boxed_local())
}

//todo implement the ... operator for middleware slicing
#[handler]
async fn hello_world() -> String {
    format!("{}", 123)
}

//RUST_LOG=darpi=info cargo test --test inject -- --nocapture
//#[tokio::test]
#[tokio::main]
async fn main() -> Result<(), darpi::Error> {
    env_logger::builder().is_test(true).try_init().unwrap();

    app!({
        "address": "127.0.0.1:3000",
        "container": {
            "factory": "make_container",
            "type": "Container"
        },
        "jobs": {
            "request": [],
            "response": []
        },
        "middleware": {
            "request": [],
            "response": []
        },
        "handlers": [{
            "route": "/hello_world",
            "method": "Method::GET",
            "handler": "hello_world"
        }]
    })
    .run()
    .await
}
