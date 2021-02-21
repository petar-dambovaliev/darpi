use crate::{Body, Response};
use async_trait::async_trait;
use futures::Future;
use http::request::Parts as RequestParts;
use std::pin::Pin;
use std::sync::Arc;

#[async_trait]
pub trait RequestJobFactory<C>
where
    C: 'static + Sync + Send,
{
    type HandlerArgs;
    async fn call(p: &RequestParts, module: Arc<C>, b: &Body, ha: Self::HandlerArgs) -> Job;
}

#[async_trait]
pub trait ResponseJobFactory<C>
where
    C: 'static + Sync + Send,
{
    type HandlerArgs;
    async fn call(r: &Response<Body>, module: Arc<C>, ha: Self::HandlerArgs) -> Job;
}

pub enum Job {
    Future(Pin<Box<dyn Future<Output = ()> + Send>>),
    CpuBound(Box<dyn Fn() + Send>),
    IOBlocking(Box<dyn Fn() + Send>),
}
