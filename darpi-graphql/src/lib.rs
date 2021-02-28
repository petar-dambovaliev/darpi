use async_graphql::http::MultipartOptions;
use async_graphql::{ParseRequestError, Result};
use async_trait::async_trait;
use darpi::header::HeaderValue;
use darpi::request::QueryPayloadError;
use darpi::{
    body::Bytes, header, hyper, request::FromRequestBody, response::ResponderError, Body, Query,
    StatusCode,
};
use derive_more::Display;
use futures_util::{StreamExt, TryStreamExt};
use http::HeaderMap;
use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use serde_json;
use shaku::{Component, HasComponent, Interface};
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Debug, Deserialize, Query)]
pub struct BatchRequest(pub async_graphql::BatchRequest);

impl BatchRequest {
    #[must_use]
    pub fn into_inner(self) -> async_graphql::BatchRequest {
        self.0
    }
}

#[derive(Debug, Deserialize)]
pub struct Response(pub async_graphql::Response);

impl darpi::response::Responder for Response {
    fn respond(self) -> darpi::Response<darpi::Body> {
        let mut res = darpi::Response::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .status(StatusCode::OK)
            .body(darpi::Body::from(serde_json::to_string(&self.0).unwrap()))
            .unwrap();

        if self.0.is_ok() {
            if let Some(cache_control) = self.0.cache_control.value() {
                res.headers_mut()
                    .insert("cache-control", cache_control.parse().unwrap());
            }
            for (name, value) in self.0.http_headers {
                if let Some(header_name) = name {
                    if let Ok(val) = HeaderValue::from_str(&value) {
                        res.headers_mut().insert(header_name, val);
                    }
                }
            }
        }
        res
    }
}

impl From<async_graphql::Response> for Response {
    fn from(r: async_graphql::Response) -> Self {
        Self(r)
    }
}

pub struct GraphQLBody<T, C>(pub T, PhantomData<C>);

impl<C> darpi::response::ErrResponder<darpi::request::QueryPayloadError, darpi::Body>
    for GraphQLBody<Request, C>
{
    fn respond_err(e: QueryPayloadError) -> darpi::Response<Body> {
        Request::respond_err(e)
    }
}

#[derive(Display)]
pub enum GraphQLError {
    ParseRequest(ParseRequestError),
    Hyper(hyper::Error),
}

impl From<ParseRequestError> for GraphQLError {
    fn from(e: ParseRequestError) -> Self {
        Self::ParseRequest(e)
    }
}

impl From<hyper::Error> for GraphQLError {
    fn from(e: hyper::Error) -> Self {
        Self::Hyper(e)
    }
}

impl ResponderError for GraphQLError {}

impl<'de, T, C> Deserialize<'de> for GraphQLBody<T, C>
where
    T: DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let deser = T::deserialize(deserializer)?.into();
        Ok(GraphQLBody(deser, PhantomData))
    }
}

pub trait MultipartOptionsProvider: Interface {
    fn get(&self) -> MultipartOptions;
}

#[derive(Component)]
#[shaku(interface = MultipartOptionsProvider)]
pub struct MultipartOptionsProviderImpl {
    opts: MultipartOptions,
}

impl MultipartOptionsProvider for MultipartOptionsProviderImpl {
    fn get(&self) -> MultipartOptions {
        self.opts.clone()
    }
}

#[async_trait]
impl<C: 'static> FromRequestBody<GraphQLBody<BatchRequest, C>, GraphQLError>
    for GraphQLBody<BatchRequest, C>
where
    C: HasComponent<dyn MultipartOptionsProvider>,
{
    type Container = Arc<C>;

    async fn extract(
        headers: &HeaderMap,
        mut b: darpi::Body,
        container: Self::Container,
    ) -> Result<GraphQLBody<BatchRequest, C>, GraphQLError> {
        let content_type = headers
            .get(http::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_string());

        let (mut tx, rx): (
            Sender<std::result::Result<Bytes, _>>,
            Receiver<std::result::Result<Bytes, _>>,
        ) = tokio::sync::mpsc::channel(16);

        tokio::runtime::Handle::current().spawn(async move {
            while let Some(item) = b.next().await {
                if tx.send(item).await.is_err() {
                    return;
                }
            }
        });

        let opts = container.resolve().get();
        Ok(GraphQLBody(
            BatchRequest(
                async_graphql::http::receive_batch_body(
                    content_type,
                    rx.map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
                        .into_async_read(),
                    opts,
                )
                .await
                .map_err(|e| GraphQLError::ParseRequest(e))?,
            ),
            PhantomData,
        ))
    }
}

#[derive(Debug, Deserialize, Query)]
pub struct Request(pub async_graphql::Request);

impl Request {
    #[must_use]
    pub fn into_inner(self) -> async_graphql::Request {
        self.0
    }
}

#[async_trait]
impl<C: 'static> FromRequestBody<GraphQLBody<Request, C>, GraphQLError> for GraphQLBody<Request, C>
where
    C: HasComponent<dyn MultipartOptionsProvider>,
{
    type Container = Arc<C>;

    async fn extract(
        headers: &HeaderMap,
        b: darpi::Body,
        container: Self::Container,
    ) -> Result<GraphQLBody<Request, C>, GraphQLError> {
        let res: GraphQLBody<BatchRequest, C> = GraphQLBody::extract(headers, b, container).await?;

        Ok(res
            .0
            .into_inner()
            .into_single()
            .map(|r| GraphQLBody(Request(r), PhantomData))
            .map_err(|e| GraphQLError::ParseRequest(e))?)
    }
}
