use crate::handler::MODULE_PREFIX;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::ToTokens;
use quote::{format_ident, quote};
use syn::parse_quote::ParseQuote;
use syn::punctuated::Punctuated;
use syn::{
    bracketed, parse::ParseStream, parse_macro_input, parse_str, token::Colon2, token::Comma,
    AttributeArgs, Error, ExprCall, FnArg, ItemFn, PatType, Path, PathArguments, PathSegment,
    ReturnType, Type,
};

#[derive(Debug)]
struct Arguments {
    middleware: Punctuated<ExprCall, Comma>,
}

impl syn::parse::Parse for Arguments {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let content;
        let _ = bracketed!(content in input);
        let middleware: Punctuated<ExprCall, Comma> = Punctuated::parse(&content)?;

        Ok(Arguments { middleware })
    }
}

fn get_return_type(r: &ReturnType) -> proc_macro2::TokenStream {
    if let ReturnType::Type(_, t) = r.clone() {
        if let Type::Path(tp) = *t {
            let last = tp.path.segments.last().unwrap();
            if &last.ident != "Result" {
                return Error::new_spanned(r, "Only Result return type supported")
                    .to_compile_error()
                    .into();
            }
            if let PathArguments::AngleBracketed(ab) = &last.arguments {
                return ab
                    .args
                    .last()
                    .expect("missing error generic argument")
                    .to_token_stream();
            }
        }
    }

    Error::new_spanned(r, "Invalid return type")
        .to_compile_error()
        .into()
}

pub(crate) fn make_middleware(args: TokenStream, input: TokenStream) -> TokenStream {
    let func = parse_macro_input!(input as ItemFn);
    if func.sig.asyncness.is_none() {
        return Error::new_spanned(func, "Only Async functions can be used as handlers")
            .to_compile_error()
            .into();
    }

    let args = parse_macro_input!(args as AttributeArgs);

    if args.len() != 1 {
        return Error::new_spanned(func, format!("Expected 1 argument, {} given. Accepted arguments are middlewares type `Request` or `Response`", args.len()))
            .to_compile_error()
            .into();
    }

    let first_arg = args
        .first()
        .expect("this cannot happen")
        .to_token_stream()
        .to_string();

    let (_, arg_type) = match first_arg.as_str() {
        "Request" => ("RequestMiddleware", "RequestParts"),
        "Response" => ("ResponseMiddleware", "Response<Body>"),
        _ => {
            return Error::new_spanned(
                func,
                format!(
                    "Accepted arguments are middlewares type `Request` or `Response`, `{}` given",
                    first_arg
                ),
            )
            .to_compile_error()
            .into();
        }
    };

    if func.sig.inputs.is_empty() {
        return Error::new_spanned(func, format!("{} is a mandatory argument", arg_type))
            .to_compile_error()
            .into();
    }

    let err_type = get_return_type(&func.sig.output)
        .to_token_stream()
        .to_string();

    let err_ident = format_ident!("{}", err_type);
    let name = func.sig.ident.clone();

    let mut make_args = vec![];
    let mut give_args = vec![];
    let mut i = 0_u32;
    let fn_call_module_where = quote! { where T: };
    let mut where_segments = vec![];
    let mut fn_call_module_args = vec![];
    //todo create default T when container is not needed
    let module_ident = format_ident!("{}", MODULE_PREFIX);

    func.sig.inputs.iter().for_each(|arg| {
        if let FnArg::Typed(tp) = arg {
            //todo change the return type to enum
            //todo add nopath trait to check from handler if path arg is not used
            let (arg_name, method_resolve) = match make_handler_args(tp, i, &module_ident) {
                HandlerArg::Permanent(i, ts) => (i, ts),
                HandlerArg::Expect(id, ttype, ts) => {
                    let cg = format_ident!("T{}", i);
                    fn_call_module_args.push(quote! {#cg: Expect<#ttype>});

                    (id, ts)
                }
                HandlerArg::Module(i, ts) => {
                    if let Type::Path(tp) = *tp.ty.clone() {
                        let last = tp.path.segments.last().expect("PathSegment");
                        let args = &last.arguments;
                        if let PathArguments::AngleBracketed(ab) = args {
                            let args = &ab.args;
                            where_segments.push(quote! {shaku::HasComponent<#args>});
                        }
                    }
                    (i, ts)
                }
            };

            make_args.push(method_resolve);
            give_args.push(quote! {#arg_name});
            i += 1;
        }
    });

    let fn_call_module_where = if !where_segments.is_empty() {
        quote! {
            #fn_call_module_where #(#where_segments )+*
        }
    } else {
        Default::default()
    };

    let func_copy = func.clone();

    let mut p: Punctuated<PathSegment, Colon2> = Punctuated::new();
    p.push(PathSegment {
        ident: format_ident!("{}", arg_type),
        arguments: Default::default(),
    });
    let arg_type_path = Path {
        leading_colon: None,
        segments: p,
    };

    let (empty_call, real_call, p) = match first_arg.as_str() {
        "Request" => ("Response", "Request", "darpi::Response<darpi::Body>"),
        "Response" => ("Request", "Response", "darpi::RequestParts"),
        _ => {
            return Error::new_spanned(
                func,
                format!(
                    "Accepted arguments are middlewares type `Request` or `Response`, `{}` given",
                    first_arg
                ),
            )
            .to_compile_error()
            .into();
        }
    };

    let real_call = format_ident!("call_{}", real_call);
    let empty_call = format_ident!("call_{}", empty_call);

    let p: Path = parse_str(p).unwrap();

    let tokens = quote! {
        #[allow(non_camel_case_types, missing_docs)]
        pub struct #name;
        #[allow(non_camel_case_types, missing_docs)]
        impl #name {
            #func_copy
            async fn #real_call<T>(p: &#arg_type_path, #(#fn_call_module_args ,)* #module_ident: std::sync::Arc<T>) -> Result<(), #err_ident> #fn_call_module_where {
                #(#make_args )*
                Self::#name(#(#give_args ,)*).await?;
                Ok(())
            }
            async fn #empty_call<T>(p: &#p, #(#fn_call_module_args ,)* #module_ident: std::sync::Arc<T>) -> Result<(), #err_ident> #fn_call_module_where {
                Ok(())
            }
        }
    };
    //panic!("{}", tokens.to_string());
    tokens.into()
}

enum HandlerArg {
    Expect(Ident, proc_macro2::TokenStream, proc_macro2::TokenStream),
    Module(Ident, proc_macro2::TokenStream),
    Permanent(Ident, proc_macro2::TokenStream),
}

fn make_handler_args(tp: &PatType, i: u32, module_ident: &Ident) -> HandlerArg {
    let ttype = &tp.ty;

    let arg_name = format_ident!("arg_{:x}", i);

    if let Type::Reference(rt) = *ttype.clone() {
        if let Type::Path(tp) = *rt.elem.clone() {
            let last = tp.path.segments.last().unwrap();
            if last.ident == "RequestParts" {
                let res = quote! {let #arg_name = p;};
                return HandlerArg::Permanent(arg_name, res);
            }
        }
    }

    if let Type::Path(tp) = *ttype.clone() {
        let last = tp.path.segments.last().unwrap();
        if last.ident == "Expect" {
            let mut t_type = quote! {};
            if let PathArguments::AngleBracketed(ag) = &last.arguments {
                let args = ag.args.clone();
                t_type = quote! {#args};
            }
            let res = make_expect(&arg_name, i);
            return HandlerArg::Expect(arg_name, t_type, res);
        }
    }

    let method_resolve = quote! {
        let #arg_name: #ttype = #module_ident.resolve();
    };
    HandlerArg::Module(arg_name, method_resolve)
}

fn make_expect(arg_name: &Ident, i: u32) -> proc_macro2::TokenStream {
    let c = format_ident!("T{}", i);
    quote! {
        let #arg_name = #c;
    }
}
