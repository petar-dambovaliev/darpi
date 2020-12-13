mod service {
    use shaku::{Component, Interface};
    use std::sync::Arc;

    pub trait Logger: Interface {
        fn log(&self, content: &str);
    }

    pub trait DateLogger: Interface {
        fn log_date(&self);
    }

    #[derive(Component, Debug)]
    #[shaku(interface = Logger)]
    pub struct LoggerImpl;

    impl Logger for LoggerImpl {
        fn log(&self, content: &str) {
            println!("{}", content);
        }
    }

    #[derive(Component)]
    #[shaku(interface = DateLogger)]
    pub struct DateLoggerImpl {
        #[shaku(inject)]
        logger: Arc<dyn Logger>,
    }

    impl DateLogger for DateLoggerImpl {
        fn log_date(&self) {
            self.logger
                .log(&format!("Now it's {:#?}", std::time::SystemTime::now()));
        }
    }
}

mod controller {
    use super::service::DateLogger;
    use super::Handler;
    use http::Method;
    use shaku::HasComponent;
    use std::sync::Arc;

    //this is what the macro will transform the user defined handler to
    // #[get("/")
    // async fn HelloWorldHandler(logger: Arc<dyn DateLogger>) {
    //      logger.log_date()
    // }
    pub struct HelloWorldHandler;

    impl HelloWorldHandler {
        fn call(&self, mut module: Arc<super::MyModule>) {
            let date_logger: Arc<dyn DateLogger> = module.resolve();
            Self::handle(date_logger)
        }
        //user defined function
        fn handle(logger: Arc<dyn DateLogger>) {
            logger.log_date()
        }
    }

    // #[get("/")
    // async fn HelloWorldHandler(logger: Arc<dyn DateLogger>) {
    //      logger.log_date()
    // }
}

use http::Method;
use shaku::module;
use std::sync::Arc;

module! {
    pub MyModule {
        components = [service::LoggerImpl, service::DateLoggerImpl],
        providers = []
    }
}

pub struct App {
    module: Arc<MyModule>,
    handlers: [RoutePossibilities; 1],
}

pub enum RoutePossibilities {
    HelloWorldGet(controller::HelloWorldHandler),
}

impl RoutePossibilities {
    pub fn is(&self, route: &str, method: &Method) -> bool {
        return match self {
            RoutePossibilities::HelloWorldGet(_) => {
                route == "/hello_world" && method == Method::GET
            }
        };
    }
}

impl App {
    pub fn new() -> Self {
        let module = MyModule::builder().build();
        Self {
            module: Arc::new(module),
            //todo here generate all handlers by the user
            handlers: [RoutePossibilities::HelloWorldGet(
                controller::HelloWorldHandler {},
            )],
        }
    }

    pub fn start(&self) {
        println!("started");
        let routes = [
            ("/hello_world", Method::GET),
            ("/hello_world", Method::POST),
        ];
        routes.iter().for_each(|r| self.incoming(r.0, &r.1));
    }

    pub fn incoming(&self, route: &str, method: &Method) {
        let handler = self
            .handlers
            .iter()
            .find(|h| h.is(route, method))
            .expect(&format!(
                "no such handler for route: {} method: {}",
                route, method
            ));

        match handler {
            RoutePossibilities::HelloWorldGet(hw) => hw.call(self.module.clone()),
        }
    }
}

#[test]
fn main() {
    App::new().start();
}
