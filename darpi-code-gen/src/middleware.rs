use crate::handler::MODULE_PREFIX;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::ToTokens;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, AttributeArgs, Error, FnArg, ItemFn, PatType, Path, PathArguments,
    PathSegment, ReturnType, Type,
};

pub(crate) fn make_middleware(args: TokenStream, input: TokenStream) -> TokenStream {
    let func = parse_macro_input!(input as ItemFn);
    if func.sig.asyncness.is_none() {
        return Error::new_spanned(func, "Only Async functions can be used as middleware")
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

    match first_arg.as_str() {
        "Request" => make_req_middleware(func),
        "Response" => make_res_middleware(func),
        _ => Error::new_spanned(
            func,
            format!(
                "Accepted arguments are middlewares type `Request` or `Response`, `{}` given",
                first_arg
            ),
        )
        .to_compile_error()
        .into(),
    }
}

pub(crate) fn make_res_middleware(mut func: ItemFn) -> TokenStream {
    let name = func.sig.ident.clone();
    let CallArgs {
        make,
        give,
        where_clause,
        handler_types,
        handler_bounds,
    } = match make_args(&mut func) {
        Ok(a) => a,
        Err(e) => return e,
    };

    let func_gen_params = &func.sig.generics.params;
    let func_gen_call = if !func_gen_params.is_empty() {
        quote! {::<#func_gen_params>}
    } else {
        Default::default()
    };

    let mut resolve_call = quote! {Self::#name#func_gen_call(#(#give ,)*).await};
    let mut k = Default::default();
    let mut e = Default::default();

    match &func.sig.output {
        ReturnType::Default => {
            resolve_call = quote! {
                #resolve_call;
                Ok(())
            };
        }
        ReturnType::Type(_, t) => {
            if let Type::Path(tp) = *t.clone() {
                let last = tp.path.segments.last().unwrap();
                if last.ident == "Result" {
                    if let PathArguments::AngleBracketed(ab) = &last.arguments {
                        assert_eq!(ab.args.len(), 2);
                        k = ab.args[0].to_token_stream();
                        e = ab.args[1].to_token_stream();
                    }
                }
            }
        }
    }

    let where_module = match where_clause.is_empty() {
        true => Default::default(),
        false => {
            quote! {+ #(#where_clause +)*}
        }
    };

    let handler_t = if handler_types.len() == 1 {
        quote! {#(#handler_types)*}
    } else {
        quote! {( #(#handler_types ,)* )}
    };

    let (gen_params, with_brackets, bounds, phantom_data) = if handler_bounds.is_empty() {
        (
            Default::default(),
            Default::default(),
            Default::default(),
            quote! {;},
        )
    } else {
        let mut bound = vec![];
        let mut phantom_data = vec![];
        for i in 0..handler_bounds.len() {
            let id = &handler_types[i];
            let hb = handler_bounds[i].clone();
            bound.push(quote! {#id: #(#hb +)*});
            let m_id = format_ident!("_marker{}", i);
            phantom_data.push(quote! {#m_id: std::marker::PhantomData<#id>});
        }

        (
            quote! {, #(#handler_types ,)*},
            quote! {<#(#handler_types ,)*>},
            quote! { #(#bound ,)*},
            quote! {{#(#phantom_data ,)*}},
        )
    };

    let tokens = quote! {
        #[allow(non_camel_case_types, missing_docs)]
        pub struct #name#with_brackets#phantom_data
        #[allow(non_camel_case_types, missing_docs)]
        impl#with_brackets #name#with_brackets {
            #func
        }

        #[darpi::async_trait]
        impl<M #gen_params> darpi::ResponseMiddleware<M> for #name#with_brackets
        where
            M: 'static + Sync + Send #where_module,
            #bounds
        {
            type HandlerArgs = #handler_t;
            type Error = #e;
            type Type = #k;

            async fn call(
                r: &mut Response<Body>,
                module: std::sync::Arc<M>,
                ha: Self::HandlerArgs,
            ) -> Result<Self::Type, Self::Error> {
                #(#make )*
                #resolve_call
            }
        }
    };
    //asd
    //panic!("{}", tokens.to_string());
    tokens.into()
}

pub(crate) fn make_req_middleware(mut func: ItemFn) -> TokenStream {
    let name = func.sig.ident.clone();
    let CallArgs {
        make,
        give,
        where_clause,
        handler_types,
        handler_bounds,
    } = match make_args(&mut func) {
        Ok(a) => a,
        Err(e) => return e,
    };

    let func_gen_params = &func.sig.generics.params;
    let func_gen_call = if !func_gen_params.is_empty() {
        quote! {::<#func_gen_params>}
    } else {
        Default::default()
    };

    let mut resolve_call = quote! {Self::#name#func_gen_call(#(#give ,)*).await};
    let mut k = Default::default();
    let mut e = Default::default();

    match &func.sig.output {
        ReturnType::Default => {
            resolve_call = quote! {
                #resolve_call;
                Ok(())
            };
        }
        ReturnType::Type(_, t) => {
            if let Type::Path(tp) = *t.clone() {
                let last = tp.path.segments.last().unwrap();
                if last.ident == "Result" {
                    if let PathArguments::AngleBracketed(ab) = &last.arguments {
                        assert_eq!(ab.args.len(), 2);
                        k = ab.args[0].to_token_stream();
                        e = ab.args[1].to_token_stream();
                    }
                }
            }
        }
    }

    let where_module = match where_clause.is_empty() {
        true => Default::default(),
        false => {
            quote! {+ #(#where_clause +)*}
        }
    };

    let handler_t = if handler_types.len() == 1 {
        quote! {#(#handler_types)*}
    } else {
        quote! {( #(#handler_types ,)* )}
    };

    let (gen_params, with_brackets, bounds, phantom_data) = if handler_bounds.is_empty() {
        (
            Default::default(),
            Default::default(),
            Default::default(),
            quote! {;},
        )
    } else {
        let mut bound = vec![];
        let mut phantom_data = vec![];
        for i in 0..handler_bounds.len() {
            let id = &handler_types[i];
            let hb = handler_bounds[i].clone();
            bound.push(quote! {#id: #(#hb +)*});
            let m_id = format_ident!("_marker{}", i);
            phantom_data.push(quote! {#m_id: std::marker::PhantomData<#id>});
        }

        (
            quote! {, #(#handler_types ,)*},
            quote! {<#(#handler_types ,)*>},
            quote! { #(#bound ,)*},
            quote! {{#(#phantom_data ,)*}},
        )
    };

    let tokens = quote! {
        #[allow(non_camel_case_types, missing_docs)]
        pub struct #name#with_brackets#phantom_data
        #[allow(non_camel_case_types, missing_docs)]
        impl#with_brackets #name#with_brackets {
            #func
        }

        #[darpi::async_trait]
        impl<M #gen_params> darpi::RequestMiddleware<M> for #name#with_brackets
        where
            M: 'static + Sync + Send #where_module,
            #bounds
        {
            type HandlerArgs = #handler_t;
            type Error = #e;
            type Type = #k;

            async fn call(
                p: &mut darpi::RequestParts,
                module: std::sync::Arc<M>,
                b: &mut darpi::Body,
                ha: Self::HandlerArgs,
            ) -> Result<Self::Type, Self::Error> {
                #(#make )*
                #resolve_call
            }
        }
    };
    //panic!("{}", tokens.to_string());
    tokens.into()
}

struct CallArgs {
    make: Vec<TokenStream2>,
    give: Vec<TokenStream2>,
    where_clause: Vec<TokenStream2>,
    handler_types: Vec<TokenStream2>,
    handler_bounds: Vec<Vec<TokenStream2>>,
}

fn make_args(mut func: &mut ItemFn) -> Result<CallArgs, TokenStream> {
    let mut make = vec![];
    let mut give = vec![];
    let mut i = 0_u32;
    let mut where_clause = vec![];
    let mut handler_types = vec![];
    let mut handler_bounds = vec![];

    let module_ident = format_ident!("{}", MODULE_PREFIX);

    for arg in func.sig.inputs.iter_mut() {
        if let FnArg::Typed(tp) = arg {
            let h_args = match make_handler_arg(tp, i, &module_ident) {
                Ok(k) => k,
                Err(e) => return Err(e),
            };
            let (arg_name, method_resolve) = match h_args {
                HandlerArg::Permanent(i, ts) => (i, ts),
                HandlerArg::Handler(bounds, id, ttype, ts) => {
                    handler_types.push(ttype);
                    if !bounds.is_empty() {
                        handler_bounds.push(bounds);
                    }
                    (id.to_token_stream(), ts)
                }
                HandlerArg::Module(i, ts) => {
                    if let Type::Path(tp) = *tp.ty.clone() {
                        let last = tp.path.segments.last().expect("PathSegment");
                        let args = &last.arguments;
                        if let PathArguments::AngleBracketed(ab) = args {
                            let args = &ab.args;
                            where_clause.push(quote! {shaku::HasComponent<#args>});
                        }
                    }
                    (i.to_token_stream(), ts)
                }
            };

            make.push(method_resolve);
            give.push(quote! {#arg_name});
            i += 1;
            tp.attrs = Default::default();
        }
    }
    Ok(CallArgs {
        make,
        give,
        where_clause,
        handler_types,
        handler_bounds,
    })
}

enum HandlerArg {
    Handler(
        Vec<proc_macro2::TokenStream>,
        Ident,
        proc_macro2::TokenStream,
        proc_macro2::TokenStream,
    ),
    Module(Ident, proc_macro2::TokenStream),
    Permanent(proc_macro2::TokenStream, proc_macro2::TokenStream),
}

fn make_handler_arg(tp: &PatType, i: u32, module_ident: &Ident) -> Result<HandlerArg, TokenStream> {
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
                let res = quote! {let mut #arg_name = r;};
                let tt = quote! {&mut #arg_name};
                return Ok(HandlerArg::Permanent(tt, res));
            }
        }
    }

    if attr_ident == "handler" {
        let res = make_expect(&arg_name);
        let mut bounds = vec![];
        if let Type::ImplTrait(imt) = *ttype.clone() {
            for j in imt.bounds {
                bounds.push(quote! {#j});
            }
            let ii = format_ident!("T{}", i);
            let t_type = quote! {#ii};
            return Ok(HandlerArg::Handler(bounds, arg_name, t_type, res));
        }

        let t_type = quote! {#ttype};
        return Ok(HandlerArg::Handler(bounds, arg_name, t_type, res));
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

fn make_expect(arg_name: &Ident) -> proc_macro2::TokenStream {
    quote! {
        let #arg_name = ha;
    }
}
