use std::convert::Infallible;
use std::error::Error as StdError;
use std::future::Future;
use std::net::SocketAddr;

use futures::{future, FutureExt, TryFuture, TryStream, TryStreamExt};
use hyper::server::conn::{AddrIncoming, AddrStream};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server as HyperServer};
use tokio::io::{AsyncRead, AsyncWrite};

pub struct App {}

impl App {
    pub fn route(&mut self) {}
}

/// A Warp Server ready to filter requests.
#[derive(Debug)]
pub struct Server {
    pipeline: bool,
}

pub trait Transport: AsyncRead + AsyncWrite {
    fn remote_addr(&self) -> Option<SocketAddr>;
}

impl Transport for AddrStream {
    fn remote_addr(&self) -> Option<SocketAddr> {
        Some(self.remote_addr())
    }
}
