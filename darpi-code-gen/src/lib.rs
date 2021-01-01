#![forbid(unsafe_code)]

mod app;
mod handler;
mod logger;
mod middleware;
mod request;

use proc_macro::TokenStream;
use syn::{parse::ParseStream, parse_macro_input, ExprLit};

#[proc_macro]
pub fn format(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ExprLit);
    match logger::make_format(input) {
        Ok(r) => r.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn path_type(attr: TokenStream, input: TokenStream) -> TokenStream {
    request::make_path_type(attr, input)
}

#[proc_macro_attribute]
pub fn query_type(attr: TokenStream, input: TokenStream) -> TokenStream {
    request::make_query_type(attr, input)
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
