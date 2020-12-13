use http::status::InvalidStatusCode;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::{convert::Infallible, net::SocketAddr};
extern crate darpi_code_gen;
use std::cell::Cell;

use darpi_code_gen::{handler, run};
use darpi_web::{Query, QueryPayloadError};
use http::{Error, Method, StatusCode};
use hyper::server::conn::AddrStream;
use serde::Deserialize;
use shaku::{module, Component, HasComponent, Interface};
use std::borrow::{Borrow, BorrowMut};
use std::sync::Arc;
trait MyComponent: Interface {}

#[derive(Component)]
#[shaku(interface = MyComponent)]
struct MyComponentImpl;
impl MyComponent for MyComponentImpl {}

trait MyComponentMut: Interface {}

#[derive(Component)]
#[shaku(interface = MyComponentMut)]
struct MyComponentImplMut;
impl MyComponentMut for MyComponentImplMut {}

module! {
    MyModule {
        components = [MyComponentImpl, MyComponentImplMut],
        providers = [],
    }
}

#[tokio::test]
async fn main() {
    #[derive(Deserialize)]
    pub struct HelloWorldParams {
        hello: String,
    }
    #[allow(non_camel_case_types, missing_docs)]
    pub struct hello_world;
    impl hello_world {
        async fn call(
            r: Request<Body>,
            module: std::sync::Arc<MyModule>,
        ) -> Result<Response<Body>, Infallible> {
            let arg_0: Option<Query<HelloWorldParams>> = match r.uri().query() {
                Some(q) => {
                    let q: Result<Query<HelloWorldParams>, QueryPayloadError> =
                        Query::from_query(q);
                    Some(q.unwrap())
                }
                None => None,
            };
            let arg_1: Arc<dyn MyComponent> = module.resolve();
            Self::hello_world(arg_0, arg_1).await
        }
        async fn hello_world(
            q: Option<Query<HelloWorldParams>>,
            _: Arc<dyn MyComponent>,
        ) -> Result<Response<Body>, Infallible> {
            Ok(Response::new(Body::from("hello_world")))
        }
    }

    #[allow(non_camel_case_types, missing_docs)]
    pub enum RoutePossibilities {
        a5cb5bfa3b96f21b6eff4509d381261b7,
    }
    impl RoutePossibilities {
        pub fn is(&self, route: &str, method: &http::Method) -> bool {
            return match self {
                RoutePossibilities::a5cb5bfa3b96f21b6eff4509d381261b7 => {
                    route == "/hello_world" && method == Method::GET.as_str()
                }
            };
        }
    }
    pub struct App {
        module: std::sync::Arc<MyModule>,
        handlers: std::sync::Arc<[RoutePossibilities; 1usize]>,
        address: std::net::SocketAddr,
    }
    impl App {
        pub fn new() -> Self {
            let address: std::net::SocketAddr = "127.0.0.1:3000"
                .parse()
                .expect(&format!("invalid server address: `{}`", "127.0.0.1:3000"));
            let module = std::sync::Arc::new(MyModule::builder().build());
            Self {
                module: module,
                handlers: std::sync::Arc::new([
                    RoutePossibilities::a5cb5bfa3b96f21b6eff4509d381261b7,
                ]),
                address: address,
            }
        }
        pub async fn start(self) {
            let address = self.address;
            let module = self.module.clone();
            let handlers = self.handlers.clone();
            let make_svc = make_service_fn(move |_conn| {
                let inner_module = Arc::clone(&module);
                let inner_handlers = Arc::clone(&handlers);
                async move {
                    Ok::<_, Infallible>(service_fn(move |r: Request<Body>| {
                        let inner_module = Arc::clone(&inner_module);
                        let inner_handlers = Arc::clone(&inner_handlers);
                        async move {
                            let route = r.uri().path();
                            let method = r.method();
                            let handler = inner_handlers
                                .iter()
                                .find(|h| h.is(route, method))
                                .expect(&format!(
                                    "no such handler for route: {} method: {}",
                                    route, method
                                ));
                            match handler {
                                RoutePossibilities::a5cb5bfa3b96f21b6eff4509d381261b7 => {
                                    hello_world::call(r, module).await
                                }
                            }
                        }
                    }))
                }
            });
            let server = Server::bind(&address).serve(make_svc);
            if let Err(e) = server.await {
                eprintln!("server error: {}", e);
            }
        }
    }
    App::new().start().await;
}
