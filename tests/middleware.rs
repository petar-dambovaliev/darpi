use darpi::{app, handler, Method, Path};
use darpi_middleware::auth::{
    authorize, Algorithm, Error, JwtAlgorithmProviderImpl, JwtAlgorithmProviderImplParameters,
    JwtSecretProviderImpl, JwtSecretProviderImplParameters, JwtTokenCreator, JwtTokenCreatorImpl,
    Token, TokenExtractorImpl, UserRole,
};
use darpi_middleware::body_size_limit;
use darpi_web::Json;
use serde::{Deserialize, Serialize};
use shaku::module;
use std::fmt;
use std::sync::Arc;

#[derive(Clone, PartialEq, PartialOrd)]
pub enum Role {
    User,
    Admin,
}

impl Role {
    pub fn from_str(role: &str) -> Role {
        match role {
            "Admin" => Role::Admin,
            _ => Role::User,
        }
    }
}

impl UserRole for Role {
    fn is_authorized(&self, other: &str) -> bool {
        let other = Self::from_str(other);
        self < &other
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::User => write!(f, "User"),
            Role::Admin => write!(f, "Admin"),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Login {
    email: String,
    password: String,
}

#[handler(Container)]
async fn login(
    #[body] data: Json<Login>,
    #[inject] jwt_tok_creator: Arc<dyn JwtTokenCreator>,
) -> Result<Token, Error> {
    //verify user data
    let admin = Role::Admin; // hardcoded just for testing
    let uid = "uid"; // hardcoded just for testing
    let tok = jwt_tok_creator.create(uid, &admin).await?;
    Ok(tok)
}

#[derive(Deserialize, Serialize, Debug, Path)]
pub struct Name {
    name: String,
}

#[handler(Container, [authorize(Role::Admin)])]
async fn do_something(#[path] p: Name) -> String {
    format!("hello to {}", p.name)
}

#[handler([body_size_limit(64)])]
async fn do_something_else(#[path] p: Name, #[body] payload: Json<Name>) -> String {
    format!("{} sends hello to {}", p.name, payload.name)
}

module! {
    Container {
        components = [JwtAlgorithmProviderImpl, JwtSecretProviderImpl, TokenExtractorImpl, JwtTokenCreatorImpl],
        providers = [],
    }
}

fn make_container() -> Container {
    let module = Container::builder()
        .with_component_parameters::<JwtSecretProviderImpl>(JwtSecretProviderImplParameters {
            secret: "my secret".to_string(),
        })
        .with_component_parameters::<JwtAlgorithmProviderImpl>(JwtAlgorithmProviderImplParameters {
            algorithm: Algorithm::ES256,
        })
        .build();
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
                route: "/login",
                method: Method::POST,
                // the POST method allows this handler to have
                // Json<Name> as an argument
                handler: login
            },
            {
                route: "/hello_world/{name}",
                method: Method::GET,
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
