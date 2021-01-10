use logos;
use logos::Logos;
//use proc_macro2::{Span, TokenStream, TokenTree};
//use quote::quote;
//use std::convert::TryFrom;
//use syn::parse::{Error as ParseError, ParseStream, Parser};
//use syn::{Error, ExprLit, Lit};

#[derive(Logos, Debug, PartialEq)]
pub enum FormatToken {
    #[token("%a")]
    RemoteIP,

    #[token("%t")]
    When,

    #[token("%T")]
    Took,

    #[token("%s")]
    Status,

    #[token("%u")]
    Url,

    #[token("%b")]
    BodySize,

    #[token(r"%[{][A-Z][a-zA-Z_-]+[}]h")]
    HeaderValue,

    #[token(r"%[{][A-Z][a-zA-Z_-]+[}]e")]
    EnvValue,

    #[error]
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

// impl<'a> TryFrom<&'a str> for ReqRoute<'a> {
//     type Error = String;
//
//     fn try_from(s: &'a str) -> Result<Self, Self::Error> {
//         let mut lex = ReqToken::lexer(s);
//         let mut values: Vec<ReqToken<'a>> = vec![];
//
//         while let Some(next) = lex.next() {
//             match next {
//                 ReqToken::Error => return Err("invalid ReqRoute".to_string()),
//                 _ => values.push(next),
//             }
//         }
//
//         Ok(Self { values })
//     }
// }

// pub fn make_format(expr_lit: ExprLit) -> Result<TokenStream, Error> {
//     // if let Lit::Str(str) = expr_lit.lit {
//     //     return str.parse_with(parse_format);
//     // }
//
//     Err(Error::new(
//         Span::call_site(),
//         "only string literal is supported",
//     ))
// }
