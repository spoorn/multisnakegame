use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{Data, DataStruct, Fields, parse_macro_input, DeriveInput, Field};
use syn::parse::Parser;

#[proc_macro_derive(ErrorMessageNew)]
pub fn error_message_new(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as DeriveInput);
    let name = input.ident;

    let mut fields_punct = match input.data {
        Data::Struct(DataStruct {
                         fields: Fields::Named(fields),
                         ..
                     }) => {
            fields.named
        },
        _ => panic!("Only structs with named fields can be annotated with ErrorMessageNew"),
    };

    let mut has_message = false;
    for field in fields_punct.iter() {
        if field.ident.as_ref().unwrap().to_string() == "message".to_string() {
            has_message = true;
            break;
        }
    }

    if !has_message {
        panic!("Only structs with a named field 'message: String' can be annotated with ErrorMessageNew")
    }

    let modified = quote! {
        impl #name {
            fn new<S: Into<String>>(message: S) -> Self {
                #name {
                    message: message.into()
                }
            }
        }
    };
    modified.into()
}
