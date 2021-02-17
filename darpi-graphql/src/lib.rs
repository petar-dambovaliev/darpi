use async_graphql::{ParseRequestError, Result};
use async_trait::async_trait;
use darpi::header::HeaderValue;
use darpi::{
    body::Bytes, header, hyper, request::FromRequestBody, response::ResponderError, Query,
    StatusCode,
};
use derive_more::Display;
use futures_util::{StreamExt, TryStreamExt};
use http::HeaderMap;
use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use serde_json;
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

pub struct GraphQLBody<T>(pub T);

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

impl<'de, T> Deserialize<'de> for GraphQLBody<T>
where
    T: DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let deser = T::deserialize(deserializer)?.into();
        Ok(GraphQLBody(deser))
    }
}

#[async_trait]
impl FromRequestBody<GraphQLBody<BatchRequest>, GraphQLError> for GraphQLBody<BatchRequest> {
    async fn extract(
        headers: &HeaderMap,
        mut b: darpi::Body,
    ) -> Result<GraphQLBody<BatchRequest>, GraphQLError> {
        let content_type = headers
            .get(http::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_string());

        let (mut tx, rx): (
            Sender<std::result::Result<Bytes, _>>,
            Receiver<std::result::Result<Bytes, _>>,
        ) = tokio::sync::mpsc::channel(16);

        tokio::spawn(async move {
            while let Some(item) = b.next().await {
                if tx
                    .send(item) //.map_err(|e| GraphQLError::Hyper(e))
                    .await
                    .is_err()
                {
                    return;
                }
            }
        });

        Ok(GraphQLBody(BatchRequest(
            async_graphql::http::receive_batch_body(
                content_type,
                rx.map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
                    .into_async_read(),
                Default::default(),
            )
            .await
            .map_err(|e| GraphQLError::ParseRequest(e))?,
        )))
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
impl FromRequestBody<GraphQLBody<Request>, GraphQLError> for GraphQLBody<Request> {
    async fn extract(
        headers: &HeaderMap,
        b: darpi::Body,
    ) -> Result<GraphQLBody<Request>, GraphQLError> {
        let res: GraphQLBody<BatchRequest> = GraphQLBody::extract(headers, b).await?;

        Ok(res
            .0
            .into_inner()
            .into_single()
            .map(|r| GraphQLBody(Request(r)))
            .map_err(|e| GraphQLError::ParseRequest(e))?)
    }
}
