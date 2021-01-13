use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::ToTokens;
use quote::{format_ident, quote};
use syn::parse_quote::ParseQuote;
use syn::punctuated::Punctuated;
use syn::{
    bracketed, parse::ParseStream, parse_macro_input, token::Bracket, token::Comma, Error,
    ExprCall, FnArg, GenericArgument, ItemFn, PatType, Path, PathArguments, PathSegment, Type,
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

fn expand_middlewares_impl(
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
            quote! {Expect(#arg)}
        }).collect();

        let (def_c, give_c) = container.as_ref().map_or(Default::default(), |c| (quote!{c: std::sync::Arc<#c>,}, quote!{c}));

        let q = quote! {
            impl #name {
                async fn #handler_name_request(#def_c p: &darpi::RequestParts, b: &darpi::Body) -> Result<(), impl darpi::response::ResponderError> {
                    #name::call_Request(p, #(#args ,)*  #give_c, b).await
                }
                async fn #handler_name_response(#def_c r: &darpi::Response<darpi::Body>) -> Result<(), impl darpi::response::ResponderError> {
                    #name::call_Response(r, #(#args ,)* #give_c).await
                }
            }
        };

        middleware_impl.push(q);
    });
    middleware_impl
}

pub(crate) fn make_handler(args: TokenStream, input: TokenStream) -> TokenStream {
    let func = parse_macro_input!(input as ItemFn);

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
                        return Ok(e.respond_err());
                    }
                });

                middleware_res.push(quote! {
                    if let Err(e) = #name::#h_res(#module_ident.clone(), &rb).await {
                        return Ok(e.respond_err());
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

    let func_copy = func.clone();
    let mut make_args = vec![];
    let mut give_args = vec![];
    let mut i = 0_u32;
    let mut n_args = 0u8;
    let has_path_args = format_ident!("{}_{}", HAS_PATH_ARGS_PREFIX, func_name);
    let has_no_path_args = format_ident!("{}_{}", HAS_NO_PATH_ARGS_PREFIX, func_name);
    let mut has_path_args_checker = quote! {impl #has_no_path_args for #func_name {}};

    func.sig.inputs.iter().for_each(|arg| {
        if let FnArg::Typed(tp) = arg {
            let (arg_name, method_resolve) = match make_handler_args(tp, i, &module_ident) {
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
        }
    });

    if n_args > 1 {
        return Error::new_spanned(func, "One 1 path type is allowed")
            .to_compile_error()
            .into();
    }

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

    if !dummy_t.is_empty() {
        module = quote! {_: std::sync::Arc<T>};
    }

    let module_ident = if !module.is_empty() && dummy_t.is_empty() {
        quote! {#module_ident.clone()}
    } else {
        Default::default()
    };

    let fn_expand_call = quote! {
        #[inline]
        async fn expand_call<'a#dummy_t>(r: darpi::Request<darpi::Body>, (req_route, req_args): (darpi::ReqRoute<'a>, std::collections::HashMap<&'a str, &'a str>), #module) -> Result<darpi::Response<darpi::Body>, std::convert::Infallible> {
            let (parts, body) = r.into_parts();
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

        let #arg_name: #path = match #inner::extract(body).await {
            Ok(q) => ExtractBody(q),
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
        //todo return err if there are more than 1 query args
        if last.ident == "Query" {
            let res = make_query(&arg_name, last);
            return HandlerArgs::Query(arg_name, res);
        }
        //todo return err if there are more than 1 json args
        if last.ident == "ExtractBody" {
            let res = make_json_body(&arg_name, &tp.path, &last.arguments);
            return HandlerArgs::Json(arg_name, res);
        }
        //todo return err if there are more than 1 path args
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
