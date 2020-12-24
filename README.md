### darpi

A web api framework will speed and safety in mind.
One of the big goals is to catch all errors at `compile time`, if possible.
The framework uses [hyper](https://github.com/hyperium/hyper) and ideally, the performance should be as if you were using `hyper` yourself.
The framework also uses [shaku](https://github.com/Mcat12/shaku) for `compile time` verifiable dependency injection.

The framework is in early development.
All feedback is appreciated.

An example of a simple app

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
 use darpi::{handler, path_type, query_type, run, Json, Method, Path, Query};
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
     Container {
         components = [MyLogger],
         providers = [],
     }
 }
 
 #[path_type]
 #[query_type]
 #[derive(Deserialize, Serialize, Debug)]
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
 #[handler]
 async fn hello_world(p: Path<Name>, q: Option<Query<Name>>, logger: Arc<dyn Logger>) -> String {
     let other = q.map_or("nobody".to_owned(), |n| n.0.name);
     let response = format!("{} sends hello to {}", p.name, other);
     logger.log(&response);
     response
 }
 
 // Json<Name> is extracted from the request body
 // failure to do so will result in an error response
 #[handler]
 async fn do_something(p: Path<Name>, payload: Json<Name>, logger: Arc<dyn Logger>) -> String {
     let response = format!("{} sends hello to {}", p.name, payload.name);
     logger.log(&response);
     response
 }
 
 #[tokio::main]
 async fn main() {
    // the `run` macro creates and runs the server
     run!({
        // the provided address is verified at compile time
         address: "127.0.0.1:3000",
         // via the container we inject our dependencies
         // in this case, MyLogger type
         // any handler that has the trait Logger as an argument
         // will be given MyLogger
         module: Container,
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
                 // the POST method allows this handler to have
                 // Json<Name> as an argument
                 handler: do_something
             },
         ],
     });
 }
```
