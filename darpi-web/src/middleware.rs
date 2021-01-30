use crate::{response::ResponderError, Body, Response};
use async_trait::async_trait;
use http::request::Parts as RequestParts;
use std::sync::Arc;

#[async_trait]
pub trait RequestMiddleware<M>
where
    M: 'static + Sync + Send,
{
    type HandlerArgs: 'static + Sync + Send;
    type Error: ResponderError;
    type Type;

    async fn call(
        p: &mut RequestParts,
        module: std::sync::Arc<M>,
        b: &mut Body,
        ha: Self::HandlerArgs,
    ) -> Result<Self::Type, Self::Error>;
}

#[async_trait]
pub trait ResponseMiddleware<M>
where
    M: 'static + Sync + Send,
{
    type HandlerArgs;
    type Error: ResponderError;
    type Type;
    async fn call(
        r: &mut Response<Body>,
        module: std::sync::Arc<M>,
        ha: Self::HandlerArgs,
    ) -> Result<Self::Type, Self::Error>;
}
