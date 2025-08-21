use darling::FromDeriveInput;
use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{Ident, Item, Expr, parse_macro_input};

#[derive(FromDeriveInput)]
#[darling(attributes(can))]
struct Can {
    ident: Ident,
    id: Expr,
}

#[proc_macro_derive(CanMessage, attributes(can))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);

    let Can { ident, id } = Can::from_derive_input(&input).expect("Missing or incorrect #[can] attribute");

    let output = quote! {
        #[automatically_derived]
        impl ::can_messages_trait::CanMessage for #ident {
            const ID: u16 = (#id) as u16;
        }
    };
    output.into()
}

#[proc_macro_attribute]
pub fn can_message(attr: TokenStream, item: TokenStream) -> TokenStream {
    let id: Expr = parse_macro_input!(attr);
    let item: Item = parse_macro_input!(item);

    let output = quote! {
        #[repr(C)]
        #[derive(Debug, TryFromBytes, IntoBytes, Immutable, KnownLayout, Clone, CanMessage)]
        #[can(id = #id)]
        #item
    };
    output.into()
}
