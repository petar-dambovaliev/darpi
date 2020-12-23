use darpi_code_gen::{handler, path_type, run};
use darpi_web::request::Path;
use http::Method;
use serde::{Deserialize, Serialize};
use shaku::{module, Component, Interface};

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

#[path_type]
#[derive(Deserialize, Serialize, Debug)]
pub struct HelloWorldPath {
    name: usize,
}

#[handler]
async fn hello_world(p: Path<HelloWorldPath>) {}

#[tokio::test]
async fn main() {
    //todo create logging, middleware
    // todo use FromRequest in handler to enable user defined types that have custom ser/de
    //todo clean up code generation
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
