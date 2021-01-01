use crate::response::ResponderError;
use crate::{Body, Response};
use async_trait::async_trait;
use http::request::Parts;

#[async_trait]
pub trait RequestMiddleware<E: ResponderError> {
    async fn call(&self, p: &Parts) -> Result<(), E>;
}

#[async_trait]
pub trait ResponseMiddleware<E: ResponderError> {
    async fn call(&self, r: &Response<Body>) -> Result<(), E>;
}

pub struct Expect<T>(pub T);

impl<T> PartialEq<T> for Expect<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &T) -> bool {
        &self.0 == other
    }
}

impl<T> PartialEq for Expect<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        &self.0 == &other.0
    }
}