extern crate proc_macro;
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;

use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// An attribute to implement the `Signal` trait for a ZST
#[proc_macro_derive(Signal)]
pub fn signal(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;

    quote!(unsafe impl signal::Signal for #ident {
        fn usize() -> usize {
            // This `static` is used to create unique identifiers
            //
            // See the embedonomicon ("Logging with symbols" chapter) for more details about this
            // technique
            #[link_section = ".signal"]
            static ID: u8 = 0;

            &ID as *const u8 as usize
        }
    })
    .into()
}
