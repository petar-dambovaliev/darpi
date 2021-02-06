use crate::{response::ResponderError, Body, Response};
use async_trait::async_trait;
use futures_util::future::LocalBoxFuture;
use http::request::Parts as RequestParts;
use std::convert::Infallible;
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
        module: Arc<M>,
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
        module: Arc<M>,
        ha: Self::HandlerArgs,
    ) -> Result<Self::Type, Self::Error>;
}

#[async_trait]
pub trait RequestJob<M>
where
    M: 'static + Sync + Send,
{
    type HandlerArgs;
    type Error: ResponderError;
    async fn call(
        r: &mut Response<Body>,
        module: Arc<M>,
        ha: Self::HandlerArgs,
    ) -> Result<LocalBoxFuture<()>, Self::Error>;
}

use futures_util::FutureExt;
use hyper::body::HttpBody;

struct Rj;

#[async_trait]
impl<M> RequestJob<M> for Rj
where
    M: 'static + Sync + Send,
{
    type HandlerArgs = ();
    type Error = Infallible;

    async fn call(
        r: &mut Response<Body>,
        _: Arc<M>,
        _: Self::HandlerArgs,
    ) -> Result<LocalBoxFuture<'_, ()>, Self::Error> {
        let size = r.size_hint().exact().unwrap_or(r.size_hint().lower());

        Ok(async move { log::info!("log size: {}", size) }.boxed_local())
    }
}

#[async_trait]
pub trait ResponseJob<M>
where
    M: 'static + Sync + Send,
{
    type HandlerArgs;
    type Error: ResponderError;
    async fn call(
        r: &mut Response<Body>,
        module: Arc<M>,
        ha: Self::HandlerArgs,
    ) -> Result<LocalBoxFuture<()>, Self::Error>;
}
