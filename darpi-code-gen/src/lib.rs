#![forbid(unsafe_code)]

mod app;
mod handler;
mod request;

extern crate proc_macro;

use darpi_web::Route as DefRoute;
use md5;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{format_ident, quote};
use std::cmp::Ordering;
use std::convert::TryFrom;
use syn::export::ToTokens;
use syn::parse::Parse;
use syn::{
    braced, bracketed, parse::ParseStream, parse_macro_input, parse_quote::ParseQuote,
    punctuated::Punctuated, token::Brace, token::Colon, token::Comma, token::Pound, AttrStyle,
    Attribute, AttributeArgs, Error, ExprLit, ExprPath, Fields, FnArg, GenericArgument, ItemFn,
    ItemStruct, Lit, LitStr, Member, PatType, Path, PathArguments, PathSegment, Type,
};

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
pub fn run(input: TokenStream) -> TokenStream {
    match app::make_run(input) {
        Ok(r) => r,
        Err(e) => e,
    }
}
