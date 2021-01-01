use crate::handler::MODULE_PREFIX;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{format_ident, quote};
use syn::export::ToTokens;
use syn::punctuated::Punctuated;
use syn::{
    parse::ParseStream, parse_macro_input, token::Bracket, token::Colon2, token::Comma,
    AngleBracketedGenericArguments, AttributeArgs, Error, FnArg, GenericArgument, ItemFn, Lifetime,
    PatType, Path, PathArguments, PathSegment, ReturnType, Type, TypePath,
};

fn get_return_type(r: &ReturnType) -> proc_macro2::TokenStream {
    if let ReturnType::Type(_, t) = r.clone() {
        if let Type::Path(tp) = *t {
            let last = tp.path.segments.last().unwrap();
            if &last.ident != "Result" {
                return Error::new_spanned(r, "Only Result supported")
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

    let (middleware_type, arg_type) = match first_arg.as_str() {
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

    let trait_name = format_ident!("{}", middleware_type);
    let mut p = Punctuated::new();

    let mut pp = Punctuated::new();
    let mut ps: Punctuated<PathSegment, Colon2> = Punctuated::new();

    ps.push(PathSegment {
        ident: err_ident.clone(),
        arguments: Default::default(),
    });

    pp.push(GenericArgument::Type(Type::Path(TypePath {
        qself: None,
        path: Path {
            leading_colon: None,
            segments: ps,
        },
    })));

    p.push(PathSegment {
        ident: trait_name,
        arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments {
            colon2_token: None,
            lt_token: Default::default(),
            args: pp,
            gt_token: Default::default(),
        }),
    });

    let trait_name = Path {
        leading_colon: None,
        segments: p,
    };

    let name = func.sig.ident.clone();
    let expect_fn_name = format_ident!("expect_func_{}", name);

    let mut make_args = vec![];
    let mut give_args = vec![];
    let mut module_full_req = vec![];
    let mut i = 0_u32;
    let mut n_args = 0u8;
    let mut fn_call_generic = vec![];
    let mut fn_call_where = vec![];
    let mut fn_call_module_generic = quote! {T0};
    let mut fn_call_module_args = vec![];
    let expect_trait_name = format_ident!("ExpectValue_{}", name);

    func.sig.inputs.iter().for_each(|arg| {
        if let FnArg::Typed(tp) = arg {
            let module_ident = format_ident!("{}_{}", MODULE_PREFIX, i);
            let gen_t = format_ident!("T{}", i + 1);
            //todo change the return type to enum
            //todo add nopath trait to check from handler if path arg is not used
            let (arg_name, method_resolve) =
                match make_handler_args(tp, i, &module_ident, &expect_fn_name, &gen_t) {
                    HandlerArg::Permanent(i, ts) => (i, ts),
                    HandlerArg::Expect(id, ttype, ts) => {
                        let n = i + 1;
                        let cg = format_ident!("T{}", n);
                        fn_call_generic.push(quote! {
                            #cg
                        });
                        fn_call_where.push(quote! {#cg: #expect_trait_name<#ttype>});

                        (id, ts)
                    }
                    HandlerArg::Module(i, ts) => {
                        if let Type::Path(mut tp) = *tp.ty.clone() {
                            let mut last = tp
                                .path
                                .segments
                                .iter_mut()
                                .last()
                                .expect("cannot get secment");

                            module_full_req.push(quote! {shaku::HasComponent<#tp + 'static>});
                            fn_call_module_args.push(
                            quote! {#module_ident: std::sync::Arc<impl shaku::HasComponent<#tp>>},
                        );
                            fn_call_module_generic = Default::default();
                        }
                        (i, ts)
                    }
                };

            make_args.push(method_resolve);
            give_args.push(quote! {#arg_name});
            i += 1;
        }
    });

    if !fn_call_module_generic.is_empty() {
        fn_call_generic.push(fn_call_module_generic);
    }

    if fn_call_module_args.is_empty() {
        fn_call_module_args.push(quote! {m: std::sync::Arc<T>});
    }

    let mut def_gen = vec![];
    if !fn_call_generic.is_empty() {
        def_gen.push(quote! {<});
        let l = fn_call_generic.len();
        for (i, j) in fn_call_generic.into_iter().enumerate() {
            def_gen.push(j);
            if i != l - 1 {
                def_gen.push(quote! {,});
            }
        }
        def_gen.push(quote! {>});
    }

    if !fn_call_where.is_empty() {
        let mut c = vec![quote! {where }];
        let l = fn_call_where.len();
        for (i, j) in fn_call_where.into_iter().enumerate() {
            c.push(j);
            if i != l - 1 {
                c.push(quote! {,});
            }
        }
        fn_call_where = c;
    }

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

    let tokens = quote! {
        pub trait #expect_trait_name<T> {
            fn expect() -> T;
        }

        fn #expect_fn_name<T, R>() -> R
        where
            T: #expect_trait_name<R>,
        {
            T::expect()
        }

        pub struct #name;
        impl #name {
            #func_copy
            async fn call#(#def_gen)*(&self, p: &#arg_type_path, #(#fn_call_module_args ,)*) -> Result<(), #err_ident> #(#fn_call_where)* {
                #(#make_args )*
                Self::#name(#(#give_args ,)*).await?;
                Ok(())
            }
        }
    };
    tokens.into()
}

enum HandlerArg {
    Expect(Ident, proc_macro2::TokenStream, proc_macro2::TokenStream),
    Module(Ident, proc_macro2::TokenStream),
    Permanent(Ident, proc_macro2::TokenStream),
}

fn make_handler_args(
    tp: &PatType,
    i: u32,
    module_ident: &Ident,
    expect_name: &Ident,
    gen_t: &Ident,
) -> HandlerArg {
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
            let res = make_expect(&arg_name, t_type.clone(), expect_name, gen_t);
            return HandlerArg::Expect(arg_name, t_type, res);
        }
    }

    let method_resolve = quote! {
        let #arg_name: #ttype = #module_ident.resolve();
    };
    HandlerArg::Module(arg_name, method_resolve)
}

fn make_expect(
    arg_name: &Ident,
    last: proc_macro2::TokenStream,
    expect_name: &Ident,
    gen_t: &Ident,
) -> proc_macro2::TokenStream {
    quote! {
        let #arg_name = Expect(#expect_name::<#gen_t, #last>());
    }
}
