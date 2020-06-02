extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{parse_macro_input, AttributeArgs, DeriveInput, Error, Meta, NestedMeta, ReturnType};

/// Implement an xactor message type.
///
/// The return value type defaults to (), and you can specify the type with the result parameter.
///
/// # Examples
///
/// ```ignore
/// #[message(result = "i32")]
/// struct TestMessage(i32);
/// ```
#[proc_macro_attribute]
pub fn message(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let mut result_type = quote! { () };

    for arg in args {
        if let NestedMeta::Meta(Meta::NameValue(nv)) = arg {
            if nv.path.is_ident("result") {
                if let syn::Lit::Str(lit) = nv.lit {
                    if let Ok(ty) = syn::parse_str::<syn::Type>(&lit.value()) {
                        result_type = quote! { #ty };
                    } else {
                        return Error::new_spanned(&lit, "Expect type")
                            .to_compile_error()
                            .into();
                    }
                }
            }
        }
    }

    let input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;
    let expanded = quote! {
        #input
        impl xactor::Message for #ident {
            type Result = #result_type;
        }
    };
    expanded.into()
}

/// Implement an xactor main function.
///
/// Wait for all actors to exit.
#[proc_macro_attribute]
pub fn main(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::ItemFn);

    let ret = &input.sig.output;
    let inputs = &input.sig.inputs;
    let name = &input.sig.ident;
    let body = &input.block;
    let attrs = &input.attrs;

    if name != "main" {
        return TokenStream::from(quote_spanned! { name.span() =>
            compile_error!("only the main function can be tagged with #[xactor::main]"),
        });
    }

    if input.sig.asyncness.is_none() {
        return TokenStream::from(quote_spanned! { input.span() =>
            compile_error!("the async keyword is missing from the function declaration"),
        });
    }

    if let ReturnType::Type(_, _) = ret {
        return TokenStream::from(quote_spanned! { input.span() =>
            compile_error!("main function cannot have a return value"),
        });
    }

    let expanded = quote! {
        fn main() {
            #(#attrs)*
            async fn main(#inputs) {
                #body
                xactor::System::wait_all().await;
            }

            #[cfg(feature = "runtime-async-std")]
            {
                async_std::task::block_on(async {
                    main().await
                })
            }

            #[cfg(feature = "runtime-tokio")]
            {
                tokio::task::spawn_blocking(move || async {
                    main().await
                });
            }
        }
    };
    expanded.into()
}
