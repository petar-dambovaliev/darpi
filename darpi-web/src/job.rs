use crate::{Body, Response};
use async_trait::async_trait;
use futures::Future;
use futures_util::FutureExt;
use http::request::Parts as RequestParts;
use std::pin::Pin;
use std::sync::mpsc::{SendError, Sender};
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Receiver;

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
pub struct CpuJob(Box<dyn FnOnce() + Send>);
pub struct IOBlockingJob(Box<dyn FnOnce() + Send>);

#[async_trait]
pub trait SenderExt<T, J> {
    async fn oneshot<F>(self, job: F) -> Result<Receiver<T>, SendError<J>>
    where
        J: 'static + Into<Job>,
        T: Send + 'static,
        F: 'static + Send + FnOnce() -> T;
}

#[async_trait]
impl<T> SenderExt<T, IOBlockingJob> for Sender<IOBlockingJob> {
    async fn oneshot<F>(self, job: F) -> Result<Receiver<T>, SendError<IOBlockingJob>>
    where
        T: Send + 'static,
        F: 'static + Send + FnOnce() -> T,
    {
        let (otx, recv) = oneshot::channel();
        let block = IOBlockingJob(Box::new(move || {
            let _ = otx.send(job());
        }));

        self.send(block)?;
        Ok(recv)
    }
}

#[async_trait]
impl<T> SenderExt<T, CpuJob> for Sender<CpuJob> {
    async fn oneshot<F>(self, job: F) -> Result<Receiver<T>, SendError<CpuJob>>
    where
        T: Send + 'static,
        F: 'static + Send + FnOnce() -> T,
    {
        let (otx, recv) = oneshot::channel();
        let block = CpuJob(Box::new(move || {
            let _ = otx.send(job());
        }));

        self.send(block)?;
        Ok(recv)
    }
}

impl IOBlockingJob {
    pub fn into_inner(self) -> Box<dyn FnOnce() + Send> {
        self.0
    }
}

impl CpuJob {
    pub fn into_inner(self) -> Box<dyn FnOnce() + Send> {
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
