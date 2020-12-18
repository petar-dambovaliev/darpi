extern crate darpi_code_gen;

use darpi_code_gen::{handler, run, QueryType};
use darpi_web::request::{Query, QueryPayloadError};
use darpi_web::response::ErrResponder;
use darpi_web::{Body, Request, Response};
use http::Error;
use http::Method;
use serde::{Deserialize, Serialize};
use shaku::{module, Component, Interface};

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

#[derive(Deserialize, Serialize, QueryType)]
pub struct HelloWorldParams {
    hello: i32,
}

#[handler]
async fn hello_world(q: Query<HelloWorldParams>) -> Result<Response<Body>, Error> {
    Ok(Response::new(Body::from(format!(
        "hello_world {}",
        q.hello
    ))))
}

#[derive(Deserialize)]
pub struct ManualHelloWorldParams {
    hello: i32,
}

impl ErrResponder<QueryPayloadError, Body> for ManualHelloWorldParams {
    fn respond_err(e: QueryPayloadError) -> Response<Body> {
        let msg = match e {
            QueryPayloadError::Deserialize(de) => de.to_string(),
            QueryPayloadError::NotExist => "missing query params".to_string(),
        };

        Response::builder()
            .status(http::StatusCode::BAD_REQUEST)
            .body(darpi_web::Body::from(msg))
            .expect("this not to happen!")
    }
}

#[handler]
async fn hello_world_manual_query(
    q: Query<ManualHelloWorldParams>,
) -> Result<Response<Body>, Error> {
    //todo implement custom result type so users can create errors for a response
    Ok(Response::new(Body::from(format!(
        "hello_world {}",
        q.hello
    ))))
}

#[handler]
async fn hello_world_optional(q: Option<Query<HelloWorldParams>>) -> Result<Response<Body>, Error> {
    let name = match &q {
        Some(s) => s.hello,
        None => 123,
    };
    Ok(Response::new(Body::from(format!("hello_world {}", name))))
}

#[tokio::test]
async fn main() {
    //todo create logging, middleware and web path
    run!({
        address: "127.0.0.1:3000",
        module: MyModule,
        bind: [
            {
                route: "/hello_world",
                method: Method::GET,
                handler: hello_world
            },
            {
                route: "/hello_world_optional",
                method: Method::GET,
                handler: hello_world_optional
            },
            {
                route: "/hello_world_manual",
                method: Method::GET,
                handler: hello_world_manual_query
            },
        ],
    });
}
