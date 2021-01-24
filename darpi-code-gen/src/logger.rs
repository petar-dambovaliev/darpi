use logos;
use logos::{Lexer, Logos};
use proc_macro::Ident;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::quote;
use syn::{Error, ExprLit, ItemStruct, Lit};

#[derive(Logos, Debug, PartialEq)]
pub enum ReqFmtTok {
    #[token("%a")]
    RemoteIP,

    #[token("%t")]
    When,

    #[token("%u")]
    Url,

    #[token("%b")]
    BodySize,

    #[regex("%[{][A-Z][a-zA-Z_-]+[}]h")]
    HeaderValue,

    #[regex("%[{][A-Z][a-zA-Z_-]+[}]e")]
    EnvValue,

    #[regex("%s[^%]+")]
    Sep,

    #[error]
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

#[derive(Logos, Debug, PartialEq)]
pub enum RespFmtTok {
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

    #[regex(r"%[{][A-Z][a-zA-Z_-]+[}]h")]
    HeaderValue,

    #[regex(r"%[{][A-Z][a-zA-Z_-]+[}]e")]
    EnvValue,

    #[regex("%s[^%]+")]
    Sep,

    #[error]
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

pub fn make_res_fmt(expr_lit: ExprLit) -> Result<proc_macro::TokenStream, Error> {
    if let Lit::Str(str) = expr_lit.lit {
        let val = str.value();
        let mut lex = RespFmtTok::lexer(&val);
        let mut variables = vec![quote! {let mut content = vec!["[darpi::request]".to_string()];}];

        while let Some(next) = lex.next() {
            match next {
                RespFmtTok::Error => {
                    return Err(Error::new(
                        Span::call_site(),
                        format!("invalid format value: {:#?}", next),
                    ))
                }
                RespFmtTok::RemoteIP => variables.push(quote! {
                    if let Some(forwarded) = rp.headers.get(darpi::header::FORWARDED) {
                        let forwarded = format!(
                            "remote_ip: {}",
                            forwarded.to_str().map_err(|_| "").expect("never to happen")
                        );
                        content.push(forwarded);
                    }
                }),
                RespFmtTok::When => {
                    variables.push(quote! {
                        let now = format!("when: {:#?}", Instant::now());
                        content.push(now);
                    });
                }
                RespFmtTok::Took => {}
                RespFmtTok::Status => {}
                RespFmtTok::Url => {}
                RespFmtTok::BodySize => {}
                RespFmtTok::HeaderValue => {}
                RespFmtTok::EnvValue => {}
                RespFmtTok::Sep => {}
            }
        }
    }

    Err(Error::new(
        Span::call_site(),
        "only string literal is supported",
    ))
}

pub fn make_req_fmt(
    expr_lit: ExprLit,
    item_struct: ItemStruct,
) -> Result<proc_macro::TokenStream, Error> {
    if let Lit::Str(str) = expr_lit.lit {
        let val = str.value();
        let mut lex = ReqFmtTok::lexer(&val);
        let mut variables = vec![quote! {let mut content = vec!["[darpi::request]".to_string()];}];

        while let Some(next) = lex.next() {
            match next {
                ReqFmtTok::Error => {
                    return Err(Error::new(
                        Span::call_site(),
                        format!("invalid format value: {:#?}", next),
                    ))
                }
                ReqFmtTok::RemoteIP => variables.push(quote! {
                    if let Some(forwarded) = rp.headers.get(darpi::header::FORWARDED) {
                        let forwarded = format!(
                            "remote_ip: {}",
                            forwarded.to_str().map_err(|_| "").expect("never to happen")
                        );
                        content.push(forwarded);
                    }
                }),
                ReqFmtTok::When => {
                    variables.push(quote! {
                        let now = format!("when: {:#?}", Instant::now());
                        content.push(now);
                    });
                }
                ReqFmtTok::Url => {
                    variables.push(quote! {
                        let uri = format!("uri: {:#?}", rp.uri);
                        content.push(uri);
                    });
                }
                ReqFmtTok::BodySize => {
                    variables.push(quote! {
                        let size = format!("body_size: {:#?}", b.size_hint());
                        content.push(size);
                    });
                }
                ReqFmtTok::HeaderValue => {
                    let variable = lex.slice();

                    variables.push(quote! {
                        if let Some(variable) = rp.headers.get(#variable) {
                        let variable = format!(
                            "{}: {}",
                            #variable,
                            variable.to_str().map_err(|_| "").expect("never to happen")
                        );
                        content.push(variable);
                    }
                    });
                }
                ReqFmtTok::EnvValue => {
                    let variable = lex.slice();

                    variables.push(quote! {
                        if let Ok(variable) = std::env::var(#variable) {
                            content.push(format!("{}: {}", #variable, variable));
                        }
                    });
                }
                ReqFmtTok::Sep => {
                    let sep = lex.slice();

                    variables.push(quote! {
                        content.push(format!("{}", #sep));
                    });
                }
            }
        }

        variables.push(quote! {
            content.join(" ").into()
        });

        let name = &item_struct.ident;
        let q = quote! {
            #item_struct
            impl darpi::ReqFormatter for #name {
                fn format_req(&self, b: &darpi::Body, rp: &darpi::RequestParts) -> String {
                    use darpi::HttpBody;
                    #(#variables )*
                }
            }
        };
        //panic!("{}", q.to_string());
        return Ok(q.into());
    }

    Err(Error::new(
        Span::call_site(),
        "only string literal is supported",
    ))
}
