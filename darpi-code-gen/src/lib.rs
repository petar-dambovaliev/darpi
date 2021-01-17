#![forbid(unsafe_code)]

mod app;
mod handler;
mod logger;
mod middleware;
mod request;

use proc_macro::TokenStream;

#[proc_macro_derive(Path)]
pub fn path(input: TokenStream) -> TokenStream {
    request::make_path_type(input)
}

#[proc_macro_derive(Query)]
pub fn query(input: TokenStream) -> TokenStream {
    request::make_query_type(input)
}

#[proc_macro_attribute]
pub fn handler(args: TokenStream, input: TokenStream) -> TokenStream {
    handler::make_handler(args, input)
}

#[proc_macro_attribute]
pub fn middleware(args: TokenStream, input: TokenStream) -> TokenStream {
    middleware::make_middleware(args, input)
}

#[proc_macro]
pub fn app(input: TokenStream) -> TokenStream {
    match app::make_app(input) {
        Ok(r) => r,
        Err(e) => e,
    }
}
