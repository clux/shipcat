extern crate proc_macro;

use crate::proc_macro::TokenStream;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DataStruct, DeriveInput, Fields};

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

    let body_tokens = if let Data::Struct(DataStruct { fields, .. }) = &ast.data {
        impl_merge_struct(fields)
    } else {
        panic!("Only struct types are supported")
    };

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let gen = quote! {
        impl #impl_generics Merge for #name #ty_generics #where_clause {
            fn merge(self, other: Self) -> Self {
                #body_tokens
            }
        }
    };
    gen.into()
}

fn impl_merge_struct(fields: &Fields) -> TokenStream2 {
    match fields {
        Fields::Named(_) => {
            let mut field_tokens = TokenStream2::new();

            for field in fields.iter() {
                if let Option::Some(name) = &field.ident {
                    let field_token = quote! {
                        #name: self.#name.merge(other.#name),
                    };
                    field_tokens.extend(field_token.into_iter());
                } else {
                    panic!("Unnamed fields in non-tuple structs are not supported")
                }
            }
            quote! {
                Self{
                    #field_tokens
                }
            }
        },
        Fields::Unnamed(_) => {
            let mut field_tokens = TokenStream2::new();

            for (i, field) in fields.iter().enumerate() {
                if let Option::None = &field.ident {
                    let field_token = quote! {
                        self.#i.merge(other.#i),
                    };
                    field_tokens.extend(field_token.into_iter());
                } else {
                    panic!("Unnamed fields are not supported")
                }
            }
            quote! {
                Self(
                    #field_tokens
                )
            }
        }
        Fields::Unit => quote! { other },
    }
}
