use darpi::{handler, req_formatter};
use darpi_middleware::{log_request, log_response};

#[req_formatter("%a %s-- %b")]
pub struct MyFormatter;

#[handler([log_request(MyFormatter), log_response()])]
async fn hello_world() -> &'static str {
    "hello world"
}

async fn main() {}
