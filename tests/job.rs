use darpi::job::{CpuJob, FutureJob, IOBlockingJob, Job, JobExt, SenderExt};
use darpi::{
    app, from_path, handler, job_factory, logger::DefaultFormat, middleware, Body, Json, Method,
    Query, RequestParts, Response,
};
use darpi_middleware::{log_request, log_response};
use env_logger;
use serde::{Deserialize, Serialize};
use shaku::module;
use std::convert::Infallible;
use std::sync::mpsc::Sender;
use tokio::sync::mpsc::UnboundedSender;

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

#[from_path]
#[derive(Deserialize, Serialize, Debug, Query)]
pub struct Name {
    name: String,
}

#[job_factory(Request)]
async fn first_async_job() -> FutureJob {
    async { println!("first job in the background.") }.into()
}

#[job_factory(Response)]
async fn first_sync_job(#[response] r: &Response<Body>) -> Job {
    let status_code = r.status();
    {
        move || {
            std::thread::sleep(std::time::Duration::from_secs(2));
            println!(
                "first_sync_job in the background for a request with status {}",
                status_code
            );
        }
    }
    .io_blocking()
}

#[job_factory(Response)]
async fn first_sync_job1() -> CpuJob {
    {
        || {
            let mut r = 0;
            for _ in 0..10000000 {
                r += 1;
            }
            println!("first_sync_job1 finished in the background. {}", r)
        }
    }
    .into()
}

#[job_factory(Response)]
async fn first_sync_io_job() -> IOBlockingJob {
    {
        || {
            std::thread::sleep(std::time::Duration::from_secs(2));
            println!("sync io finished in the background");
        }
    }
    .into()
}

#[handler]
async fn hello_world(
    #[request_parts] rp: &RequestParts,
    #[cpu] cpu_job_queue: Sender<CpuJob>,
    #[future] _fut_job_queue: UnboundedSender<FutureJob>,
    #[blocking] _block_job_queue: Sender<IOBlockingJob>,
) -> &'static str {
    if rp.headers.get("destroy-cpu-header").is_some() {
        let cpu_job: CpuJob = {
            || {
                let mut r = 0;
                for _ in 0..10000000 {
                    r += 1;
                }
                println!("first_sync_job1 finished in the background. {}", r)
            }
        }
        .into();

        cpu_job_queue.send(cpu_job).expect("ohh noes!");
    }

    "hello world"
}

#[handler]
async fn hello_world1(#[blocking] job_queue: Sender<IOBlockingJob>) -> String {
    let secs = job_queue
        .oneshoot(move || {
            let secs = 2;
            std::thread::sleep(std::time::Duration::from_secs(secs));
            secs
        })
        .await
        .expect("ohh noes")
        .await
        .expect("ohh noes");

    format!("waited {} seconds to say hello world", secs)
}

#[middleware(Request)]
pub(crate) async fn roundtrip(
    #[request_parts] _rp: &RequestParts,
    #[body] _b: &Body,
    #[handler] msg: impl AsRef<str> + Send + Sync + 'static,
) -> Result<String, Infallible> {
    let res = format!("{} from roundtrip middleware", msg.as_ref());
    Ok(res)
}

#[handler({
    middleware: {
        request: [roundtrip("blah")]
    }
})]
async fn do_something123(
    // the request query is deserialized into Name
    // if deseriliazation fails, it will result in an error response
    // to make it optional wrap it in an Option<Name>
    #[query] query: Name,
    // the request path is deserialized into Name
    #[path] path: Name,
    // the request body is deserialized into the struct Name
    // it is important to mention that the wrapper around Name
    // should implement darpi::request::FromRequestBody
    // Common formats like Json, Xml and Yaml are supported out
    // of the box but users can implement their own
    #[body] payload: Json<Name>,
    // we can access the T from Ok(T) in the middleware result
    #[middleware::request(0)] m_str: String, // returning a String works because darpi has implemented
                                             // the Responder trait for common types
) -> String {
    format!(
        "query: {} path: {} body: {} middleware: {}",
        query.name, path.name, payload.name, m_str
    )
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
