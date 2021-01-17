use darpi::request::PayloadError;
use darpi::HttpBody;
use darpi::{app, handler, middleware, Body, Method, Path};
use darpi_web::Json;
use serde::{Deserialize, Serialize};
use shaku::module;

#[middleware(Request)]
pub async fn body_size_limit(#[body] b: &Body, #[expect] size: u64) -> Result<(), PayloadError> {
    if let Some(limit) = b.size_hint().upper() {
        if size < limit {
            return Err(PayloadError::Size(size, limit));
        }
    }
    Ok(())
}

#[derive(Deserialize, Serialize, Debug, Path)]
pub struct Name {
    name: String,
}

#[handler([body_size_limit(64)])]
async fn do_something(#[path] p: Name, #[body] payload: Json<Name>) -> String {
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
        middleware: [body_size_limit(128)],
        bind: [
            {
                route: "/hello_world/{name}",
                method: Method::POST,
                // the POST method allows this handler to have
                // Json<Name> as an argument
                handler: do_something
            },
        ],
    })
    .run()
    .await
}
