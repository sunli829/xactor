extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Error, Meta, NestedMeta, AttributeArgs};

/// Implement an xactor message type.
///
/// The return value type defaults to (), and you can specify the type with the result parameter.
///
/// # Examples
///
/// ```rust
/// #[xactor::message(result="i32")]
/// struct TestMessage(i32);
/// ```
#[proc_macro_attribute]
pub fn message(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as AttributeArgs);
    let mut result_type = quote! { () };

    for arg in args {
        match arg {
            NestedMeta::Meta(Meta::NameValue(nv)) => {
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
            _ => {}
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
