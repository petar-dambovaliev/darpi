use darpi::Error;
use darpi_code_gen::{app, handler, path_type};
use darpi_web::request::Path;
use http::Method;
use serde::{Deserialize, Serialize};
use shaku::{module, Component, Interface};
use std::sync::Arc;

trait Logger: Interface {
    fn log(&self, arg: &dyn std::fmt::Debug);
}

trait UserService: Interface {
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

#[derive(Component)]
#[shaku(interface = UserService)]
struct UserImpl;
impl UserService for UserImpl {
    fn log(&self, arg: &dyn std::fmt::Debug) {
        println!("{:#?}", arg)
    }
}

module! {
    MyModule {
        components = [MyLogger, UserImpl],
        providers = [],
    }
}

#[path_type]
#[derive(Deserialize, Serialize, Debug)]
pub struct HelloWorldPath {
    name: usize,
}

#[handler]
async fn hello_world(
    p: Path<HelloWorldPath>,
    logger: Arc<dyn Logger>,
    user_service: Arc<dyn UserService>,
) -> String {
    let response = format!("hello_world: user {}", p.name);
    logger.log(&response);
    response
}

#[tokio::test]
async fn main() {
    //todo create logging, middleware
    // todo use FromRequest in handler to enable user defined types that have custom ser/de
    //todo clean up code generation
    app!({
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
