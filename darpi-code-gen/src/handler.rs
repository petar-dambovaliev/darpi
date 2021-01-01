use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::export::ToTokens;
use syn::{
    bracketed, parse::ParseStream, parse_macro_input, token::Bracket, token::Comma, Error, FnArg,
    GenericArgument, ItemFn, PatType, Path, PathArguments, PathSegment, Type,
};

use crate::app::ExprKeyValue;
use syn::parse_quote::ParseQuote;
use syn::punctuated::Punctuated;

struct Methods {
    methods: Punctuated<ExprKeyValue, Comma>,
}

impl syn::parse::Parse for Methods {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let methods: Punctuated<ExprKeyValue, Comma> = if input.peek(Bracket) {
            let content;
            let _ = bracketed!(content in input);
            Punctuated::parse(&content)?
        } else {
            let mut p: Punctuated<ExprKeyValue, Comma> = Punctuated::new();
            let expr: ExprKeyValue = input.parse()?;
            p.push(expr);
            p
        };

        Ok(Methods { methods })
    }
}

pub(crate) const HAS_PATH_ARGS_PREFIX: &str = "HasPathArgs";
pub(crate) const HAS_NO_PATH_ARGS_PREFIX: &str = "HasNoPathArgs";
pub(crate) const NO_BODY_PREFIX: &str = "NoBody";
pub(crate) const MODULE_PREFIX: &str = "module";

fn implement_getters(methods: Methods, name: &Ident) -> proc_macro2::TokenStream {
    let mut res = vec![];

    for m in methods.methods {
        let return_type = m.key.clone();
        let return_val = m.value.clone();
        let func_name = format_ident!(
            "{}",
            return_type.to_token_stream().to_string().replace("::", "")
        );

        res.push(quote! {
            fn #func_name() -> #return_type {
                #return_val
            }
        });
    }

    quote! {
        impl #name {
             #(#res )*
        }
    }
}

pub(crate) fn make_handler(args: TokenStream, input: TokenStream) -> TokenStream {
    let func = parse_macro_input!(input as ItemFn);

    if func.sig.asyncness.is_none() {
        return Error::new_spanned(func, "Only Async functions can be used as handlers")
            .to_compile_error()
            .into();
    }

    let func_name = &func.sig.ident;
    let mut impl_getters = proc_macro2::TokenStream::new();

    if !args.is_empty() {
        let methods = parse_macro_input!(args as Methods);
        impl_getters = implement_getters(methods, func_name);
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

    let func_copy = func.clone();
    let mut make_args = vec![];
    let mut give_args = vec![];
    let mut module_full_req = vec![];
    let mut i = 0_u32;
    let mut n_args = 0u8;
    let has_path_args = format_ident!("{}_{}", HAS_PATH_ARGS_PREFIX, func_name);
    let has_no_path_args = format_ident!("{}_{}", HAS_NO_PATH_ARGS_PREFIX, func_name);
    let mut has_path_args_checker = quote! {impl #has_no_path_args for #func_name {}};
    let mut fn_call_generic = quote! {,T};
    let mut fn_call_module_args = vec![];

    func.sig.inputs.iter().for_each(|arg| {
        if let FnArg::Typed(tp) = arg {
            let module_ident = format_ident!("{}_{}", MODULE_PREFIX, i);
            //todo change the return type to enum
            //todo add nopath trait to check from handler if path arg is not used
            let (arg_name, method_resolve) = match make_handler_args(tp, i, &module_ident) {
                HandlerArgs::Query(i, ts) => (i, ts),
                HandlerArgs::Json(i, ts) => (i, ts),
                HandlerArgs::Path(i, ts) => {
                    n_args += 1;
                    has_path_args_checker = quote! {impl #has_path_args for #func_name {}};
                    (i, ts)
                },
                HandlerArgs::Option(i, ts) => (i, ts),
                HandlerArgs::Module(i, ts) => {
                    if let Type::Path(tp) = *tp.ty.clone() {
                        let segment = tp.path.segments.first().unwrap();
                        if let PathArguments::AngleBracketed(ab) = &segment.arguments {
                            let user_type = &ab.args;
                            module_full_req.push(quote! {shaku::HasComponent<#user_type + 'static>});
                            fn_call_module_args.push(quote! {#module_ident: std::sync::Arc<impl shaku::HasComponent<#user_type + 'static>>});
                            fn_call_generic = Default::default();
                        }
                    }
                    (i, ts)
                },
            };

            make_args.push(method_resolve);
            give_args.push(quote! {#arg_name});
            i += 1;
        }
    });

    if n_args > 1 {
        return Error::new_spanned(func, "One 1 path type is allowed")
            .to_compile_error()
            .into();
    }

    if fn_call_module_args.is_empty() {
        fn_call_module_args.push(quote! {m: std::sync::Arc<T>});
    }

    let fn_call = quote! {
        async fn call<'a #fn_call_generic>(
            r: darpi::Request<darpi::Body>,
            (req_route, req_args): (darpi::ReqRoute<'a>, std::collections::HashMap<&'a str, &'a str>),
            #(#fn_call_module_args ,)*) -> Result<darpi::Response<darpi::Body>, std::convert::Infallible> {
               use darpi::response::Responder;

               #(#make_args )*
               Ok(async {
                    Self::#func_name(#(#give_args ,)*).await.respond()
               }.await)
        }
    };

    let mut inject_args = vec![];
    module_full_req
        .iter()
        .for_each(|_| inject_args.push(quote! {m.clone()}));

    let mut fn_expand_where = proc_macro2::TokenStream::new();
    let mut fn_expand_self_call = quote! {Self::call(r, (req_route, req_args), m).await};

    if !module_full_req.is_empty() {
        fn_expand_where = quote! {where T: #(#module_full_req +)*};
        fn_expand_self_call = quote! {Self::call(r, (req_route, req_args), #(#inject_args),*).await}
    }

    let fn_expand_call = quote! {
        #[inline]
            async fn expand_call<'a, T>(r: darpi::Request<darpi::Body>, (req_route, req_args): (darpi::ReqRoute<'a>, std::collections::HashMap<&'a str, &'a str>), m: std::sync::Arc<T>) -> Result<darpi::Response<darpi::Body>, std::convert::Infallible>
            #fn_expand_where {
                #fn_expand_self_call
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
           #fn_expand_call
       }
       #impl_getters
    };
    //panic!("{}", output.to_string());
    output.into()
}

fn make_optional_query(arg_name: &Ident, last: &PathSegment) -> proc_macro2::TokenStream {
    quote! {
        let #arg_name: #last = match r.uri().query() {
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
        let #arg_name = match r.uri().query() {
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
    let inner = &last.arguments;
    let respond_err = make_respond_err(
        quote! {respond_to_path_err},
        quote! {darpi::request::PathError},
    );
    quote! {
        #respond_err
        let json_args = match darpi::serde_json::to_string(&req_args) {
            Ok(k) => k,
            Err(e) => {
                return Ok(respond_to_path_err::#inner(
                    darpi::request::PathError::Deserialize(e.to_string()),
                ))
            }
        };
        let #arg_name: #last = match darpi::serde_json::from_str(&json_args) {
            Ok(k) => k,
            Err(e) => {
                return Ok(respond_to_path_err::#inner(
                    darpi::request::PathError::Deserialize(e.to_string()),
                ))
            }
        };
    }
}

fn make_json_body(
    arg_name: &Ident,
    path: &Path,
    inner: &PathArguments,
) -> proc_macro2::TokenStream {
    let output = quote! {
        use darpi::request::FromRequestBody;
        use darpi::response::ResponderError;
        let (_, body) = r.into_parts();

        if let Err(e) = Json::#inner::assert_content_size(&body) {
            return Ok(e.respond_err());
        }

        let #arg_name: #path = match Json::#inner::extract(body).await {
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

fn make_handler_args(tp: &PatType, i: u32, module_ident: &Ident) -> HandlerArgs {
    let ttype = &tp.ty;

    let arg_name = format_ident!("arg_{:x}", i);

    if let Type::Path(tp) = *ttype.clone() {
        let last = tp.path.segments.last().unwrap();
        if last.ident == "Query" {
            let res = make_query(&arg_name, last);
            return HandlerArgs::Query(arg_name, res);
        }
        if last.ident == "Json" {
            let res = make_json_body(&arg_name, &tp.path, &last.arguments);
            return HandlerArgs::Json(arg_name, res);
        }

        if last.ident == "Path" {
            let res = make_path_args(&arg_name, &last);
            return HandlerArgs::Path(arg_name, res);
        }

        if last.ident == "Option" {
            if let PathArguments::AngleBracketed(ab) = &last.arguments {
                if let GenericArgument::Type(t) = ab.args.first().unwrap() {
                    if let Type::Path(tp) = t {
                        let first = tp.path.segments.first().unwrap();
                        if first.ident == "Query" {
                            let res = make_optional_query(&arg_name, last);
                            return HandlerArgs::Option(arg_name, res);
                        }
                    }
                }
            }
        }
    }

    let method_resolve = quote! {
        let #arg_name: #ttype = #module_ident.resolve();
    };
    HandlerArgs::Module(arg_name, method_resolve)
}
