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
        components = [MyLogger, UserImpl, DateLoggerImpl],
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

trait DateLogger: Interface {
    fn log_date(&self);
}

#[derive(Component)]
#[shaku(interface = DateLogger)]
struct DateLoggerImpl {
    #[shaku(inject)]
    logger: Arc<dyn Logger>,
    today: String,
    year: usize,
}

impl DateLogger for DateLoggerImpl {
    fn log_date(&self) {
        self.logger
            .log(&format!("Today is {}, {}", self.today, self.year));
    }
}

fn make_container() -> MyModule {
    let module = MyModule::builder()
        .with_component_parameters::<DateLoggerImpl>(DateLoggerImplParameters {
            today: "Jan 26".to_string(),
            year: 2020,
        })
        .build();
    module
}

#[tokio::test]
async fn main() {
    //todo create logging, middleware
    // todo use FromRequest in handler to enable user defined types that have custom ser/de
    app!({
        address: "127.0.0.1:3000",
        module: make_container => MyModule,
        bind: [
            {
                route: "/hello_world/{name}",
                method: Method::GET,
                handler: hello_world
            },
        ],
    });
}
