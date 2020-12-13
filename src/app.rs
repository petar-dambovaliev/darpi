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

// impl Server {
//     /// Run this `Server` forever on the current thread.
//     pub async fn run(self, addr: impl Into<SocketAddr>) {
//         self.bind(addr).ins
//     }
//
//     /// Bind to a socket address, returning a `Future` that can be
//     /// executed on any runtime.
//     ///
//     /// # Panics
//     ///
//     /// Panics if we are unable to bind to the provided address.
//     pub fn bind(self, addr: impl Into<SocketAddr> + 'static) -> impl Future<Output = ()> + 'static {
//         let (_, fut) = self.bind_ephemeral(addr);
//         fut
//     }
//     /// Bind to a possibly ephemeral socket address.
//     ///
//     /// Returns the bound address and a `Future` that can be executed on
//     /// any runtime.
//     ///
//     /// # Panics
//     ///
//     /// Panics if we are unable to bind to the provided address.
//     pub fn bind_ephemeral(
//         self,
//         addr: impl Into<SocketAddr>,
//     ) -> (SocketAddr, impl Future<Output = ()> + 'static) {
//         let (addr, incoming) = {
//             let mut incoming = AddrIncoming::bind(addr)?;
//             incoming.set_nodelay(true);
//             let addr = incoming.local_addr();
//             (addr, incoming)
//         };
//
//         let service = make_service_fn(move |transport| {
//             let inner = inner.clone();
//             let remote_addr = Transport::remote_addr(transport);
//             future::ok::<_, Infallible>(service_fn(move |req| {
//                 inner.call_with_addr(req, remote_addr)
//             }))
//         });
//
//         let srv = HyperServer::builder(incoming)
//             .http1_pipeline_flush(self.pipeline)
//             .serve(service);
//
//         let srv = srv.map(|result| {
//             if let Err(err) = result {
//                 //error!("server error: {}", err)
//             }
//         });
//
//         (addr, srv)
//     }
// }

pub trait Transport: AsyncRead + AsyncWrite {
    fn remote_addr(&self) -> Option<SocketAddr>;
}

impl Transport for AddrStream {
    fn remote_addr(&self) -> Option<SocketAddr> {
        Some(self.remote_addr())
    }
}
