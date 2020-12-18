use darpi_code_gen::{handler, run, QueryType};
use darpi_web::json::Json;
use darpi_web::request::{Query, QueryPayloadError};
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
    hello: String,
}

#[handler]
async fn hello_world(
    q: Query<HelloWorldParams>,
) -> Result<Json<HelloWorldParams>, QueryPayloadError> {
    if q.hello == "petar" {
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

#[derive(Deserialize, Serialize)]
pub struct HelloWorldBody {
    hello: String,
}

#[handler]
async fn hello_world_json_body(body: Json<HelloWorldBody>) {}

#[tokio::test]
async fn main() {
    //todo create logging, middleware and web path
    //todo add handler for missing routes
    // todo use FromRequest in handler to enable user defined types
    //todo make sure GET handler cannot have json in the args
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
                route: "/hello_world_json_body",
                method: Method::POST,
                handler: hello_world_json_body
            },
        ],
    });
}
