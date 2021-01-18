use darpi::request::PayloadError;
use darpi::HttpBody;
use darpi::{app, handler, middleware, Body, Method, Path};
use darpi_web::Json;
use serde::{Deserialize, Serialize};
use shaku::module;

#[middleware(Request)]
async fn body_size_limit(#[body] b: &Body, #[handler] size: u64) -> Result<u64, PayloadError> {
    if let Some(limit) = b.size_hint().upper() {
        if size < limit {
            return Err(PayloadError::Size(size, limit));
        }
    }
    Ok(size)
}

#[derive(Deserialize, Serialize, Debug, Path)]
pub struct Name {
    name: String,
}

#[handler([body_size_limit(64)])]
async fn do_something(
    #[path] p: Name,
    #[body] payload: Json<Name>,
    #[middleware(0)] size: u64,
) -> String {
    format!("{} sends hello to {} size {}", p.name, payload.name, size)
}

#[handler]
async fn do_something_else(#[path] p: Name, #[body] payload: Json<Name>) -> String {
    format!("{} sends hello to {}", p.name, payload.name)
}

module! {
    Container {
        components = [],
        providers = [],
    }
}

fn make_container() -> Container {
    let module = Container::builder().build();
    module
}

#[tokio::test]
async fn main() -> Result<(), darpi::Error> {
    let address = format!("127.0.0.1:{}", 3000);
    app!({
        address: address,
        module: make_container => Container,
        // a set of global middleware that will be executed for every handler
        middleware: [body_size_limit(128)],
        bind: [
            {
                route: "/hello_world/{name}",
                method: Method::POST,
                // the POST method allows this handler to have
                // Json<Name> as an argument
                handler: do_something
            },
            {
                route: "/hello_world1/{name}",
                method: Method::POST,
                // the POST method allows this handler to have
                // Json<Name> as an argument
                handler: do_something_else
            },
        ],
    })
    .run()
    .await
}
