use crate::{Body, Response};
use async_trait::async_trait;
use futures::Future;
use http::request::Parts as RequestParts;
use std::pin::Pin;
use std::sync::Arc;

#[async_trait]
pub trait RequestJob<C>
where
    C: 'static + Sync + Send,
{
    type HandlerArgs;
    async fn call(
        p: &mut RequestParts,
        module: Arc<C>,
        b: &mut Body,
        ha: Self::HandlerArgs,
    ) -> ReturnType;
}

#[async_trait]
pub trait ResponseJob<C>
where
    C: 'static + Sync + Send,
{
    type HandlerArgs;
    async fn call(r: &mut Response<Body>, module: Arc<C>, ha: Self::HandlerArgs) -> ReturnType;
}

pub enum ReturnType {
    Future(Pin<Box<dyn Future<Output = ()> + Send>>),
    Fn(fn()),
}
