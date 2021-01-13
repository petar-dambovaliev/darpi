use darpi::{
    app, handler, middleware, middleware::Expect, path_type, query_type, request::ExtractBody,
    Body, Method, Path,
};
use darpi_middleware::body_size_limit;
use darpi_web::Json;
use serde::{Deserialize, Serialize};
use shaku::module;
use std::convert::Infallible;

#[middleware(Request)]
pub async fn hello_middleware(b: &Body, handler: Expect<&str>) -> Result<(), Infallible> {
    println!("hello middleware from `{}`", handler.into_inner());
    Ok(())
}

#[path_type]
#[query_type]
#[derive(Deserialize, Serialize, Debug)]
pub struct Name {
    name: String,
}

// the handler macro has 2 optional arguments
// the shaku container type and a collection of middlewares
// the enum variant `Admin` is corresponding to the middlewre `access_control`'s Expect<UserRole>
// Json<Name> is extracted from the request body
// failure to do so will result in an error response
// here we pass in the string "John Doe" to the `hello_middleware`
// it is being passed on matching the `handler: Expect<&str>` argument
// then we use the builtin body_size_limit middleware and we limit the body size
// to 64 bytes
#[handler([hello_middleware("John Doe"), body_size_limit(64)])]
async fn do_something(p: Path<Name>, payload: ExtractBody<Json<Name>>) -> String {
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

#[tokio::main]
async fn main() -> Result<(), darpi::Error> {
    let address = format!("127.0.0.1:{}", 3000);
    app!({
        address: address,
        module: make_container => Container,
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
