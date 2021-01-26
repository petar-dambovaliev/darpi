use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::ToTokens;
use quote::{format_ident, quote};
use std::collections::HashMap;
use syn::parse_quote::ParseQuote;
use syn::punctuated::Punctuated;
use syn::{
    bracketed, parse::ParseStream, parse_macro_input, token::Bracket, token::Comma, Error, Expr,
    ExprCall, ExprLit, FnArg, GenericArgument, ItemFn, PatType, PathArguments, PathSegment, Type,
    TypePath,
};

#[derive(Debug)]
struct Arguments {
    module: Option<Ident>,
    middleware: Option<Punctuated<ExprCall, Comma>>,
}

impl syn::parse::Parse for Arguments {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let mut module = None;
        let mut middleware = None;

        if !input.is_empty() && !input.peek(Bracket) {
            let f: Option<Ident> = Some(input.parse()?);
            module = f;
        } else if !input.is_empty() {
            let content;
            let _ = bracketed!(content in input);
            let m: Option<Punctuated<ExprCall, Comma>> = Some(Punctuated::parse(&content)?);
            middleware = m;
        }

        if input.peek(Comma) {
            let _: Comma = input.parse()?;
        }

        if !input.is_empty() && module.is_none() && !input.peek(Bracket) {
            let f: Option<Ident> = Some(input.parse()?);
            module = f;
        } else if !input.is_empty() {
            let content;
            let _ = bracketed!(content in input);
            let m: Option<Punctuated<ExprCall, Comma>> = Some(Punctuated::parse(&content)?);
            middleware = m;
        }

        Ok(Arguments { module, middleware })
    }
}

pub(crate) const HAS_PATH_ARGS_PREFIX: &str = "HasPathArgs";
pub(crate) const HAS_NO_PATH_ARGS_PREFIX: &str = "HasNoPathArgs";
pub(crate) const NO_BODY_PREFIX: &str = "NoBody";
pub(crate) const MODULE_PREFIX: &str = "module";

pub(crate) fn make_handler(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(input as ItemFn);

    if func.sig.asyncness.is_none() {
        return Error::new_spanned(func, "Only Async functions can be used as handlers")
            .to_compile_error()
            .into();
    }

    let func_name = &func.sig.ident;
    let module_ident = format_ident!("{}", MODULE_PREFIX);
    let mut make_args = vec![];
    let mut give_args = vec![];
    let mut i = 0_u32;
    let mut n_args = 0u8;
    let mut wants_body = false;
    let has_path_args = format_ident!("{}_{}", HAS_PATH_ARGS_PREFIX, func_name);
    let has_no_path_args = format_ident!("{}_{}", HAS_NO_PATH_ARGS_PREFIX, func_name);
    let mut has_path_args_checker = quote! {impl #has_no_path_args for #func_name {}};
    let mut map = HashMap::new();
    let mut max_middleware_index = None;

    for arg in func.sig.inputs.iter_mut() {
        if let FnArg::Typed(tp) = arg {
            let h_args = match make_handler_args(tp, i, &module_ident) {
                Ok(k) => k,
                Err(e) => return e,
            };
            let (arg_name, method_resolve) = match h_args {
                HandlerArgs::Query(i, ts) => (i, ts),
                HandlerArgs::Body(i, ts) => {
                    wants_body = true;
                    (i, ts)
                }
                HandlerArgs::Path(i, ts) => {
                    n_args += 1;
                    has_path_args_checker = quote! {impl #has_path_args for #func_name {}};
                    (i, ts)
                }
                HandlerArgs::Option(i, ts) => (i, ts),
                HandlerArgs::Module(i, ts) => (i, ts),
                HandlerArgs::Middleware(i, ts, index, ttype) => {
                    if let Some(s) = max_middleware_index {
                        if index > s {
                            max_middleware_index = Some(index);
                        }
                    } else {
                        max_middleware_index = Some(index)
                    }
                    map.insert(index, ttype);
                    (i, ts)
                }
            };

            make_args.push(method_resolve);
            give_args.push(quote! {#arg_name});
            i += 1;
            tp.attrs = Default::default();
        }
    }

    if n_args > 1 {
        return Error::new_spanned(func, "One 1 path type is allowed")
            .to_compile_error()
            .into();
    }

    let mut module = Default::default();
    let mut dummy_t = quote! {,T};
    let mut middleware_req = vec![];
    let mut middleware_res = vec![];
    let mut i = 0u16;
    let mut len_middleware = None;

    if !args.is_empty() {
        let arguments = parse_macro_input!(args as Arguments);
        if let Some(m) = arguments.middleware {
            len_middleware = Some(m.len());
            for e in &m {
                let name = &e.func;
                let m_arg_ident = format_ident!("m_arg_{}", i);
                let r_m_arg_ident = format_ident!("res_m_arg_{}", i);
                let mut sorter = 0_u16;
                let m_args: Vec<proc_macro2::TokenStream> = e
                    .args
                    .iter()
                    .map(|arg| {
                        if let Expr::Call(expr_call) = arg {
                            if expr_call.func.to_token_stream().to_string() == "middleware" {
                                let index: u16 = expr_call
                                    .args
                                    .first()
                                    .unwrap()
                                    .to_token_stream()
                                    .to_string()
                                    .parse()
                                    .unwrap();

                                if index as usize >= m.len() {
                                    return Error::new_spanned(
                                        &func,
                                        "middleware index out of bounds",
                                    )
                                    .to_compile_error()
                                    .into();
                                }

                                sorter += index;
                                let i_ident = format_ident!("m_arg_{}", index);
                                return quote! {#i_ident.clone()};
                            }
                        }
                        quote! {#arg}
                    })
                    .collect();

                middleware_req.push((sorter, quote! {
                    let #m_arg_ident = match #name::call_Request(&mut parts, #(#m_args ,)* #module_ident.clone(), &mut body).await {
                        Ok(k) => k,
                        Err(e) => return Ok(e.respond_err()),
                    };
                }));

                middleware_res.push((std::u16::MAX - i - sorter, quote! {
                    let #r_m_arg_ident = match #name::call_Response(&mut rb, #(#m_args ,)* #module_ident.clone()).await {
                        Ok(k) => k,
                        Err(e) => return Ok(e.respond_err()),
                    };
                }));
                i += 1;
            }
        }
        if let Some(m) = arguments.module {
            module = quote! {#module_ident: std::sync::Arc<#m>};
            dummy_t = Default::default();
        }
    }

    match (len_middleware, max_middleware_index) {
        (Some(l), Some(m)) => {
            if m >= l as u64 {
                return Error::new_spanned(
                    &func,
                    format!("middleware index out of bounds: len [{}] index [{}]", l, m),
                )
                .to_compile_error()
                .into();
            }
        }
        _ => {}
    }

    middleware_req.sort_by(|a, b| a.0.cmp(&b.0));
    middleware_res.sort_by(|a, b| a.0.cmp(&b.0));

    let middleware_req: Vec<proc_macro2::TokenStream> =
        middleware_req.into_iter().map(|e| e.1).collect();
    let middleware_res: Vec<proc_macro2::TokenStream> =
        middleware_res.into_iter().map(|e| e.1).collect();

    let no_body = format_ident!("{}_{}", NO_BODY_PREFIX, func_name);
    let mut body_checker = proc_macro2::TokenStream::new();

    if !wants_body {
        body_checker = quote! {
            impl #no_body for #func_name {}
        };
    }

    let func_copy = func.clone();

    let mut dummy_where = Default::default();
    if !dummy_t.is_empty() {
        module = quote! {#module_ident: std::sync::Arc<T>};
        dummy_where = quote! {
        where
        T: 'static + Sync + Send,
        };
    }

    let module_ident = if !module.is_empty() && dummy_t.is_empty() {
        quote! {#module_ident.clone()}
    } else {
        quote! {#module_ident.clone()}
    };

    let fn_call = quote! {
        pub(crate) async fn call<'a#dummy_t>(
            mut parts: darpi::RequestParts,
            mut body: darpi::Body,
            (req_route, req_args): (darpi::ReqRoute<'a>, std::collections::HashMap<&'a str, &'a str>), #module ) -> Result<darpi::Response<darpi::Body>, std::convert::Infallible> #dummy_where {
               use darpi::response::Responder;
               #[allow(unused_imports)]
               use shaku::HasComponent;
               #[allow(unused_imports)]
               use darpi::request::FromQuery;
               use darpi::request::FromRequestBody;
               use darpi::response::ResponderError;

                #(#middleware_req )*

               #(#make_args )*

               let mut rb = Self::#func_name(#(#give_args ,)*).await.respond();

               #(#middleware_res )*
                Ok(rb)
        }
    };

    let output = quote! {
        #[allow(non_camel_case_types, missing_docs)]
        trait #has_path_args {}
        #[allow(non_camel_case_types, missing_docs)]
        trait #has_no_path_args {}
        #[allow(non_camel_case_types, missing_docs)]
        trait #no_body {}
        #[allow(non_camel_case_types, missing_docs)]
        pub struct #func_name;
        #body_checker
        #has_path_args_checker
        impl #func_name {
           #fn_call
           //user defined function
           #func_copy
       }
    };
    //panic!("{}", output.to_string());
    output.into()
}

fn make_optional_query(arg_name: &Ident, last: &PathSegment) -> proc_macro2::TokenStream {
    let inner = &last.arguments;
    quote! {
        let #arg_name: #last = match &parts.uri.query() {
            Some(q) => {
                let #arg_name: #last = match #inner::from_query(q) {
                    Ok(w) => Some(w),
                    Err(w) => None
                };
                #arg_name
            }
            None => None,
        };
    }
}

fn make_query(arg_name: &Ident, last: &PathSegment) -> proc_macro2::TokenStream {
    let respond_err = make_respond_err(
        quote! {respond_to_err},
        quote! {darpi::request::QueryPayloadError},
    );
    quote! {
        #respond_err
        let #arg_name = match parts.uri.query() {
            Some(q) => q,
            None => return Ok(respond_to_err::<#last>(darpi::request::QueryPayloadError::NotExist))
        };

        let #arg_name: #last = match #last::from_query(#arg_name) {
            Ok(q) => q,
            Err(e) => return Ok(respond_to_err::<#last>(e))
        };
    }
}

fn make_respond_err(
    name: proc_macro2::TokenStream,
    err_path: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        fn #name<T>(e: #err_path) -> darpi::Response<darpi::Body>
        where
            T: darpi::response::ErrResponder<#err_path, darpi::Body>,
        {
            T::respond_err(e)
        }
    }
}

fn make_path_args(arg_name: &Ident, last: &PathSegment) -> proc_macro2::TokenStream {
    let respond_err = make_respond_err(
        quote! {respond_to_path_err},
        quote! {darpi::request::PathError},
    );
    quote! {
        #respond_err
        let json_args = match darpi::serde_json::to_string(&req_args) {
            Ok(k) => k,
            Err(e) => {
                return Ok(respond_to_path_err::<#last>(
                    darpi::request::PathError::Deserialize(e.to_string()),
                ))
            }
        };
        let #arg_name: #last = match darpi::serde_json::from_str(&json_args) {
            Ok(k) => k,
            Err(e) => {
                return Ok(respond_to_path_err::<#last>(
                    darpi::request::PathError::Deserialize(e.to_string()),
                ))
            }
        };
    }
}

fn make_json_body(arg_name: &Ident, path: &TypePath) -> proc_macro2::TokenStream {
    let mut format = path.path.segments.clone();
    format
        .iter_mut()
        .for_each(|s| s.arguments = Default::default());

    let inner = &path.path.segments.last().unwrap().arguments;

    let output = quote! {
        match #format::#inner::assert_content_type(parts.headers.get("content-type")).await {
            Ok(()) => {}
            Err(e) => return Ok(e.respond_err()),
        }

        let #arg_name: #path = match #format::extract(body).await {
            Ok(q) => q,
            Err(e) => return Ok(e.respond_err())
        };
    };
    output
}

enum HandlerArgs {
    Query(Ident, proc_macro2::TokenStream),
    Body(Ident, proc_macro2::TokenStream),
    Path(Ident, proc_macro2::TokenStream),
    Option(Ident, proc_macro2::TokenStream),
    Module(Ident, proc_macro2::TokenStream),
    Middleware(Ident, proc_macro2::TokenStream, u64, Type),
}

fn make_handler_args(
    tp: &PatType,
    i: u32,
    module_ident: &Ident,
) -> Result<HandlerArgs, TokenStream> {
    let ttype = &tp.ty;

    let arg_name = format_ident!("arg_{:x}", i);
    if tp.attrs.len() != 1 {
        return Err(
            Error::new(Span::call_site(), format!("expected 1 attribute macro"))
                .to_compile_error()
                .into(),
        );
    }

    let attr = tp.attrs.first().unwrap();

    if let Type::Path(tp) = *ttype.clone() {
        let last = tp.path.segments.last().unwrap();
        let attr_ident = attr.path.get_ident().unwrap();

        //todo return err if there are more than 1 query args
        if attr_ident == "query" {
            if last.ident == "Option" {
                if let PathArguments::AngleBracketed(ab) = &last.arguments {
                    if let GenericArgument::Type(t) = ab.args.first().unwrap() {
                        if let Type::Path(_) = t {
                            let res = make_optional_query(&arg_name, last);
                            return Ok(HandlerArgs::Option(arg_name, res));
                        }
                    }
                }
            }

            let res = make_query(&arg_name, last);
            return Ok(HandlerArgs::Query(arg_name, res));
        }
        //todo return err if there are more than 1 json args
        if attr_ident == "body" {
            let res = make_json_body(&arg_name, &tp);
            return Ok(HandlerArgs::Body(arg_name, res));
        }
        //todo return err if there are more than 1 path args
        if attr_ident == "path" {
            let res = make_path_args(&arg_name, &last);
            return Ok(HandlerArgs::Path(arg_name, res));
        }

        if attr_ident == "inject" {
            let method_resolve = quote! {
                let #arg_name: #ttype = #module_ident.resolve();
            };
            return Ok(HandlerArgs::Module(arg_name, method_resolve));
        }

        if attr_ident == "middleware" {
            let index: ExprLit = match attr.parse_args() {
                Ok(el) => el,
                Err(_) => {
                    return Err(Error::new(Span::call_site(), format!("missing index"))
                        .to_compile_error()
                        .into())
                }
            };

            let index = match index.lit {
                syn::Lit::Int(i) => {
                    let value = match i.base10_parse::<u64>() {
                        Ok(k) => k,
                        Err(_) => {
                            return Err(Error::new(Span::call_site(), format!("invalid index"))
                                .to_compile_error()
                                .into())
                        }
                    };
                    value
                }
                _ => {
                    return Err(Error::new(Span::call_site(), format!("invalid index"))
                        .to_compile_error()
                        .into())
                }
            };

            let m_arg_ident = format_ident!("m_arg_{}", index);
            let method_resolve = quote! {
                let #arg_name: #ttype = #m_arg_ident;
            };
            return Ok(HandlerArgs::Middleware(
                arg_name,
                method_resolve,
                index,
                *ttype.clone(),
            ));
        }
    }
    Err(Error::new(
        Span::call_site(),
        format!("unsupported type {}", ttype.to_token_stream().to_string()),
    )
    .to_compile_error()
    .into())
}

// pub(crate) fn expand_middlewares_impl(
//     container: &Option<Ident>,
//     handler_name: &Ident,
//     mut p: Punctuated<ExprCall, Comma>,
//     map: HashMap<u64, Type>,
// ) -> Vec<proc_macro2::TokenStream> {
//     let mut middleware_impl = vec![];
//     let mut i = 0_u64;
//
//     p.iter_mut().for_each(|e| {
//         let name = &e.func;
//         let handler_name_request = format_ident!("{}_request", handler_name);
//         let handler_name_response = format_ident!("{}_response", handler_name);
//         let args: Vec<proc_macro2::TokenStream> = e.args.iter().map(|arg| {
//             if let Expr::Call(expr_call) = arg {
//                 if expr_call.func.to_token_stream().to_string() == "middleware" {
//                     let i_ident = format_ident!("m_arg_{}", expr_call.args.first().unwrap().to_token_stream().to_string());
//                     return quote!{#i_ident};
//                 }
//             }
//             quote! {#arg}
//         }).collect();
//
//         let (where_clause, dummy_gen, def_c, give_c) = container.as_ref().map_or((quote!{
//         where
//         T: 'static + Sync + Send,
//         }, quote!{<T>}, quote!{c: std::sync::Arc<T>,}, quote!{c}), |c| (Default::default(), Default::default(), quote!{c: std::sync::Arc<#c>,}, quote!{c}));
//
//         let dummy_trait = format_ident!("{}_{}_trait", handler_name, name.to_token_stream().to_string());
//
//         let (ttype, ok) = match map.get(&i) {
//             Some(t) => (t.clone(), quote!{Ok(t) => Ok(t)}),
//             None => (Type::Tuple(TypeTuple{ paren_token: Default::default(), elems: Default::default() }), quote!{Ok(_) => Ok(())})
//         };
//
//         let q = quote! {
//             #[async_trait::async_trait]
//             #[allow(non_camel_case_types, missing_docs)]
//             trait #dummy_trait {
//                 async fn #handler_name_request #dummy_gen (#def_c p: &mut darpi::RequestParts, b: &mut darpi::Body) -> Result<#ttype, darpi::Response<darpi::Body>> #where_clause;
//                 async fn #handler_name_response #dummy_gen (#def_c r: &mut darpi::Response<darpi::Body>) -> Result<(), darpi::Response<darpi::Body>> #where_clause;
//             }
//             #[async_trait::async_trait]
//             #[allow(non_camel_case_types, missing_docs)]
//             impl #dummy_trait for #name {
//                 async fn #handler_name_request #dummy_gen (#def_c p: &mut darpi::RequestParts, mut b: &mut darpi::Body) -> Result<#ttype, darpi::Response<darpi::Body>> #where_clause{
//                     use darpi::response::ResponderError;
//                     let concrete = #name::call_Request(p, #(#args ,)*  #give_c, &mut b).await;
//
//                     match concrete {
//                         #ok,
//                         Err(e) => Err(e.respond_err())
//                     }
//                 }
//                 async fn #handler_name_response #dummy_gen (#def_c r: &mut darpi::Response<darpi::Body>) -> Result<(), darpi::Response<darpi::Body>> #where_clause {
//                     use darpi::response::ResponderError;
//                     let concrete = #name::call_Response(r, #(#args ,)* #give_c).await;
//                     match concrete {
//                         Ok(_) => Ok(()),
//                         Err(e) => Err(e.respond_err())
//                     }
//                 }
//             }
//         };
//
//         middleware_impl.push(q);
//         i += 1;
//     });
//     middleware_impl
// }
