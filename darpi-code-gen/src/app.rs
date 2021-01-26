use crate::handler::{HAS_NO_PATH_ARGS_PREFIX, HAS_PATH_ARGS_PREFIX, NO_BODY_PREFIX};
use darpi_route::Route as DefRoute;
use md5;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::ToTokens;
use quote::{format_ident, quote};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::convert::TryFrom;
use syn::parse::Parse;
use syn::token::Token;
use syn::{
    braced, bracketed, parse::ParseStream, parse_quote::ParseQuote, punctuated::Punctuated,
    token::Brace, token::Colon, token::Comma, token::FatArrow, Error, Expr as SynExpr, ExprCall,
    ExprLit, ExprPath, Lit, LitStr, Member, Type,
};

pub(crate) fn make_app(input: TokenStream) -> Result<TokenStream, TokenStream> {
    let app_struct: AppStruct =
        syn::parse(input).unwrap_or_else(|e| panic!("app_struct: {:#?}", e));

    let FieldResult {
        address,
        module_path,
        handlers,
        middlewares,
    } = get_fields(app_struct)?;

    let address_value = match address {
        Address::LitStr(ls) => {
            let address: std::net::SocketAddr = ls
                .value()
                .parse()
                .expect(&format!("invalid server address: `{}`", ls.value()));
            let s = format!("{}", address.to_string());
            let q = quote! {#s};
            q.to_token_stream()
        }
        Address::Ident(i) => {
            let q = quote! {&#i};
            q.to_token_stream()
        }
    };

    if handlers.is_empty() {
        return Err(Error::new(Span::call_site(), "no handlers registered")
            .to_compile_error()
            .into());
    }

    let handler_len = handlers.len();
    let HandlerTokens {
        routes,
        route_arg_assert,
        route_arg_assert_def,
        routes_match,
        is,
        body_assert,
        body_assert_def,
    } = make_handlers(handlers);

    let route_possibilities = quote! {
        use std::convert::TryFrom;
        #[allow(non_camel_case_types, missing_docs)]
        pub enum RoutePossibilities {
            #(#routes ,)*
        }

        impl RoutePossibilities {
            pub fn get_route<'a>(&self, route: &'a str, method: &darpi::Method) -> Option<(darpi::ReqRoute<'a>, std::collections::HashMap<&'a str, &'a str>)> {
                return match self {
                    #(#is ,)*
                };
            }
        }
    };

    let (module_def, module_let, module_self, mut middleware_req, mut middleware_res) =
        module_path.map_or(Default::default(), |mp| {
            let make_container_func = mp
                .key
                .path
                .get_ident()
                .expect("could not get container function");
            let patj = mp
                .value
                .path
                .get_ident()
                .expect("could not get module_ident");

            let cloned = mp.value.path.get_ident().cloned();

            let fake_ident = format_ident!("global");
            let map: HashMap<u64, Type> = HashMap::new();

            let mut middleware_req = vec![];
            let mut middleware_res = vec![];
            let mut i = 0u16;

            let h_req = format_ident!("{}_request", fake_ident);
            let h_res = format_ident!("{}_response", fake_ident);
            middlewares.iter().for_each(|e| {
                let name = &e.func;
                let m_arg_ident = format_ident!("m_arg_{}", i);
                let mut sorter = 0_u16;

                let m_args: Vec<proc_macro2::TokenStream> = e.args.iter().map(|arg| {
                    if let SynExpr::Call(expr_call) = arg {
                        if expr_call.func.to_token_stream().to_string() == "middleware" {
                            let index: u16 = expr_call.args.first().unwrap().to_token_stream().to_string().parse().unwrap();
                            sorter += index;
                            let i_ident = format_ident!("m_arg_{}", index);
                            return quote!{#i_ident.clone()};
                        }
                    }
                    quote! {#arg}
                }).collect();

                middleware_req.push((sorter, quote! {
                    let #m_arg_ident = match #name::call_Request(&mut parts, #(#m_args ,)* inner_module.clone(), &mut body).await {
                        Ok(k) => k,
                        Err(e) => return Ok(e.respond_err()),
                    };
                }));

                middleware_res.push((std::u16::MAX - i - sorter, quote! {
                    let #m_arg_ident = match #name::call_Response(&mut rb, #(#m_args ,)* inner_module.clone()).await {
                        Ok(k) => k,
                        Err(e) => return Ok(e.respond_err()),
                    };
                }));
            });

            (
                quote! {module: std::sync::Arc<#patj>,},
                quote! {let module = std::sync::Arc::new(#make_container_func());},
                quote! {module: module,},
                middleware_req,
                middleware_res,
            )
        });

    middleware_req.sort_by(|a, b| a.0.cmp(&b.0));
    middleware_res.sort_by(|a, b| a.0.cmp(&b.0));

    let middleware_req: Vec<proc_macro2::TokenStream> =
        middleware_req.into_iter().map(|e| e.1).collect();
    let middleware_res: Vec<proc_macro2::TokenStream> =
        middleware_res.into_iter().map(|e| e.1).collect();

    let app = quote! {
        #(#body_assert_def )*
        #(#route_arg_assert_def )*

         pub struct App {
            #module_def
            handlers: std::sync::Arc<[RoutePossibilities; #handler_len]>,
            address: std::net::SocketAddr,
        }

        impl App {
            pub fn new(address: &str) -> Self {
                #(#body_assert;)*
                #(#route_arg_assert;)*
                let address: std::net::SocketAddr = address
                    .parse()
                    .expect(&format!("invalid server address: `{}`", address));

                #module_let
                Self {
                    #module_self
                    handlers: std::sync::Arc::new([#(RoutePossibilities::#routes ,)*]),
                    address: address,
                }
            }

             pub async fn run(self) -> Result<(), darpi::Error> {
                let address = self.address;
                let module = self.module.clone();
                let handlers = self.handlers.clone();

                let make_svc = darpi::service::make_service_fn(move |_conn| {
                    let inner_module = std::sync::Arc::clone(&module);
                    let inner_handlers = std::sync::Arc::clone(&handlers);
                    async move {
                        Ok::<_, std::convert::Infallible>(darpi::service::service_fn(move |r: darpi::Request<darpi::Body>| {
                            use darpi::futures::FutureExt;
                            let inner_module = std::sync::Arc::clone(&inner_module);
                            let inner_handlers = std::sync::Arc::clone(&inner_handlers);
                            async move {
                                let route = r.uri().path().to_string();
                                let method = r.method().clone();

                                let (mut parts, mut body) = r.into_parts();

                                #(#middleware_req )*

                                let mut handler = None;
                                for rp in inner_handlers.iter() {
                                    if let Some(rr) = rp.get_route(&route, &method) {
                                        handler = Some((rp, rr));
                                        break;
                                    }
                                }

                                let handler = match handler {
                                    Some(s) => s,
                                    None => return  async {
                                         Ok::<_, std::convert::Infallible>(darpi::Response::builder()
                                                .status(darpi::StatusCode::NOT_FOUND)
                                                .body(darpi::Body::empty())
                                                .unwrap())
                                    }.await,
                                };

                                let mut rb = match handler.0 {
                                    #(#routes_match ,)*
                                };

                                if let Ok(rb) = rb.as_mut() {
                                    #(#middleware_res )*
                                }

                                rb
                            }
                        }))
                    }
                });

                let server = darpi::Server::bind(&address).serve(make_svc);
                Ok(server.await?)
             }
        }
    };

    let tokens = quote! {
        {
            #route_possibilities
            #app
            App::new(#address_value)
        }
    };
    //panic!("{}", tokens.to_string());
    Ok(tokens.into())
}

struct HandlerTokens {
    routes: Vec<proc_macro2::TokenStream>,
    route_arg_assert: Vec<proc_macro2::TokenStream>,
    route_arg_assert_def: Vec<proc_macro2::TokenStream>,
    routes_match: Vec<proc_macro2::TokenStream>,
    is: Vec<proc_macro2::TokenStream>,
    body_assert: Vec<proc_macro2::TokenStream>,
    body_assert_def: Vec<proc_macro2::TokenStream>,
}

fn make_handlers(handlers: Vec<ExprHandler>) -> HandlerTokens {
    let mut is = vec![];
    let mut routes = vec![];
    let mut routes_match = vec![];
    let mut body_assert = vec![];
    let mut body_assert_def = vec![];
    let mut route_arg_assert = vec![];
    let mut route_arg_assert_def = vec![];

    handlers.iter().for_each(|el| {
        let handler = el
            .handler
            .path
            .segments
            .last()
            .expect("cannot get handler segment")
            .ident
            .clone();

        let method = el.method.path.segments.to_token_stream();
        let route = el.route.clone();

        let hash = md5::compute(format!("{}{}", handler.clone(), method.clone()));
        let variant_name = format_ident!("a{}", format!("{:?}", hash));
        let variant_value = el
            .handler
            .path
            .get_ident()
            .expect("cannot get handler path ident");

        let method_name = el.method.path.segments.last().unwrap();
        let mut f_name = format_ident!("assert_has_no_path_args_{}", variant_value);
        let mut t_name = format_ident!("{}_{}", HAS_NO_PATH_ARGS_PREFIX, variant_value);

        if route.clone().to_token_stream().to_string().contains('{') {
            f_name = format_ident!("assert_has_path_args_{}", variant_value);
            t_name = format_ident!("{}_{}", HAS_PATH_ARGS_PREFIX, variant_value);
        }

        //todo fix use the handler path
        //route_arg_assert_def.push(quote! {fn #f_name<T>() where T: #t_name {}});
        // route_arg_assert.push(quote! {
        //     #f_name::<#variant_value>();
        // });

        if method_name.ident == "GET" {
            let f_name = format_ident!("assert_no_body_{}", variant_value);
            let t_name = format_ident!("{}_{}", NO_BODY_PREFIX, variant_value);
            // body_assert_def.push(quote! {fn #f_name<T>() where T: #t_name {}});
            // body_assert.push(quote! {
            //     #f_name::<#variant_value>();
            // });
        }

        is.push(quote! {
            RoutePossibilities::#variant_name => {
                let req_route = darpi::ReqRoute::try_from(route).unwrap();
                let def_route = darpi::Route::try_from(#route).unwrap();
                if def_route == req_route && method == #method.as_str() {
                    let args = req_route.extract_args(&def_route).unwrap();
                    return Some((req_route, args));
                }
                None
            }
        });

        let route_str = route.to_token_stream().to_string();
        routes.push((
            quote! {
                #variant_name
            },
            route_str,
        ));

        routes_match.push(quote! {
            RoutePossibilities::#variant_name => {
                #variant_value::call(parts, body, handler.1, inner_module.clone()).await
            }
        });
    });

    routes.sort_by(|left, right| {
        let left_matches: Vec<usize> = left.1.match_indices('{').map(|t| t.0).collect();

        if left_matches.is_empty() {
            return Ordering::Less;
        }

        let left_count = left_matches.iter().fold(0, |acc, a| acc + a);
        let right_matches: Vec<usize> = right.1.match_indices('{').map(|t| t.0).collect();

        if right_matches.is_empty() {
            return Ordering::Greater;
        }

        let right_count = right_matches.iter().fold(0, |acc, a| acc + a);

        if left_matches.len() + left_count > right_matches.len() + right_count {
            return Ordering::Less;
        }

        Ordering::Greater
    });

    let routes: Vec<proc_macro2::TokenStream> = routes.into_iter().map(|(ts, _)| ts).collect();

    HandlerTokens {
        routes,
        route_arg_assert,
        route_arg_assert_def,
        routes_match,
        is,
        body_assert,
        body_assert_def,
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ExprKeyValue {
    pub key: ExprPath,
    pub sep: FatArrow,
    pub value: ExprPath,
}

impl syn::parse::Parse for ExprKeyValue {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let key: ExprPath = input.parse()?;
        let sep: FatArrow = input.parse()?;
        let value: ExprPath = input.parse()?;
        Ok(Self { key, sep, value })
    }
}

#[derive(Debug)]
enum Expr {
    ExprArrayHandler(ExprArrayHandler),
    Module(ExprKeyValue),
    ExprLit(ExprLit),
    ExprPath(ExprPath),
    Ident(Ident),
    Punctuated(Punctuated<ExprCall, Comma>),
}

#[derive(Debug)]
struct FieldValue {
    pub member: Ident,
    pub colon_token: Colon,
    pub expr: Expr,
}

fn parse_variant<T>(input: ParseStream) -> Result<T, Error>
where
    T: Parse,
{
    let val: T = match input.parse() {
        Ok(m) => m,
        Err(e) => {
            let name = std::any::type_name::<T>();
            return Err(Error::new(
                Span::call_site(),
                format!("could not parse {}: {}", name, e),
            ));
        }
    };
    Ok(val)
}

impl syn::parse::Parse for FieldValue {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let member: Member = parse_variant(input)?;

        let member_ident = if let Member::Named(ident) = member {
            ident
        } else {
            return Err(Error::new(
                Span::call_site(),
                format!("tuples are not supported"),
            ));
        };

        let colon_token: Colon = parse_variant(input)?;

        let value = if member_ident == "module" {
            let val: ExprKeyValue = parse_variant(input)?;
            Expr::Module(val)
        } else if member_ident == "bind" {
            let val: ExprArrayHandler = parse_variant(input)?;
            Expr::ExprArrayHandler(val)
        } else if member_ident == "route" {
            let val: ExprLit = parse_variant(input)?;

            match &val.lit {
                Lit::Str(lit) => {
                    match DefRoute::try_from(lit.value().as_str()) {
                        Ok(_) => {}
                        Err(e) => return Err(Error::new(Span::call_site(), e)),
                    };
                    Expr::ExprLit(val)
                }
                _ => return Err(Error::new(Span::call_site(), "invalid route")),
            }
        } else if member_ident == "address" {
            if Ident::peek(input.cursor()) {
                let val: Ident = parse_variant(input)?;
                Expr::Ident(val)
            } else {
                let val: ExprLit = parse_variant(input)?;
                Expr::ExprLit(val)
            }
        } else if member_ident == "method" {
            let val: ExprPath = parse_variant(input)?;
            Expr::ExprPath(val)
        } else if member_ident == "handler" {
            let val: ExprPath = parse_variant(input)?;
            Expr::ExprPath(val)
        } else if member_ident == "middleware" {
            let content;
            let _ = bracketed!(content in input);
            let val: Punctuated<ExprCall, Comma> = Punctuated::parse(&content)?;
            Expr::Punctuated(val)
        } else {
            return Err(Error::new(
                Span::call_site(),
                format!("unknown member: {}", member_ident),
            ));
        };

        Ok(FieldValue {
            member: member_ident,
            colon_token,
            expr: value,
        })
    }
}

#[derive(Debug)]
struct AppStruct {
    pub brace_token: Brace,
    pub fields: Punctuated<FieldValue, Comma>,
}

impl syn::parse::Parse for AppStruct {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let content;
        let brace_token = braced!(content in input);
        let fields: Punctuated<FieldValue, Comma> = Punctuated::parse(ParseStream::from(&content))?;

        Ok(Self {
            brace_token,
            fields,
        })
    }
}

#[derive(Debug)]
struct ExprArrayHandler {
    elements: Punctuated<ExprHandler, Comma>,
}

impl syn::parse::Parse for ExprArrayHandler {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let content;
        let _ = bracketed!(content in input);
        let elements: Punctuated<ExprHandler, Comma> = Punctuated::parse(&content)?;

        Ok(Self { elements })
    }
}

#[derive(Debug, Clone)]
struct ExprHandler {
    route: ExprLit,
    method: ExprPath,
    handler: ExprPath,
}

impl syn::parse::Parse for ExprHandler {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let content;
        let _ = braced!(content in input);
        let fields: Punctuated<FieldValue, Comma> = Punctuated::parse(ParseStream::from(&content))?;

        let route = fields
            .iter()
            .find(|f| &f.member == "route")
            .expect("could not get route");

        let method = fields
            .iter()
            .find(|f| &f.member == "method")
            .expect("could not get method");

        let handler = fields
            .iter()
            .find(|f| &f.member == "handler")
            .expect("could not get handler");

        let route = match &route.expr {
            Expr::ExprLit(el) => el.clone(),
            _ => {
                return Err(Error::new(
                    Span::call_site(),
                    "expected route to be a &str literal",
                ));
            }
        };

        let method = match &method.expr {
            Expr::ExprPath(el) => el.clone(),
            _ => {
                return Err(Error::new(
                    Span::call_site(),
                    "expected method to be an identifier",
                ));
            }
        };

        let handler = match &handler.expr {
            Expr::ExprPath(el) => el.clone(),
            _ => {
                return Err(Error::new(
                    Span::call_site(),
                    "expected handler to be an identifier",
                ))
            }
        };

        Ok(Self {
            route,
            method,
            handler,
        })
    }
}

struct FieldResult {
    address: Address,
    module_path: Option<ExprKeyValue>,
    handlers: Vec<ExprHandler>,
    middlewares: Punctuated<ExprCall, Comma>,
}

enum Address {
    LitStr(LitStr),
    Ident(Ident),
}

fn get_fields(app_struct: AppStruct) -> Result<FieldResult, TokenStream> {
    let module = app_struct
        .fields
        .iter()
        .find(|f| &f.member.to_string() == "module");

    let module_path: Option<ExprKeyValue> = module.map_or(Ok(None), |m| match &m.expr {
        Expr::Module(module_path) => Ok(Some(module_path.clone())),
        _ => {
            return Err(Error::new(
                Span::call_site(),
                "module should be a path to the DI module definition",
            )
            .to_compile_error()
            .to_token_stream())
        }
    })?;

    let handlers = app_struct
        .fields
        .iter()
        .find(|f| &f.member.to_string() == "bind")
        .expect("missing handlers");

    let handlers = match &handlers.expr {
        Expr::ExprArrayHandler(array_expr) => {
            let mut r = vec![];
            array_expr.elements.iter().for_each(|e| r.push(e.clone()));
            r
        }
        _ => {
            return Err(
                Error::new(Span::call_site(), "handlers should be an array literal")
                    .to_compile_error()
                    .into(),
            )
        }
    };

    let address_field = app_struct
        .fields
        .iter()
        .find(|f| &f.member.to_string() == "address")
        .expect("missing handlers");

    let middleware_field = app_struct
        .fields
        .iter()
        .find(|f| &f.member.to_string() == "middleware")
        .expect("missing middleware");

    let middlewares = match &middleware_field.expr {
        Expr::Punctuated(middlewares) => middlewares.clone(),
        _ => {
            return Err(
                Error::new(Span::call_site(), "middleware should be an array literal")
                    .to_compile_error()
                    .into(),
            )
        }
    };

    let address = match &address_field.expr {
        Expr::ExprLit(lit) => match lit.lit.clone() {
            Lit::Str(str) => Address::LitStr(str),
            _ => {
                return Err(Error::new(
                    Span::call_site(),
                    "server address should be a &str literal",
                )
                .to_compile_error()
                .into())
            }
        },
        Expr::Ident(i) => Address::Ident(i.clone()),
        _ => {
            return Err(
                Error::new(Span::call_site(), "server address should be a &str literal")
                    .to_compile_error()
                    .into(),
            )
        }
    };

    Ok(FieldResult {
        address,
        module_path,
        handlers,
        middlewares,
    })
}
