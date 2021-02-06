use darpi::request::PayloadError;
use darpi::{
    app, handler, middleware, req_formatter, resp_formatter, Body, Error, Method, Path, Query,
};
use darpi_middleware::{log_request, log_response};
use env_logger;
use futures::Future;
use futures_util::__private::Pin;
use futures_util::future::{BoxFuture, LocalBoxFuture};
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
use tokio::task;

#[resp_formatter("%a %t %T %s %b")]
#[req_formatter("%a %t %b")]
struct LogFormat;

//RUST_LOG=darpi=info cargo test --test inject -- --nocapture
//#[tokio::test]
fn main() {
    task::spawn(async {
        // perform some work here...
    });
    env_logger::builder().is_test(true).try_init().unwrap();

    // app!({
    //     address: "127.0.0.1:3000",
    //     container: make_container => Container,
    //     req_middleware: [log_request(LogFormat)],
    //     res_middleware: [log_response((LogFormat, req_middleware(0)))],
    //     bind: [
    //         {
    //             // When a path argument is defined in the route,
    //             // the handler is required to have Path<T> as an argument
    //             // if not present, it will result in a compilation error
    //             route: "/hello_world",
    //             method: Method::GET,
    //             // handlers bound with a GET method are not allowed
    //             // to request a body(payload) from the request.
    //             // Json<T> argument would result in a compilation error
    //             handler: hello_world
    //         }
    //     ],
    // })
    // .run()
    // .await
    use std::convert::TryFrom;
    #[allow(non_camel_case_types, missing_docs)]
    pub enum RoutePossibilities {
        a5cb5bfa3b96f21b6eff4509d381261b7,
    }
    impl RoutePossibilities {
        pub fn get_route<'a>(
            &self,
            route: &'a str,
            method: &darpi::Method,
        ) -> Option<(
            darpi::ReqRoute<'a>,
            std::collections::HashMap<&'a str, &'a str>,
        )> {
            return match self {
                RoutePossibilities::a5cb5bfa3b96f21b6eff4509d381261b7 => {
                    let req_route = darpi::ReqRoute::try_from(route).unwrap();
                    let def_route = darpi::Route::try_from("/hello_world").unwrap();
                    if def_route == req_route && method == Method::GET.as_str() {
                        let args = req_route.extract_args(&def_route).unwrap();
                        return Some((req_route, args));
                    }
                    None
                }
            };
        }
    }
    pub struct App {
        module: std::sync::Arc<Container>,
        handlers: std::sync::Arc<[RoutePossibilities; 1usize]>,
        address: std::net::SocketAddr,
    }
    impl App {
        pub fn new(address: &str) -> Self {
            let address: std::net::SocketAddr = address
                .parse()
                .expect(&format!("invalid server address: `{}`", address));
            let module = std::sync::Arc::new(make_container());
            Self {
                module: module,
                handlers: std::sync::Arc::new([
                    RoutePossibilities::a5cb5bfa3b96f21b6eff4509d381261b7,
                ]),
                address: address,
            }
        }
        pub async fn run(self) -> Result<(), darpi::Error> {
            let address = self.address;
            let module = self.module.clone();
            let handlers = self.handlers.clone();

            let (send_sync, mut recv_sync): (
                std::sync::mpsc::Sender<fn()>,
                std::sync::mpsc::Receiver<fn()>,
            ) = std::sync::mpsc::channel();
            let sync_job_executor = task::spawn_blocking(move || loop {
                let job = match recv_sync.recv() {
                    Ok(k) => k,
                    Err(e) => return,
                };
                (job)()
            });

            let (send, mut recv) = tokio::sync::mpsc::unbounded_channel();
            let job_executor = tokio::spawn(async move {
                loop {
                    let job: Option<BoxFuture<()>> = recv.recv().await;
                    if let Some(job) = job {
                        job.await;
                    }
                }
            });

            let make_svc = darpi::service::make_service_fn(move |_conn| {
                let inner_module = std::sync::Arc::clone(&module);
                let inner_handlers = std::sync::Arc::clone(&handlers);
                let inner_send = send.clone();
                async move {
                    Ok::<_, std::convert::Infallible>(darpi::service::service_fn(
                        move |r: darpi::Request<darpi::Body>| {
                            use darpi::futures::FutureExt;
                            use darpi::response::ResponderError;
                            use darpi::Handler;
                            #[allow(unused_imports)]
                            use darpi::RequestMiddleware;
                            #[allow(unused_imports)]
                            use darpi::ResponseMiddleware;
                            let inner_module = std::sync::Arc::clone(&inner_module);
                            let inner_handlers = std::sync::Arc::clone(&inner_handlers);
                            let inner_send = inner_send.clone();
                            async move {
                                let route = r.uri().path().to_string();
                                let method = r.method().clone();
                                let (mut parts, mut body) = r.into_parts();
                                let m_arg_0 = match log_request::call(
                                    &mut parts,
                                    inner_module.clone(),
                                    &mut body,
                                    LogFormat,
                                )
                                .await
                                {
                                    Ok(k) => k,
                                    Err(e) => return Ok(e.respond_err()),
                                };
                                let mut handler = None;
                                for rp in inner_handlers.iter() {
                                    if let Some(rr) = rp.get_route(&route, &method) {
                                        handler = Some((rp, rr));
                                        break;
                                    }
                                }
                                let handler = match handler {
                                    Some(s) => s,
                                    None => {
                                        return async {
                                            Ok::<_, std::convert::Infallible>(
                                                darpi::Response::builder()
                                                    .status(darpi::StatusCode::NOT_FOUND)
                                                    .body(darpi::Body::empty())
                                                    .unwrap(),
                                            )
                                        }
                                        .await
                                    }
                                };
                                let mut rb = match handler.0 {
                                    RoutePossibilities::a5cb5bfa3b96f21b6eff4509d381261b7 => {
                                        let mut args = darpi::Args {
                                            request_parts: &mut parts,
                                            container: inner_module.clone(),
                                            body: &mut body,
                                            route_args: handler.1 .1,
                                        };
                                        Handler::call(&hello_world, &mut args).await
                                    }
                                };

                                if let Ok(mut rb) = rb.as_mut() {
                                    let b = log_response::call(
                                        &mut rb,
                                        inner_module.clone(),
                                        (LogFormat, m_arg_0.clone()),
                                    );
                                    inner_send.send(async {}.boxed());
                                }
                                rb
                            }
                        },
                    ))
                }
            });
            let server = darpi::Server::bind(&address).serve(make_svc);

            async {
                let _ = tokio::join!(job_executor, sync_job_executor, server);
            }
            .await;
            Ok(())
        }
    }
}
