#![forbid(unsafe_code)]

mod app;
mod handler;
mod job;
mod logger;
mod middleware;
mod request;

use proc_macro::TokenStream;
use syn::{parse, parse_macro_input, ExprLit, ItemStruct};

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

#[proc_macro_attribute]
pub fn job(args: TokenStream, input: TokenStream) -> TokenStream {
    job::make_job(args, input)
}

#[proc_macro_attribute]
pub fn req_formatter(args: TokenStream, input: TokenStream) -> TokenStream {
    let expr_lit: ExprLit = parse(args).unwrap();
    let item_struct = parse_macro_input!(input as ItemStruct);
    match logger::make_req_fmt(expr_lit, item_struct) {
        Ok(r) => r,
        Err(e) => e.into_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn resp_formatter(args: TokenStream, input: TokenStream) -> TokenStream {
    let expr_lit: ExprLit = parse(args).unwrap();
    let item_struct = parse_macro_input!(input as ItemStruct);
    match logger::make_res_fmt(expr_lit, item_struct) {
        Ok(r) => r,
        Err(e) => e.into_compile_error().into(),
    }
}

#[proc_macro]
pub fn app(input: TokenStream) -> TokenStream {
    match app::make_app(input) {
        Ok(r) => r,
        Err(e) => e,
    }
}
