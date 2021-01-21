use crate::handler::MODULE_PREFIX;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::ToTokens;
use quote::{format_ident, quote};
use syn::parse_quote::{parse, ParseQuote};
use syn::punctuated::Punctuated;
use syn::{
    bracketed, parse::ParseStream, parse_macro_input, parse_str, token::Colon2, token::Comma,
    AngleBracketedGenericArguments, AttributeArgs, Error, ExprCall, FnArg, ItemFn, PatType, Path,
    PathArguments, PathSegment, ReturnType, Type,
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
    let mut func = parse_macro_input!(input as ItemFn);
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

    let (arg_type, gen_args) = match first_arg.as_str() {
        "Request" => ("RequestParts", PathArguments::default()),
        "Response" => {
            let q = quote! {<Body>};
            let args: AngleBracketedGenericArguments = parse(q);
            ("Response", PathArguments::AngleBracketed(args))
        }
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
    let fn_call_module_where = quote! { where mygenericmodule: };
    let mut where_segments = vec![];
    let mut fn_call_module_args = vec![];
    let module_ident = format_ident!("{}", MODULE_PREFIX);

    for arg in func.sig.inputs.iter_mut() {
        if let FnArg::Typed(tp) = arg {
            let h_args = match make_handler_args(tp, i, &module_ident) {
                Ok(k) => k,
                Err(e) => return e,
            };
            let (arg_name, method_resolve) = match h_args {
                HandlerArg::Permanent(i, ts) => (i, ts),
                HandlerArg::Expect(id, ttype, ts) => {
                    let cg = format_ident!("T{}", i);
                    fn_call_module_args.push(quote! {#cg: #ttype});

                    (id.to_token_stream(), ts)
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
                    (i.to_token_stream(), ts)
                }
            };

            make_args.push(method_resolve);
            give_args.push(quote! {#arg_name});
            i += 1;
            tp.attrs = Default::default();
        }
    }

    let mut func_where = func.sig.generics.where_clause.to_token_stream();
    let func_gen_params = &func.sig.generics.params;
    let func_gen_call = if !func_gen_params.is_empty() {
        quote! {::<F, T, E>}
    } else {
        Default::default()
    };

    let fn_call_module_where = if !where_segments.is_empty() {
        if !func_where.is_empty() {
            func_where = quote! {#func_where ,};
        }
        quote! {
            #func_where #fn_call_module_where #(#where_segments )+*
        }
    } else {
        func_where.to_token_stream()
    };

    let func_copy = func.clone();

    let mut p: Punctuated<PathSegment, Colon2> = Punctuated::new();
    p.push(PathSegment {
        ident: format_ident!("{}", "darpi"),
        arguments: Default::default(),
    });
    p.push(PathSegment {
        ident: format_ident!("{}", arg_type),
        arguments: gen_args,
    });
    let arg_type_path = Path {
        leading_colon: None,
        segments: p,
    };

    let mut real_body: Option<Path> = None;
    let mut empty_body: Option<Path> = None;

    let (empty_call, real_call, p) = match first_arg.as_str() {
        "Request" => {
            real_body = Some(parse_str("Body").unwrap());
            ("Response", "Request", "darpi::Response<darpi::Body>")
        }
        "Response" => {
            empty_body = Some(parse_str("Body").unwrap());
            ("Request", "Response", "darpi::RequestParts")
        }
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

    let output = &func_copy.sig.output;

    let p: Path = parse_str(p).unwrap();

    let real_body = real_body.map_or(Default::default(), |b| {
        quote! {,b: &mut #b}
    });
    let empty_body = empty_body.map_or(Default::default(), |b| {
        quote! {,b: &mut #b}
    });

    let visibility = func.vis;

    let tokens = quote! {
        #[allow(non_camel_case_types, missing_docs)]
        pub struct #name;
        #[allow(non_camel_case_types, missing_docs)]
        impl #name {
            #func_copy
            #visibility async fn #real_call<mygenericmodule, #func_gen_params>(p: &mut #arg_type_path, #(#fn_call_module_args ,)* #module_ident: std::sync::Arc<mygenericmodule> #real_body) #output #fn_call_module_where {
                #(#make_args )*
                Self::#name#func_gen_call(#(#give_args ,)*).await
            }
            #visibility async fn #empty_call<mygenericmodule, #func_gen_params>(p: &#p, #(#fn_call_module_args ,)* #module_ident: std::sync::Arc<mygenericmodule> #empty_body) -> Result<(), #err_ident> #fn_call_module_where {
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
    Permanent(proc_macro2::TokenStream, proc_macro2::TokenStream),
}

fn make_handler_args(
    tp: &PatType,
    i: u32,
    module_ident: &Ident,
) -> Result<HandlerArg, TokenStream> {
    let ttype = &tp.ty;

    let arg_name = format_ident!("arg_{:x}", i);

    let attr = tp.attrs.first().unwrap();
    let attr_ident = attr.path.get_ident().unwrap();

    if let Type::Reference(rt) = *ttype.clone() {
        if let Type::Path(_) = *rt.elem.clone() {
            if attr_ident == "request_parts" {
                let res = quote! {let #arg_name = p;};
                return Ok(HandlerArg::Permanent(arg_name.to_token_stream(), res));
            }
            if attr_ident == "body" {
                let res = quote! {let mut #arg_name = b;};
                let tt = quote! {&mut #arg_name};
                return Ok(HandlerArg::Permanent(tt, res));
            }
            if attr_ident == "response" {
                let res = quote! {let mut #arg_name = p;};
                let tt = quote! {&mut #arg_name};
                return Ok(HandlerArg::Permanent(tt, res));
            }
        }
    }

    if attr_ident == "handler" {
        let res = make_expect(&arg_name, i);
        let t_type = quote! {#ttype};
        return Ok(HandlerArg::Expect(arg_name, t_type, res));
    }
    if attr_ident == "inject" {
        let method_resolve = quote! {
            let #arg_name: #ttype = #module_ident.resolve();
        };
        return Ok(HandlerArg::Module(arg_name, method_resolve));
    }

    Err(Error::new(
        Span::call_site(),
        format!(
            "unsupported attribute #[{}] type {}",
            attr_ident,
            ttype.to_token_stream().to_string()
        ),
    )
    .to_compile_error()
    .into())
}

fn make_expect(arg_name: &Ident, i: u32) -> proc_macro2::TokenStream {
    let c = format_ident!("T{}", i);
    quote! {
        let #arg_name = #c;
    }
}
