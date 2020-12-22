use darpi_code_gen::{handler, run, QueryType};
use darpi_web::json::Json;
use darpi_web::request::{Path, Query, QueryPayloadError};
use http::Method;
use serde::{Deserialize, Serialize};
use shaku::{module, Component, Interface};
use std::sync::Arc;

trait Logger: Interface {
    fn log(&self, arg: &dyn std::fmt::Debug);
}

#[derive(Component)]
#[shaku(interface = Logger)]
struct MyLogger;
impl Logger for MyLogger {
    fn log(&self, arg: &dyn std::fmt::Debug) {
        println!("{:#?}", arg)
    }
}

module! {
    MyModule {
        components = [MyLogger],
        providers = [],
    }
}

#[derive(Deserialize, Serialize, QueryType)]
pub struct HelloWorldParams {
    hello: String,
}

#[handler]
async fn hello_world(
    q: Query<HelloWorldParams>,
) -> Result<Json<HelloWorldParams>, QueryPayloadError> {
    if q.hello == "john" {
        return Err(QueryPayloadError::NotExist);
    }
    Ok(Json(q.into_inner()))
}

#[handler]
async fn hello_world_optional(q: Option<Query<HelloWorldParams>>) -> String {
    let name = match &q {
        Some(hw) => &hw.hello,
        None => "nobody",
    };
    format!("hello_world {}", name)
}

// #[handler]
// async fn hello_world_path(Path((id, name)): Path<(String, u32)>) -> String {
//     format!("hello_world id: {} name: {}", id, name)
// }

#[derive(Deserialize, Serialize, Debug)]
pub struct HelloWorldBody {
    hello: String,
}

#[handler]
async fn hello_world_json_body(
    body: Json<HelloWorldBody>,
    logger: Arc<dyn Logger>,
) -> Json<HelloWorldBody> {
    logger.log(&body);
    body
}

#[tokio::test]
async fn main() {
    //todo create logging, middleware and web path
    // todo use FromRequest in handler to enable user defined types
    run!({
        address: "127.0.0.1:3000",
        module: MyModule,
        bind: [
            {
                route: "/hello_world/{name}",
                method: Method::GET,
                handler: hello_world
            },
        ],
    });
}
