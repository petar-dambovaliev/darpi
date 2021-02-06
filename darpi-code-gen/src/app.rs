use crate::handler::{HAS_NO_PATH_ARGS_PREFIX, HAS_PATH_ARGS_PREFIX, NO_BODY_PREFIX};
use md5;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::ToTokens;
use quote::{format_ident, quote};
use serde::Deserialize;
use serde_json;
use std::cmp::Ordering;
use syn::{parse_str, Error, Expr as SynExpr, ExprCall, ExprLit, ExprPath, Type};

pub(crate) fn make_app(input: TokenStream) -> Result<TokenStream, TokenStream> {
    let input_str = input.to_string();
    let config: Config = match serde_json::from_str(&input_str) {
        Ok(c) => c,
        Err(e) => {
            return Err(Error::new(Span::call_site(), e.to_string())
                .to_compile_error()
                .into());
        }
    };

    let address_value = {
        let av = &config.address;
        let q = quote! {&#av};
        q.to_token_stream()
    };

    if config.handlers.is_empty() {
        return Err(Error::new(Span::call_site(), "no handlers registered")
            .to_compile_error()
            .into());
    }

    let handler_len = config.handlers.len();
    let handlers: Vec<ExprHandler> = config.handlers.into_iter().map(|h| h.into()).collect();

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

    let (module_def, module_let, module_self) = config.container.map_or(Default::default(), |mp| {
        let patj: ExprPath = parse_str(&mp.ttype).unwrap();
        let make_container_func: ExprPath = parse_str(&mp.factory).unwrap();

        (
            quote! {module: std::sync::Arc<#patj>,},
            quote! {let module = std::sync::Arc::new(#make_container_func());},
            quote! {module: module,},
        )
    });

    let (mut middleware_req, mut middleware_res) =
        config.middleware.map_or(Default::default(), |mut middleware| {
            let mut middleware_req = vec![];
            let mut middleware_res = vec![];
            let mut i = 0u16;

            middleware.request.iter().for_each(|e| {
                let e: ExprCall = parse_str(e).unwrap();
                let name = &e.func;
                let m_arg_ident = format_ident!("m_arg_{}", i);
                let mut sorter = 0_u16;

                let m_args: Vec<proc_macro2::TokenStream> = e.args.iter().map(|arg| {
                    if let SynExpr::Call(expr_call) = arg {
                        if expr_call.func.to_token_stream().to_string() == "request" {
                            let index: u16 = expr_call.args.first().unwrap().to_token_stream().to_string().parse().unwrap();
                            sorter += index;
                            let i_ident = format_ident!("m_arg_{}", index);
                            return quote!{#i_ident.clone()};
                        }
                    }
                    quote! {#arg}
                }).collect();

                middleware_req.push((sorter, quote! {
                    let #m_arg_ident = match #name::call(&mut parts, inner_module.clone(), &mut body, #(#m_args ,)*).await {
                        Ok(k) => k,
                        Err(e) => return Ok(e.respond_err()),
                    };
                }));
                i += 1;
            });

            middleware.response.iter_mut().for_each(|e| {
                let mut e: ExprCall = parse_str(e).unwrap();
                let name = &e.func;
                let r_m_arg_ident = format_ident!("res_m_arg_{}", i);
                let mut sorter = 0_u16;

                let m_args: Vec<proc_macro2::TokenStream> = e.args.iter_mut().map(|arg| {
                    if let SynExpr::Call(expr_call) = arg {
                        if expr_call.func.to_token_stream().to_string() == "request" {
                            let index: u16 = expr_call.args.first().unwrap().to_token_stream().to_string().parse().unwrap();
                            let i_ident = format_ident!("m_arg_{}", index);
                            return quote!{#i_ident.clone()};
                        }
                        if expr_call.func.to_token_stream().to_string() == "response" {
                            let index: u16 = expr_call.args.first().unwrap().to_token_stream().to_string().parse().unwrap();
                            sorter += index;
                            return quote!{#r_m_arg_ident.clone()};
                        }
                    }
                    if  let SynExpr::Tuple(tuple) = arg.clone() {
                        let tuple_expr: Vec<proc_macro2::TokenStream> = tuple.elems.iter().map(|tuple_arg| {
                            if let SynExpr::Call(expr_call) = tuple_arg {
                                if expr_call.func.to_token_stream().to_string() == "request" {
                                    let index: u16 = expr_call.args.first().unwrap().to_token_stream().to_string().parse().unwrap();
                                    let i_ident = format_ident!("m_arg_{}", index);
                                    return quote!{#i_ident.clone()};
                                }
                                if expr_call.func.to_token_stream().to_string() == "response" {
                                    let index: u16 = expr_call.args.first().unwrap().to_token_stream().to_string().parse().unwrap();
                                    sorter += index;
                                    return quote!{#r_m_arg_ident.clone()};
                                }
                            }
                            quote! {#tuple_arg}
                        }).collect();
                        return quote! {( #(#tuple_expr ,)* )};
                    }
                    quote! {#arg}
                }).collect();

                middleware_res.push((std::u16::MAX - i - sorter, quote! {
                    let #r_m_arg_ident = match #name::call(&mut rb, inner_module.clone(), #(#m_args ,)* ).await {
                        Ok(k) => k,
                        Err(e) => return Ok(e.respond_err()),
                    };
                }));
                i += 1;
            });

            (
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

                let (send_sync, mut recv_sync): (
                std::sync::mpsc::Sender<fn()>,
                std::sync::mpsc::Receiver<fn()>,
            ) = std::sync::mpsc::channel();
            let sync_job_executor = tokio::task::spawn_blocking(move || loop {
                let job = match recv_sync.recv() {
                    Ok(k) => k,
                    Err(e) => return,
                };
                (job)()
            });

                let (send, mut recv) = tokio::sync::mpsc::unbounded_channel();
                let job_executor = tokio::spawn(async move {
                    loop {
                        let job: Option<BoxFuture<()>> = recv.recv().await;
                        if let Some(job) = job {
                            job.await;
                        }
                    }
                });

                let make_svc = darpi::service::make_service_fn(move |_conn| {
                    let inner_module = std::sync::Arc::clone(&module);
                    let inner_handlers = std::sync::Arc::clone(&handlers);
                    let inner_send = send.clone();
                    async move {
                        Ok::<_, std::convert::Infallible>(darpi::service::service_fn(move |r: darpi::Request<darpi::Body>| {
                            use darpi::futures::FutureExt;
                            use darpi::response::ResponderError;
                            #[allow(unused_imports)]
                            use darpi::RequestMiddleware;
                            #[allow(unused_imports)]
                            use darpi::ResponseMiddleware;
                            use darpi::Handler;
                            let inner_module = std::sync::Arc::clone(&inner_module);
                            let inner_handlers = std::sync::Arc::clone(&inner_handlers);
                            let inner_send = inner_send.clone();
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

                                if let Ok(mut rb) = rb.as_mut() {
                                    #(#middleware_res )*
                                }

                                rb
                            }
                        }))
                    }
                });

                let server = darpi::Server::bind(&address).serve(make_svc);
                let res = async {
                    tokio::join!(job_executor, sync_job_executor, server);
                };
                Ok(res.await)
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
                let mut args = darpi::Args{
                    request_parts: &mut parts,
                    container: inner_module.clone(),
                    body: &mut body,
                    route_args: handler.1.1,
                };
                Handler::call(&#variant_value, &mut args).await
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

#[derive(Debug, Deserialize)]
struct Container {
    factory: String,
    #[serde(alias = "type")]
    ttype: String,
}

#[derive(Debug, Deserialize)]
struct Jobs {
    pub request: Vec<String>,
    pub response: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Middleware {
    pub request: Vec<String>,
    pub response: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Handler {
    pub route: String,
    pub method: String,
    pub handler: String,
}

impl Into<ExprHandler> for Handler {
    fn into(self) -> ExprHandler {
        let route = format!(r#""{}""#, &self.route);

        let route: ExprLit = parse_str(&route).unwrap();
        let method: ExprPath = parse_str(&self.method).unwrap();
        let handler: ExprPath = parse_str(&self.handler).unwrap();

        ExprHandler {
            route,
            method,
            handler,
        }
    }
}

#[derive(Debug, Deserialize)]
struct Config {
    pub address: String,
    pub container: Option<Container>,
    pub jobs: Option<Jobs>,
    pub middleware: Option<Middleware>,
    pub handlers: Vec<Handler>,
}

#[derive(Debug, Clone)]
struct ExprHandler {
    route: ExprLit,
    method: ExprPath,
    handler: ExprPath,
}
