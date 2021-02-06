use crate::{Body, Response};
use async_trait::async_trait;
use futures::Future;
use http::request::Parts as RequestParts;
use std::sync::Arc;

#[async_trait]
pub trait RequestJob<C, T>
where
    C: 'static + Sync + Send,
    T: Future,
{
    type HandlerArgs;
    type Error;
    type Future: Future;
    async fn call(
        p: &mut RequestParts,
        module: Arc<C>,
        b: &mut Body,
        ha: Self::HandlerArgs,
    ) -> Type<T>;
}

#[async_trait]
pub trait ResponseJob<C, T>
where
    C: 'static + Sync + Send,
    T: Future,
{
    type HandlerArgs;
    type Error;
    async fn call(r: &mut Response<Body>, module: Arc<C>, ha: Self::HandlerArgs) -> Type<T>;
}

pub enum Type<T>
where
    T: Future,
{
    Future(T),
    Fn(fn()),
}
