use crate::{Body, Response};
use async_trait::async_trait;
use futures::future::BoxFuture;
use http::request::Parts as RequestParts;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

#[allow(dead_code)]
pub struct Args<'a, C> {
    pub request_parts: &'a mut RequestParts,
    pub container: Arc<C>,
    pub body: &'a mut Body,
    pub route_args: HashMap<&'a str, &'a str>,
    pub async_job_sender: UnboundedSender<BoxFuture<'static, ()>>,
    pub sync_cpu_job_sender: Sender<fn()>,
    pub sync_io_job_sender: Sender<fn()>,
}

#[async_trait]
pub trait Handler<'a, C>
where
    C: 'static + Sync + Send,
{
    async fn call(&self, args: &mut Args<'a, C>) -> Result<Response<Body>, Infallible>;
}

#[async_trait]
impl<'a, C, F> Handler<'a, C> for F
where
    C: 'static + Sync + Send,
    F: Fn(&mut Args<'a, C>) -> Result<Response<Body>, Infallible> + 'static + Send + Sync,
{
    async fn call(
        &self,
        args: &mut Args<'a, C>,
    ) -> Result<Response<Body>, std::convert::Infallible> {
        (self)(args)
    }
}
