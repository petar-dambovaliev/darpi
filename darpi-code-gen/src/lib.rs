extern crate proc_macro;

use md5;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{format_ident, quote};
use syn::export::ToTokens;
use syn::parse::Parse;
use syn::{
    braced, bracketed, parse::ParseStream, parse_macro_input, parse_quote::ParseQuote,
    punctuated::Punctuated, token::Brace, token::Colon, token::Comma, AttributeArgs, Error,
    ExprLit, ExprPath, FnArg, GenericArgument, ItemFn, ItemStruct, Lit, LitStr, Member, NestedMeta,
    PatType, PathArguments, PathSegment, Type,
};

#[proc_macro_derive(ErrorResponder)]
pub fn err_responder(input: TokenStream) -> TokenStream {
    let struct_arg = parse_macro_input!(input as ItemStruct);

    let name = &struct_arg.ident;

    let tokens = quote! {
        impl darpi_web::ErrorResponder<darpi_web::Body, darpi_web::Body, darpi_web::QueryPayloadError> for #name {
            fn respond_to_error(
                _: darpi_web::Request<darpi_web::Body>,
                e: darpi_web::QueryPayloadError,
            ) -> darpi_web::Response<darpi_web::Body> {
                let msg = match e {
                    darpi_web::QueryPayloadError::Deserialize(de) => de.to_string(),
                    darpi_web::QueryPayloadError::NotExist => "missing query params".to_string(),
                };

            darpi_web::Response::builder()
                .status(http::StatusCode::BAD_REQUEST)
                .body(darpi_web::Body::from(msg))
                .expect("this not to happen!")
            }
        }
    };

    tokens.into()
}

fn make_optional_query(arg_name: &Ident, last: &PathSegment) -> proc_macro2::TokenStream {
    quote! {
        let #arg_name: #last = match r.uri().query() {
            Some(q) => {
                let q: Result<Query<HelloWorldParams>, QueryPayloadError> =
                    Query::from_query(q);
                Some(q.unwrap())
            }
            None => None,
        };
    }
}

fn make_query(arg_name: &Ident, last: &PathSegment) -> proc_macro2::TokenStream {
    let inner = &last.arguments;
    quote! {
        fn query_error_response<T, R, S, E>(r: darpi_web::Request<R>, e: E) -> darpi_web::Response<S>
        where
            T: darpi_web::ErrorResponder<R, S, E>,
            E: std::error::Error,
        {
            T::respond_to_error(r, e)
        }

        let #arg_name = match r.uri().query() {
            Some(q) => q,
            None => return Ok(query_error_response::<#last, darpi_web::Body, darpi_web::Body, darpi_web::QueryPayloadError>(r, darpi_web::QueryPayloadError::NotExist))
        };

        let #arg_name: #last = match Query::from_query(#arg_name) {
            Ok(q) => q,
            Err(e) => return Ok(query_error_response::<#last, darpi_web::Body, darpi_web::Body, darpi_web::QueryPayloadError>(r, e))
        };
    }
}

fn make_handler_args(tp: &PatType, i: u32) -> (Ident, proc_macro2::TokenStream) {
    let ttype = &tp.ty;

    let arg_name = format_ident!("arg_{:x}", i);

    let method_resolve = quote! {
        let #arg_name: #ttype = module.resolve();
    };

    if let Type::Path(tp) = *ttype.clone() {
        let last = tp.path.segments.last().unwrap();
        if last.ident == "Query" {
            let res = make_query(&arg_name, last);
            return (arg_name, res);
        }
        if last.ident == "Option" {
            if let PathArguments::AngleBracketed(ab) = &last.arguments {
                if let GenericArgument::Type(t) = ab.args.first().unwrap() {
                    if let Type::Path(tp) = t {
                        let first = tp.path.segments.first().unwrap();
                        if first.ident == "Query" {
                            let res = make_optional_query(&arg_name, last);
                            return (arg_name, res);
                        }
                    }
                }
            }
        }
    }

    (arg_name, method_resolve)
}

#[proc_macro_attribute]
pub fn handler(args: TokenStream, input: TokenStream) -> TokenStream {
    let func = parse_macro_input!(input as ItemFn);
    let attr_args = parse_macro_input!(args as AttributeArgs);

    let mut is_query = false;
    func.sig
        .inputs
        .iter()
        .for_each(|arg| is_query = arg.to_token_stream().to_string().contains("Query"));

    if attr_args.is_empty() && !func.sig.inputs.is_empty() && !is_query {
        return Error::new_spanned(func, "Handlers with dependencies require a module")
            .to_compile_error()
            .into();
    }

    if attr_args.len() > 1 {
        return Error::new_spanned(func, "There should be 0 or 1 arguments")
            .to_compile_error()
            .into();
    }

    let mut module_ident = None;

    attr_args.iter().for_each(|arg| {
        if let NestedMeta::Meta(meta) = arg {
            module_ident = meta.path().get_ident();
        }
    });

    if func.sig.asyncness.is_none() {
        return Error::new_spanned(func, "Only Async functions can be used as handlers")
            .to_compile_error()
            .into();
    }

    let func_copy = func.clone();
    let mut make_args = vec![];
    let mut give_args = vec![];
    let mut i = 0_u32;

    func.sig.inputs.iter().for_each(|arg| {
        if let FnArg::Typed(tp) = arg {
            let (arg_name, method_resolve) = make_handler_args(tp, i);

            make_args.push(method_resolve);
            give_args.push(quote! {#arg_name});
            i += 1;
        }
    });

    let return_type = &func.sig.output;

    let func_name = func.sig.ident;

    let fn_call = match module_ident {
        Some(m) => {
            quote! {
                async fn call(r: Request<Body> ,module: std::sync::Arc<#m>) #return_type {
                   #(#make_args )*
                   Self::#func_name(#(#give_args ,)*).await
               }
            }
        }
        None => {
            quote! {
                async fn call<T>(r: Request<Body>, _: T) #return_type {
                    #(#make_args )*
                   Self::#func_name(#(#give_args ,)*).await
               }
            }
        }
    };

    let output = quote! {
        #[allow(non_camel_case_types, missing_docs)]
        pub struct #func_name;

        impl #func_name {
           #fn_call
           //user defined function
           #func_copy
       }
    };
    output.into()
}

#[derive(Debug)]
enum Expr {
    ExprArrayHandler(ExprArrayHandler),
    Module(ExprPath),
    ExprLit(ExprLit),
    ExprPath(ExprPath),
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
            let val: ExprPath = parse_variant(input)?;
            Expr::Module(val)
        } else if member_ident == "bind" {
            let val: ExprArrayHandler = parse_variant(input)?;
            Expr::ExprArrayHandler(val)
        } else if member_ident == "route" {
            let val: ExprLit = parse_variant(input)?;

            let mut illegal_chars = vec![];
            let allowed = vec!['/', '_', '{', '}', '"'];

            if let Lit::Str(lt) = &val.lit {
                lt.value().chars().for_each(|ch| {
                    if !ch.is_alphanumeric() && !allowed.contains(&ch) {
                        illegal_chars.push(ch);
                    }
                });
            }

            if !illegal_chars.is_empty() {
                return Err(Error::new(
                    Span::call_site(),
                    format!("invalid characters in route: {:?}", illegal_chars),
                ));
            }

            Expr::ExprLit(val)
        } else if member_ident == "address" {
            let val: ExprLit = parse_variant(input)?;
            Expr::ExprLit(val)
        } else if member_ident == "method" {
            let val: ExprPath = parse_variant(input)?;
            Expr::ExprPath(val)
        } else if member_ident == "handler" {
            let val: ExprPath = parse_variant(input)?;
            Expr::ExprPath(val)
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

fn get_fields(
    app_struct: AppStruct,
) -> Result<(LitStr, Option<ExprPath>, Vec<ExprHandler>), TokenStream> {
    let module = app_struct
        .fields
        .iter()
        .find(|f| &f.member.to_string() == "module");

    let module_path = if module.is_some() {
        match &module.unwrap().expr {
            Expr::Module(module_path) => Some(module_path.clone()),
            _ => {
                return Err(Error::new(
                    Span::call_site(),
                    "module should be a path to the DI module definition",
                )
                .to_compile_error()
                .into())
            }
        }
    } else {
        None
    };

    let handlers = app_struct
        .fields
        .iter()
        .find(|f| &f.member.to_string() == "bind")
        .expect("missing handlers");

    let array_expr = match &handlers.expr {
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
        .into_iter()
        .find(|f| &f.member.to_string() == "address")
        .expect("missing handlers");

    let address = match address_field.expr {
        Expr::ExprLit(lit) => match lit.lit {
            Lit::Str(str) => str,
            _ => {
                return Err(Error::new(
                    Span::call_site(),
                    "server address should be a &str literal",
                )
                .to_compile_error()
                .into())
            }
        },
        _ => {
            return Err(
                Error::new(Span::call_site(), "server address should be a &str literal")
                    .to_compile_error()
                    .into(),
            )
        }
    };

    Ok((address, module_path, array_expr))
}

#[proc_macro]
pub fn run(input: TokenStream) -> TokenStream {
    let app_struct: AppStruct =
        syn::parse(input).unwrap_or_else(|e| panic!("app_struct: {:#?}", e));

    let (addr, module_path, elements) = match get_fields(app_struct) {
        Ok((addr, m, a)) => (addr, m, a),
        Err(e) => return e,
    };

    let address: std::net::SocketAddr = addr
        .value()
        .parse()
        .expect(&format!("invalid server address: `{}`", addr.value()));

    let address_value = format!("{}", address.to_string());

    if elements.is_empty() {
        return Error::new(Span::call_site(), "no handlers registered")
            .to_compile_error()
            .into();
    }

    let mut is = vec![];
    let handler_len = elements.len();

    let mut routes = vec![];
    let mut routes_match = vec![];

    elements.iter().for_each(|el| {
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

        is.push(quote! {
            RoutePossibilities::#variant_name => {
                route == #route && method == #method.as_str()
            }
        });

        routes.push(quote! {
            #variant_name
        });

        routes_match.push(quote! {
            RoutePossibilities::#variant_name => #variant_value::call(r, inner_module).await
        });
    });

    let route_possibilities = quote! {
        #[allow(non_camel_case_types, missing_docs)]
        pub enum RoutePossibilities {
            #(#routes ,)*
        }

        impl RoutePossibilities {
            pub fn is(&self, route: &str, method: &http::Method) -> bool {
                return match self {
                    #(#is ,)*
                };
            }
        }
    };

    let (module_def, module_t) = match module_path {
        Some(mp) => {
            let patj = mp.path.get_ident().expect("could not get module_ident");
            (quote! {module: std::sync::Arc<#patj>,}, quote! {#patj})
        }
        None => (quote! {}, quote! {}),
    };

    let (module_let, module_self) = if !module_def.is_empty() {
        (
            quote! {let module = std::sync::Arc::new(#module_t::builder().build());},
            quote! {
                module: module,
            },
        )
    } else {
        (quote! {}, quote! {})
    };

    let app = quote! {
         pub struct App {
            #module_def
            handlers: std::sync::Arc<[RoutePossibilities; #handler_len]>,
            address: std::net::SocketAddr,
        }

        impl App {
            pub fn new() -> Self {
                let address: std::net::SocketAddr = #address_value
                    .parse()
                    .expect(&format!("invalid server address: `{}`", #address_value));

                #module_let
                Self {
                    #module_self
                    handlers: std::sync::Arc::new([#(RoutePossibilities::#routes ,)*]),
                    address: address,
                }
            }

             pub async fn start(self) {
                let address = self.address;
                let module = self.module.clone();
                let handlers = self.handlers.clone();

                let make_svc = hyper::service::make_service_fn(move |_conn| {
                    let inner_module = std::sync::Arc::clone(&module);
                    let inner_handlers = std::sync::Arc::clone(&handlers);
                    async move {
                        Ok::<_, std::convert::Infallible>(hyper::service::service_fn(move |r: hyper::Request<hyper::Body>| {
                            let inner_module = std::sync::Arc::clone(&inner_module);
                            let inner_handlers = std::sync::Arc::clone(&inner_handlers);
                            async move {
                                let route = r.uri().path();
                                let method = r.method();

                                let handler = inner_handlers
                                    .iter()
                                    .find(|h| h.is(route, method))
                                    .expect(&format!(
                                        "no such handler for route: {} method: {}",
                                        route, method
                                    ));

                                match handler {
                                    #(#routes_match ,)*
                                }
                            }
                        }))
                    }
                });

                let server = hyper::Server::bind(&address).serve(make_svc);
                if let Err(e) = server.await {
                    eprintln!("server error: {}", e);
                }
             }
        }
    };

    let tokens = quote! {
        #route_possibilities
        #app
        App::new().start().await;
    };
    tokens.into()
}
