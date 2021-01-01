use crate::middleware::{RequestMiddleware, ResponseMiddleware};
use crate::{Body, Response};
use async_trait::async_trait;
use http::request::Parts;
use log::info;
use std::convert::Infallible;

pub struct Logger<F: Formatter>(Inner<F>);

impl<F: Formatter> Default for Logger<F> {
    fn default() -> Self {
        Logger(Inner {
            formatter: Default::default(),
        })
    }
}

impl<F: Formatter> Logger<F> {
    pub fn new(formatter: F) -> Self {
        Self(Inner { formatter })
    }
}

pub trait Formatter: Default + Send + Sync {
    fn format<T>(&self, _: T) -> String;
}

struct Inner<F: Formatter> {
    formatter: F,
}

impl<F: Formatter> Inner<F> {
    fn info<T>(&self, t: T) -> Result<(), Infallible> {
        let format = self.formatter.format(t);
        info!("{}", format);
        Ok(())
    }
}

#[async_trait]
impl<F: Formatter> RequestMiddleware<Infallible> for Inner<F> {
    async fn call(&self, r: &Parts) -> Result<(), Infallible> {
        self.info(r)
    }
}

#[async_trait]
impl<F: Formatter> ResponseMiddleware<Infallible> for Inner<F> {
    async fn call(&self, r: &Response<Body>) -> Result<(), Infallible> {
        self.info(r)
    }
}
