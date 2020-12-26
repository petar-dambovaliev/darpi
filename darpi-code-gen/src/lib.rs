#![forbid(unsafe_code)]

mod app;
mod handler;
mod request;

use proc_macro::TokenStream;

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

#[proc_macro]
pub fn app(input: TokenStream) -> TokenStream {
    match app::make_app(input) {
        Ok(r) => r,
        Err(e) => e,
    }
}
