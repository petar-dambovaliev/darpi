use async_trait::async_trait;
use darpi::response::ResponderError;
use darpi::{middleware, RequestMiddleware, RequestParts};
use darpi_web::Body;
use http::Response;
use shaku::Interface;
use std::convert::Infallible;
use std::sync::Arc;

trait Bar: Interface {}

#[middleware(Request)]
pub async fn authorize(
    #[request_parts] rp: &RequestParts,
    #[inject] algo_provider: Arc<dyn Bar>,
) -> Result<(), String> {
    Ok(())
}
// #[middleware(Request)]
// async fn hello_world(#[request_parts] rp: &RequestParts) -> Result<(), Infallible> {
//     Ok(())
// }

// pub async fn body_size_limit(b: &Body, size: u64) -> Result<(), String> {
//     Ok(())
// }

#[test]
fn main() {
    // let mut rp = RequestParts {
    //     method: Default::default(),
    //     uri: Default::default(),
    //     version: Default::default(),
    //     headers: Default::default(),
    //     extensions: Default::default(),
    //     _priv: (),
    // };
    //
    // let mut b = Body::empty();
    //
    // body_size_limit::call(&mut rp, Arc::new(()), &mut b, ());
}
