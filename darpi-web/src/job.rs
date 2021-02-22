use crate::{Body, Response};
use async_trait::async_trait;
use futures::Future;
use futures_util::FutureExt;
use http::request::Parts as RequestParts;
use std::pin::Pin;
use std::sync::Arc;

#[async_trait]
pub trait RequestJobFactory<C>
where
    C: 'static + Sync + Send,
{
    type HandlerArgs;
    type Return: Into<Job>;

    async fn call(
        p: &RequestParts,
        module: Arc<C>,
        b: &Body,
        ha: Self::HandlerArgs,
    ) -> Self::Return;
}

#[async_trait]
pub trait ResponseJobFactory<C>
where
    C: 'static + Sync + Send,
{
    type HandlerArgs;
    type Return: Into<Job>;

    async fn call(r: &Response<Body>, module: Arc<C>, ha: Self::HandlerArgs) -> Self::Return;
}

pub enum Job {
    Future(FutureJob),
    CpuBound(CpuJob),
    IOBlocking(IOBlockingJob),
}

impl From<FutureJob> for Job {
    fn from(fut: FutureJob) -> Self {
        Self::Future(fut)
    }
}
impl From<CpuJob> for Job {
    fn from(job: CpuJob) -> Self {
        Self::CpuBound(job)
    }
}
impl From<IOBlockingJob> for Job {
    fn from(job: IOBlockingJob) -> Self {
        Self::IOBlocking(job)
    }
}

pub struct FutureJob(Pin<Box<dyn Future<Output = ()> + Send>>);
pub struct CpuJob(Box<dyn Fn() + Send>);
pub struct IOBlockingJob(Box<dyn FnOnce() + Send>);

use std::sync::mpsc::{SendError, Sender};
use tokio::sync::oneshot;
use tokio::sync::oneshot::Receiver;

pub async fn oneshoot_blocking<T, F>(
    tx: Sender<IOBlockingJob>,
    job: F,
) -> Result<Receiver<T>, SendError<IOBlockingJob>>
where
    T: Send + 'static,
    F: 'static + Sync + Send + FnOnce() -> T,
{
    let (otx, recv) = oneshot::channel();
    let block = IOBlockingJob(Box::new(move || {
        let _ = otx.send(job());
    }));

    tx.send(block)?;
    Ok(recv)
}

impl IOBlockingJob {
    pub fn into_inner(self) -> Box<dyn FnOnce() + Send> {
        self.0
    }
}

impl CpuJob {
    pub fn into_inner(self) -> Box<dyn Fn() + Send> {
        self.0
    }
}

impl FutureJob {
    pub fn into_inner(self) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        self.0
    }
}

impl<T> JobExt for T {}

pub trait JobExt {
    fn cpu_bound(self) -> Job
    where
        Self: Sized + Fn() + Send + 'static,
    {
        Job::CpuBound(Box::new(self).into())
    }
    fn io_blocking(self) -> Job
    where
        Self: Sized + Fn() + Send + 'static,
    {
        Job::IOBlocking(Box::new(self).into())
    }
    fn future(self) -> Job
    where
        Self: Sized + Future<Output = ()> + Send + 'static,
    {
        Job::Future(self.boxed().into())
    }
}

impl<T> From<T> for IOBlockingJob
where
    T: Fn() + Send + 'static,
{
    fn from(func: T) -> Self {
        Self(Box::new(func))
    }
}

impl<T> From<T> for CpuJob
where
    T: Fn() + Send + 'static,
{
    fn from(func: T) -> Self {
        Self(Box::new(func))
    }
}

impl<T> From<T> for FutureJob
where
    T: Future<Output = ()> + Send + 'static,
{
    fn from(fut: T) -> Self {
        Self(fut.boxed())
    }
}

impl<T> From<T> for Job
where
    T: Future<Output = ()> + Send + 'static,
{
    fn from(fut: T) -> Self {
        Self::Future(fut.into())
    }
}
