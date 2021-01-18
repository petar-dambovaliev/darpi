### darpi

A web api framework with speed and safety in mind.
One of the big goals is to catch all errors at `compile time`, if possible.
The framework uses [hyper](https://github.com/hyperium/hyper) and ideally, the performance should be as if you were using `hyper` yourself.
The framework also uses [shaku](https://github.com/Mcat12/shaku) for `compile time` verifiable dependency injection.

### The framework is in early development and will only be updated on github, until it's stable.
All feedback is appreciated.

A simple example

`Cargo.toml`
```
[dependencies]
darpi = {git = "https://github.com/petar-dambovaliev/darpi.git", branch = "master"}
serde = { version = "1.0", features = ["derive"] }
tokio = {version = "0.2.11", features = ["full"]}
shaku = {version = "0.5.0", features = ["thread_safe"]}
```
`main.rs`
```rust
use darpi::request::PayloadError;
use darpi::{app, handler, middleware, Body, Error, HttpBody, Json, Method, Path, Query};
use serde::{Deserialize, Serialize};
use shaku::module;

fn make_container() -> Container {
    let module = Container::builder().build();
    module
}

module! {
    Container {
        components = [],
        providers = [],
    }
}

#[derive(Deserialize, Serialize, Debug, Path, Query)]
pub struct Name {
    name: String,
}

// The Request argument to the macro tells it this middleware is to be executed for a Request
// as oppose to a Response. It is allowed to receive a shared reference of the request body and
// it is gotten via the #[body] marker.
// The #[handler] marker is set to values that are provided by the user himself.
// we can see the u64 being provided at the places where the middleware is used
#[middleware(Request)]
async fn body_size_limit(#[body] b: &Body, #[handler] size: u64) -> Result<u64, PayloadError> {
    if let Some(limit) = b.size_hint().upper() {
        if size < limit {
            return Err(PayloadError::Size(size, limit));
        }
    }
    Ok(size)
}

// #[path] tells the handler macro that it should decode the path arguments "/hello_world/{name}" into Name
// and it is always mandatory. A request without "{name}" will result
// in the request path not matching the handler. It will either match another
// handler or result in an 404
// #[query] Option<Name> is extracted from the url query "?name=jason"
// it is optional, as the type suggests. To make it mandatory, simply
// remove the Option type. If there is a query in the handler and
// an incoming request url does not contain the query parameters, it will
// result in an error response
#[handler]
async fn hello_world(#[path] p: Name, #[query] q: Name) -> String {
    format!("{} sends hello to {}", p.name, q.name)
}

// the body_size_limit(64) middleware with the value of 64 is passed to the middleware and mapped
// to #[handler] size: u64. ie handler -> middleware communication
// #[body] tells the handler macro that it should decode the request body as json in the struct Name
// the handler is guarded by the body_size_limit middleware.
// it will assert that every request for this handler has body size less than 64 bytes
// #[middleware(0)] is a way to receive the Ok(u64) value from the body_size_limit middleware's result
// ie middleware -> handler communication
#[handler([body_size_limit(64)])]
async fn do_something_else(
    #[path] p: Name,
    #[body] payload: Json<Name>,
    #[middleware(0)] size: u64,
) -> String {
    format!("{} sends hello to {} size {}", p.name, payload.name, size)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // the `app` macro creates a server and allows the user to call
    // the method `run` and await on that future
    app!({
       // the provided address is verified at compile time
        address: "127.0.0.1:3000",
        // via the container we inject our dependencies
        module: make_container => Container,
        // a set of global middleware that will be executed for every handler
        // it will assert that every request has a body size less than 128 bytes
        middleware: [body_size_limit(128)],
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
            {
                route: "/hello_world/{name}",
                method: Method::POST,
                handler: do_something_else
            },
        ],
    })
        .run()
        .await
}

```
