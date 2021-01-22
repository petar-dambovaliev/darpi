use async_trait::async_trait;
use chrono::Utc;
use darpi::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    middleware,
    response::ResponderError,
    Body, RequestParts,
};
use derive_more::Display;
pub use jsonwebtoken::*;
use serde::{Deserialize, Serialize};
use shaku::{Component, Interface};
use std::sync::Arc;

pub type Token = String;

#[middleware(Request)]
pub async fn authorize(
    #[handler] role: impl UserRole,
    #[request_parts] rp: &RequestParts,
    #[inject] algo_provider: Arc<dyn JwtAlgorithmProvider>,
    #[inject] token_ext: Arc<dyn TokenExtractor>,
    #[inject] secret_provider: Arc<dyn JwtSecretProvider>,
) -> Result<Token, Error> {
    let token_res = token_ext.extract(&rp).await;
    match token_res {
        Ok(jwt) => {
            let decoded = decode::<Claims>(
                &jwt,
                &DecodingKey::from_secret(secret_provider.secret().await.as_ref()),
                &Validation::new(algo_provider.algorithm().await),
            )
            .map_err(|_| Error::JWTTokenError)?;

            if !role.is_authorized(&decoded.claims.role) {
                return Err(Error::NoPermissionError);
            }

            Ok(decoded.claims.sub)
        }
        Err(e) => return Err(e),
    }
}

pub trait UserRole: ToString + 'static + Sync + Send {
    fn is_authorized(&self, other: &str) -> bool;
}

#[derive(Component)]
#[shaku(interface = TokenExtractor)]
pub struct TokenExtractorImpl;

#[async_trait]
impl TokenExtractor for TokenExtractorImpl {
    async fn extract(&self, rp: &RequestParts) -> Result<Token, Error> {
        jwt_from_header(&rp.headers)
    }
}

#[async_trait]
pub trait TokenExtractor: Interface {
    async fn extract(&self, p: &RequestParts) -> Result<Token, Error>;
}

#[derive(Component)]
#[shaku(interface = JwtSecretProvider)]
pub struct JwtSecretProviderImpl {
    secret: String,
}

#[async_trait]
impl JwtSecretProvider for JwtSecretProviderImpl {
    async fn secret(&self) -> &str {
        &self.secret
    }
}

#[async_trait]
pub trait JwtSecretProvider: Interface {
    async fn secret(&self) -> &str;
}

#[derive(Component)]
#[shaku(interface = JwtAlgorithmProvider)]
pub struct JwtAlgorithmProviderImpl {
    algorithm: Algorithm,
}

#[async_trait]
impl JwtAlgorithmProvider for JwtAlgorithmProviderImpl {
    async fn algorithm(&self) -> Algorithm {
        self.algorithm
    }
}

#[async_trait]
pub trait JwtAlgorithmProvider: Interface {
    async fn algorithm(&self) -> Algorithm;
}

#[derive(Debug, Deserialize, Serialize)]
struct Claims {
    sub: String,
    role: String,
    exp: usize,
}

#[derive(Component)]
#[shaku(interface = JwtTokenCreator)]
pub struct JwtTokenCreatorImpl {
    #[shaku(inject)]
    secret_provider: Arc<dyn JwtSecretProvider>,
    #[shaku(inject)]
    algo_provider: Arc<dyn JwtAlgorithmProvider>,
}

#[async_trait]
impl JwtTokenCreator for JwtTokenCreatorImpl {
    async fn create(&self, uid: &str, role: &dyn UserRole) -> Result<Token, Error> {
        let expiration = Utc::now()
            .checked_add_signed(chrono::Duration::seconds(60))
            .expect("valid timestamp")
            .timestamp();

        let claims = Claims {
            sub: uid.to_owned(),
            role: role.to_string(),
            exp: expiration as usize,
        };
        let header = Header::new(self.algo_provider.algorithm().await);
        encode(
            &header,
            &claims,
            &EncodingKey::from_secret(self.secret_provider.secret().await.as_ref()),
        )
        .map_err(|_| Error::JWTTokenCreationError)
    }
}

#[async_trait]
pub trait JwtTokenCreator: Interface {
    async fn create(&self, uid: &str, role: &dyn UserRole) -> Result<Token, Error>;
}

const BEARER: &str = "Bearer ";

fn jwt_from_header(headers: &HeaderMap<HeaderValue>) -> Result<String, Error> {
    let header = match headers.get(AUTHORIZATION) {
        Some(v) => v,
        None => return Err(Error::NoAuthHeaderError),
    };
    let auth_header = match std::str::from_utf8(header.as_bytes()) {
        Ok(v) => v,
        Err(_) => return Err(Error::NoAuthHeaderError),
    };
    if !auth_header.starts_with(BEARER) {
        return Err(Error::InvalidAuthHeaderError);
    }
    Ok(auth_header.trim_start_matches(BEARER).to_owned())
}

#[derive(Display, Debug)]
pub enum Error {
    #[display(fmt = "wrong credentials")]
    WrongCredentialsError,
    #[display(fmt = "jwt token not valid")]
    JWTTokenError,
    #[display(fmt = "jwt token creation error")]
    JWTTokenCreationError,
    #[display(fmt = "no auth header")]
    NoAuthHeaderError,
    #[display(fmt = "invalid auth header")]
    InvalidAuthHeaderError,
    #[display(fmt = "no permission")]
    NoPermissionError,
}

impl ResponderError for Error {}
