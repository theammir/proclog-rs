// #[log(INFO)]
// (async)? fn foo(a: i32, b: i32) -> Result<i32, ()> { ... }
//
// into
//
// match_result: (if ReturnType ends with "Result", I guess)
//     match result {
//         // if level < ERROR
//         ::core::result::Result::Ok(_) => println!("[#level] foo({args}) = {result:?}"),
//         ::core::result::Result::Err(_) => println!("[ERROR] foo({args}) =  {result:?}"),
//     };
//
//  print_value: (otherwise)
//      // if level < ERROR
//      println!("[#level] foo({args}) = {result:?}");
//
// (async)? fn bar(a: i32, b: i32) -> Result<i32, ()> {
//     let args = [format!("{:?}", a), format!("{:?}", b)]
//         .iter()
//         .cloned()
//         .reduce(|acc, arg| format!("{acc}, {arg}"))
//         .unwrap_or(String::new());
//     let result = ((async)? || { ... })()(.await)?;
//     #match_result
//     #print_value
//     result
// }

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{parse::Parse, parse_macro_input};

#[cfg_attr(all(debug_assertions, feature = "extra-debug"), derive(Debug))]
struct LoggedFn {
    level: LogLevel,
    function: syn::ItemFn,
}

#[cfg(not(all(debug_assertions, feature = "extra-debug")))]
impl std::fmt::Debug for LoggedFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoggedFn").finish()
    }
}

impl ToTokens for LoggedFn {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let level = self.level;

        let mut new_fn = self.function.clone();
        let fn_name = &new_fn.sig.ident.to_string();
        let fn_args = new_fn
            .sig
            .inputs
            .iter()
            .map(|arg| match arg {
                syn::FnArg::Receiver(receiver) => {
                    let token = receiver.self_token;
                    quote! { &#token }
                }
                syn::FnArg::Typed(typed) => typed.pat.to_token_stream(),
            })
            .collect::<Vec<_>>();
        let fn_body = &*new_fn.block;

        let optional_async = (new_fn.sig.asyncness.is_some()).then(|| quote! { async });
        let optional_await = (new_fn.sig.asyncness.is_some()).then(|| quote! { .await });

        let returntype_path = (match &new_fn.sig.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, ty) => Some(ty),
        })
        .and_then(|ty| match &**ty {
            syn::Type::Path(syn::TypePath { qself: _, path }) => Some(path),
            _ => None,
        });

        let match_result = returntype_path
            .map(|path| path.segments.last().unwrap().ident.to_string())
            .is_some_and(|ty| ty == "Result")
            .then(|| {
                let ok_arm = (self.level < LogLevel::Error)
                    .then(|| {
                        quote! { ::core::result::Result::Ok(_) => println!(
                            "[{}] {}({args}) = {result:?}",
                            #level, #fn_name
                            )
                        }
                    })
                    .unwrap_or(quote! { _ => {} });
                quote! {
                    match result {
                        ::core::result::Result::Err(_) => println!(
                            "[ERROR] {}({args}) = {result:?}", #fn_name
                        ),
                        #ok_arm,
                    };
                }
            });
        let print_value = (match_result.is_none() && self.level < LogLevel::Error)
            .then(|| quote! { println!("[{}] {}({args}) = {result:?}", #level, #fn_name); });

        let new_body: syn::Block = syn::parse2(quote! {
            {
                let args = [#(format!("{:?}", #fn_args).to_string(),)*]
                    .iter()
                    .cloned()
                    .reduce(|acc, arg| format!("{acc}, {arg}"))
                    .unwrap_or(String::new());

                let result = (#optional_async || #fn_body)()#optional_await;
                #match_result
                #print_value
                result
            }
        })
        .unwrap();
        new_fn.block = Box::new(new_body);

        new_fn.to_tokens(tokens);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LogLevel {
    Debug,
    Info,
    Error,
}

impl Parse for LogLevel {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let level: syn::Ident = input.parse()?;
        Ok(match level.to_string().to_ascii_uppercase().as_str() {
            "DEBUG" => Self::Debug,
            "INFO" => Self::Info,
            "ERROR" => Self::Error,
            _ => return Err(syn::Error::new(level.span(), "invalid log level")),
        })
    }
}

impl ToTokens for LogLevel {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Self::Debug => quote! { "DEBUG" },
            Self::Info => quote! { "INFO" },
            Self::Error => quote! { "ERROR" },
        });
    }
}

/// Logs every execution of the function to *stdout*.
/// Accepts a literal log level: `DEBUG`, `INFO` or `ERROR`.
/// For *async* functions, only logs after receiving a result.
///
/// Every argument of the annotated function must implement [Debug],
/// as well as its return value (for *async* functions, the associated [Future::Output] type).
#[proc_macro_attribute]
pub fn log(
    attr: proc_macro::TokenStream,
    function: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let level = parse_macro_input!(attr as LogLevel);
    let function = parse_macro_input!(function as syn::ItemFn);
    let new_fn = LoggedFn { level, function };
    quote! { #new_fn }.into()
}
