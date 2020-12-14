extern crate darpi_code_gen;

use darpi_code_gen::{handler, run, ErrorResponder};
use darpi_web::{Body, Query, QueryPayloadError, Request, Response};
use http::Error;
use http::Method;
use serde::Deserialize;
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

#[derive(Deserialize, Default, ErrorResponder)]
pub struct HelloWorldParams {
    hello: String,
}

#[handler]
async fn hello_world(q: Query<HelloWorldParams>) -> Result<Response<Body>, Error> {
    //todo implement custom result type so users can create errors for a response
    Ok(Response::new(Body::from(format!(
        "hello_world {}",
        q.hello.as_str()
    ))))
}

#[handler]
async fn hello_world_optional(q: Option<Query<HelloWorldParams>>) -> Result<Response<Body>, Error> {
    let name = match &q {
        Some(s) => s.hello.as_str(),
        None => "who the hell are you",
    };
    Ok(Response::new(Body::from(format!("hello_world {}", name))))
}

#[tokio::test]
async fn main() {
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
        ],
    });
}
