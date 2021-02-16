use darpi::{app, handler, job::Job, job_factory, logger::DefaultFormat, Method, Path, Query};
use darpi_middleware::{log_request, log_response};
use env_logger;
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

#[job_factory(Request)]
async fn first_async_job() -> Job {
    Job::Future(async { println!("first job in the background.") }.boxed())
}

#[job_factory(Response)]
async fn first_sync_job() -> Job {
    Job::CpuBound(|| println!("first_sync_job in the background"))
}

#[job_factory(Response)]
async fn first_sync_job1() -> Job {
    Job::CpuBound(|| {
        let mut r = 0;
        for _ in 0..10000000 {
            r += 1;
        }
        println!("first_sync_job1 finished in the background. {}", r)
    })
}

#[job_factory(Response)]
async fn first_sync_io_job() -> Job {
    Job::IOBlocking(|| {
        std::thread::sleep(std::time::Duration::from_secs(2));
        println!("sync io finished in the background");
    })
}

#[handler({
    jobs: {
        request: [],
        response: [first_sync_job1]
    }
})]
async fn hello_world() -> String {
    format!("{}", 123)
}

//RUST_LOG=darpi=info cargo test --test job -- --nocapture
//#[tokio::test]
#[tokio::test]
async fn main() -> Result<(), darpi::Error> {
    env_logger::builder().is_test(true).try_init().unwrap();

    app!({
        address: "127.0.0.1:3000",
        container: {
            factory: make_container(),
            type: Container
        },
        jobs: {
            request: [],
            response: [first_sync_io_job]
        },
        middleware: {
            request: [log_request(DefaultFormat)],
            response: [log_response(DefaultFormat, request(0))]
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
