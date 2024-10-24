#![allow(incomplete_features)]
#![cfg(feature = "nightly")]
#![feature(proc_macro_diagnostic)]

extern crate proc_macro;
mod actor;

use {
    actor::{generate_actor, Item},
    proc_macro::TokenStream,
    quote::quote,
    syn::{
        parse_macro_input, {self},
    },
};

#[proc_macro_attribute]
pub fn actor(_: TokenStream, input: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(input as Item);
    generate_actor(&mut item);
    TokenStream::from(quote!(#item))
}
