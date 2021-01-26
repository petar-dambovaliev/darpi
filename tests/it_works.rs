use darpi::{app, handler, Error, Method};
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

#[handler]
async fn hello_world() -> String {
    format!("hello world")
}

async fn asd<T>(s: T) -> String {
    format!("hello world")
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    app!({
        address: "127.0.0.1:3000",
        module: make_container => Container,
        middleware: [],
        bind: [
            {
                route: "/hello_world",
                method: Method::GET,
                handler: hello_world
            },
        ],
    })
    .run()
    .await
}
