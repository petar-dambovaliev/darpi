use crate::request::FromRequest;
use crate::response::{Responder, ResponderError};
use crate::Response;
use derive_more::Display;
use futures::future::LocalBoxFuture;
use futures::FutureExt;
use http::header;
use hyper::Body;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Error;
use std::{fmt, ops};

pub struct Json<T>(pub T);

impl<T> Json<T> {
    pub fn into_inner(self) -> T {
        self.0
    }

    async fn deserialize_future(b: Body) -> Result<Json<T>, JsonErr>
    where
        T: DeserializeOwned,
    {
        let full_body = hyper::body::to_bytes(b).await?;
        let ser: T = serde_json::from_slice(&full_body)?;
        Ok(Json(ser))
    }
}

impl<'de, T> Deserialize<'de> for Json<T>
where
    T: DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let deser = T::deserialize(deserializer)?.into();
        Ok(Json(deser))
    }
}

impl<T> ops::Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> ops::DerefMut for Json<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> fmt::Debug for Json<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Json: {:?}", self.0)
    }
}

impl<T> fmt::Display for Json<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl<T> Responder for Json<T>
where
    T: Serialize,
{
    fn respond(self) -> Response<Body> {
        match serde_json::to_string(&self.0) {
            Ok(body) => Response::builder()
                .header(header::CONTENT_TYPE, "application/json")
                .status(self.status_code())
                .body(Body::from(body))
                .expect("this cannot happen"),
            Err(e) => e.respond_err(),
        }
    }
}

#[derive(Display)]
pub enum JsonErr {
    ReadBody(hyper::Error),
    Serde(Error),
}

impl From<Error> for JsonErr {
    fn from(e: Error) -> Self {
        Self::Serde(e)
    }
}

impl From<hyper::Error> for JsonErr {
    fn from(e: hyper::Error) -> Self {
        Self::ReadBody(e)
    }
}

impl ResponderError for JsonErr {}

impl<T: 'static> FromRequest<Self, JsonErr> for Json<T>
where
    T: DeserializeOwned,
{
    type Future = LocalBoxFuture<'static, Result<Self, JsonErr>>;

    fn extract(b: Body) -> Self::Future {
        Self::deserialize_future(b).boxed_local()
    }
}

impl ResponderError for serde_json::Error {}
