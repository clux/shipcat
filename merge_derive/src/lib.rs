extern crate proc_macro;

use crate::proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DataStruct, DeriveInput};

#[proc_macro_derive(Merge)]
pub fn merge_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    impl_merge(&ast)
}

fn impl_merge(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let mut field_tokens = proc_macro2::TokenStream::new();
    if let Data::Struct(DataStruct { fields, .. }) = &ast.data {
        for field in fields.iter() {
            if let Option::Some(name) = &field.ident {
                let field_token = quote! {
                    #name: self.#name.merge(other.#name),
                };
                field_tokens.extend(field_token.into_iter());
            } else {
                panic!("Unnamed fields are not supported")
            }
        }
    } else {
        panic!("Only struct types are supported")
    }

    let gen = quote! {
        impl Merge for #name {
            fn merge(self, other: Self) -> Self {
                Self{
                    #field_tokens
                }
            }
        }
    };
    gen.into()
}
