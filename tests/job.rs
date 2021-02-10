use darpi::{
    app, handler, job, req_formatter, resp_formatter, Error, Method, Path, Query, RequestJob,
    ResponseJob,
};
use darpi_middleware::{log_request, log_response};
use env_logger;
use futures_util::future::BoxFuture;
use futures_util::FutureExt;
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

#[job(Request)]
async fn first_async_job() -> job::ReturnType {
    job::ReturnType::Future(async { println!("first job in the background.") }.boxed())
}

#[job(Response)]
async fn first_sync_job() -> job::ReturnType {
    println!("first_sync_job start");
    job::ReturnType::Fn(|| {
        let mut r = 0;
        for i in 0..10000000 {
            r += 1;
        }
        let mut r = 0;
        for i in 0..10000000 {
            r += 1;
        }
        let mut r = 0;
        for i in 0..10000000 {
            r += 1;
        }
        println!("first sync job in the background. {}", r)
    })
}

//todo implement the ... operator for middleware slicing
#[handler]
async fn hello_world() -> String {
    format!("{}", 123)
}

#[resp_formatter("%a")]
#[req_formatter("%a")]
struct LogFormatter;

//RUST_LOG=darpi=info cargo test --test inject -- --nocapture
//#[tokio::test]
#[tokio::test]
async fn main() -> Result<(), darpi::Error> {
    env_logger::builder().is_test(true).try_init().unwrap();
    app!({
        address: "127.0.0.1:3000",
        container : {
            factory: make_container,
            type: Container
        },
        jobs: {
            request: [first_async_job],
            response: [first_sync_job]
        },
        middleware: {
            request: [log_request(LogFormatter)],
            response: [log_response(LogFormatter, request(0))]
        },
        handlers: [{
            route: "/hello_world",
            method: Method::GET,
            handler: hello_world
        }]
    })
    .run()
    .await
}
