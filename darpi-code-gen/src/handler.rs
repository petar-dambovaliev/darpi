use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::ToTokens;
use quote::{format_ident, quote};
use syn::parse_quote::ParseQuote;
use syn::punctuated::Punctuated;
use syn::{
    bracketed, parse::ParseStream, parse_macro_input, token::Bracket, token::Comma, Error,
    ExprCall, FnArg, GenericArgument, ItemFn, PatType, PathArguments, PathSegment, Type, TypePath,
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

pub(crate) fn expand_middlewares_impl(
    container: &Option<Ident>,
    handler_name: &Ident,
    mut p: Punctuated<ExprCall, Comma>,
) -> Vec<proc_macro2::TokenStream> {
    let mut middleware_impl = vec![];
    p.iter_mut().for_each(|e| {
        let name = &e.func;
        let handler_name_request = format_ident!("{}_request", handler_name);
        let handler_name_response = format_ident!("{}_response", handler_name);
        let args: Vec<proc_macro2::TokenStream> = e.args.iter().map(|arg| {
            quote! {#arg}
        }).collect();

        let (where_clause, dummy_gen, def_c, give_c) = container.as_ref().map_or((quote!{
        where
        T: 'static + Sync + Send,
        }, quote!{<T>}, quote!{c: std::sync::Arc<T>,}, quote!{c}), |c| (Default::default(), Default::default(), quote!{c: std::sync::Arc<#c>,}, quote!{c}));

        let dummy_trait = format_ident!("{}_{}_trait", handler_name, name.to_token_stream().to_string());
        let q = quote! {
            #[async_trait::async_trait]
            #[allow(non_camel_case_types, missing_docs)]
            trait #dummy_trait {
                async fn #handler_name_request #dummy_gen (#def_c p: &darpi::RequestParts, b: &darpi::Body) -> Result<(), darpi::Response<darpi::Body>> #where_clause;
                async fn #handler_name_response #dummy_gen (#def_c r: &darpi::Response<darpi::Body>) -> Result<(), darpi::Response<darpi::Body>> #where_clause;
            }
            #[async_trait::async_trait]
            #[allow(non_camel_case_types, missing_docs)]
            impl #dummy_trait for #name {
                async fn #handler_name_request #dummy_gen (#def_c p: &darpi::RequestParts, b: &darpi::Body) -> Result<(), darpi::Response<darpi::Body>> #where_clause{
                    use darpi::response::ResponderError;
                    let concrete = #name::call_Request(p, #(#args ,)*  #give_c, b).await;

                    match concrete {
                        Ok(()) => Ok(()),
                        Err(e) => Err(e.respond_err())
                    }
                }
                async fn #handler_name_response #dummy_gen (#def_c r: &darpi::Response<darpi::Body>) -> Result<(), darpi::Response<darpi::Body>> #where_clause {
                    use darpi::response::ResponderError;
                    let concrete = #name::call_Response(r, #(#args ,)* #give_c).await;
                    match concrete {
                        Ok(()) => Ok(()),
                        Err(e) => Err(e.respond_err())
                    }
                }
            }
        };

        middleware_impl.push(q);
    });
    middleware_impl
}

pub(crate) fn make_handler(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(input as ItemFn);

    if func.sig.asyncness.is_none() {
        return Error::new_spanned(func, "Only Async functions can be used as handlers")
            .to_compile_error()
            .into();
    }

    let func_name = &func.sig.ident;
    let mut module = Default::default();
    let mut dummy_t = quote! {,T};
    let module_ident = format_ident!("{}", MODULE_PREFIX);
    let mut middlewares_impl = Default::default();
    let mut middleware_req = vec![];
    let mut middleware_res = vec![];

    if !args.is_empty() {
        let arguments = parse_macro_input!(args as Arguments);
        if let Some(m) = arguments.middleware {
            middlewares_impl = expand_middlewares_impl(&arguments.module, func_name, m.clone());

            let h_req = format_ident!("{}_request", func_name);
            let h_res = format_ident!("{}_response", func_name);
            m.iter().for_each(|e| {
                let name = &e.func;

                middleware_req.push(quote! {
                    if let Err(e) = #name::#h_req(#module_ident.clone(), &parts, &body).await {
                        return Ok(e);
                    }
                });

                middleware_res.push(quote! {
                    if let Err(e) = #name::#h_res(#module_ident.clone(), &rb).await {
                        return Ok(e);
                    }
                });
            });
        }
        if let Some(m) = arguments.module {
            module = quote! {#module_ident: std::sync::Arc<#m>};
            dummy_t = Default::default();
        }
    }

    let no_body = format_ident!("{}_{}", NO_BODY_PREFIX, func_name);
    let mut body_checker = proc_macro2::TokenStream::new();
    let mut wants_body = false;

    func.sig.inputs.iter().for_each(|arg| {
        if arg.to_token_stream().to_string().contains("Json") {
            wants_body = true;
        }
    });

    if !wants_body {
        body_checker = quote! {
            impl #no_body for #func_name {}
        };
    }

    let mut make_args = vec![];
    let mut give_args = vec![];
    let mut i = 0_u32;
    let mut n_args = 0u8;
    let has_path_args = format_ident!("{}_{}", HAS_PATH_ARGS_PREFIX, func_name);
    let has_no_path_args = format_ident!("{}_{}", HAS_NO_PATH_ARGS_PREFIX, func_name);
    let mut has_path_args_checker = quote! {impl #has_no_path_args for #func_name {}};

    for arg in func.sig.inputs.iter_mut() {
        if let FnArg::Typed(tp) = arg {
            let h_args = match make_handler_args(tp, i, &module_ident) {
                Ok(k) => k,
                Err(e) => return e,
            };
            let (arg_name, method_resolve) = match h_args {
                HandlerArgs::Query(i, ts) => (i, ts),
                HandlerArgs::Json(i, ts) => (i, ts),
                HandlerArgs::Path(i, ts) => {
                    n_args += 1;
                    has_path_args_checker = quote! {impl #has_path_args for #func_name {}};
                    (i, ts)
                }
                HandlerArgs::Option(i, ts) => (i, ts),
                HandlerArgs::Module(i, ts) => (i, ts),
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

    let func_copy = func.clone();

    let fn_call = quote! {
        async fn call<'a>(
            parts: darpi::RequestParts,
            body: darpi::Body,
            (req_route, req_args): (darpi::ReqRoute<'a>, std::collections::HashMap<&'a str, &'a str>), #module ) -> Result<darpi::Response<darpi::Body>, std::convert::Infallible> {
               use darpi::response::Responder;
               #[allow(unused_imports)]
               use shaku::HasComponent;

               #(#make_args )*
               Ok(async {
                    Self::#func_name(#(#give_args ,)*).await.respond()
               }.await)
        }
    };

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
        Default::default()
    };

    let fn_expand_call = quote! {
        #[inline]
        async fn expand_call<'a#dummy_t>(parts: darpi::RequestParts, body: darpi::Body, (req_route, req_args): (darpi::ReqRoute<'a>, std::collections::HashMap<&'a str, &'a str>), #module) -> Result<darpi::Response<darpi::Body>, std::convert::Infallible> #dummy_where {
            #(#middleware_req )*
            let rb = Self::call(parts, body, (req_route, req_args), #module_ident).await.unwrap();
            #(#middleware_res )*
            Ok(rb)
        }
    };

    let output = quote! {
        #[allow(non_camel_case_types, missing_docs)]
        #(#middlewares_impl )*
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
           #fn_expand_call
       }
    };
    //panic!("{}", output.to_string());
    output.into()
}

fn make_optional_query(arg_name: &Ident, last: &PathSegment) -> proc_macro2::TokenStream {
    quote! {
        let #arg_name: #last = match &parts.uri.query() {
            Some(q) => {
                let #arg_name: #last = match Query::from_query(q) {
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
    let inner = &last.arguments;
    let respond_err = make_respond_err(
        quote! {respond_to_err},
        quote! {darpi::request::QueryPayloadError},
    );
    quote! {
        #respond_err
        let #arg_name = match &parts.uri.query() {
            Some(q) => q,
            None => return Ok(respond_to_err::#inner(darpi::request::QueryPayloadError::NotExist))
        };

        let #arg_name: #last = match Query::from_query(#arg_name) {
            Ok(q) => q,
            Err(e) => return Ok(respond_to_err::#inner(e))
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
        use darpi::request::FromRequestBody;
        use darpi::response::ResponderError;

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
    Json(Ident, proc_macro2::TokenStream),
    Path(Ident, proc_macro2::TokenStream),
    Option(Ident, proc_macro2::TokenStream),
    Module(Ident, proc_macro2::TokenStream),
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
                        if let Type::Path(tp) = t {
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
            return Ok(HandlerArgs::Json(arg_name, res));
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
    }
    Err(Error::new(
        Span::call_site(),
        format!("unsupported type {}", ttype.to_token_stream().to_string()),
    )
    .to_compile_error()
    .into())
}
