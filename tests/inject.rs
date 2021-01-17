use darpi::{app, handler, Error, Json, Method, Path, Query};
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

fn make_container() -> Container {
    let module = Container::builder()
        .with_component_parameters::<DateLoggerImpl>(DateLoggerImplParameters {
            today: "Jan 26".to_string(),
            year: 2020,
        })
        .build();
    module
}

module! {
    Container {
        components = [MyLogger, DateLoggerImpl],
        providers = [],
    }
}

#[derive(Deserialize, Serialize, Debug, Path, Query)]
pub struct Name {
    name: String,
}

// Path<Name> is extracted from the registered path "/hello_world/{name}"
// and it is always mandatory. A request without "{name}" will result
// in the request path not matching the handler. It will either match another
// handler or result in an 404
// Option<Query<Name>> is extracted from the url query "?name=jason"
// it is optional, as the type suggests. To make it mandatory, simply
// remove the Option type. If there is a Query<T> in the handler and
// an incoming request url does not contain the query parameters, it will
// result in an error response
#[handler(Container)]
async fn hello_world(
    #[path] p: Name,
    #[query] q: Option<Name>,
    #[inject] logger: Arc<dyn Logger>,
) -> String {
    let other = q.map_or("nobody".to_owned(), |n| n.name);
    let response = format!("{} sends hello to {}", p.name, other);
    logger.log(&response);
    response
}

#[tokio::test]
async fn main() -> Result<(), Error> {
    // the `app` macro creates a server and allows the user to call
    // the method `run` and await on that future
    app!({
       // the provided address is verified at compile time
        address: "127.0.0.1:3000",
        // via the container we inject our dependencies
        // in this case, MyLogger type
        // any handler that has the trait Logger as an argument
        // will be given MyLogger
        module: make_container => Container,
        middleware: [],
        bind: [
            {
                // When a path argument is defined in the route,
                // the handler is required to have Path<T> as an argument
                // if not present, it will result in a compilation error
                route: "/hello_world/{name}",
                method: Method::GET,
                // handlers bound with a GET method are not allowed
                // to request a body(payload) from the request.
                // Json<T> argument would result in a compilation error
                handler: hello_world
            },
        ],
    })
    .run()
    .await
}
