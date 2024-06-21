use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let builder_name = syn::Ident::new(&format!("{}Builder", name), name.span());

    // Iterate through the fields of the struct
    let fields = if let Data::Struct(ref data) = input.data {
        if let Fields::Named(ref fields) = data.fields {
            &fields.named
        } else {
            unimplemented!()
        }
    } else {
        unimplemented!()
    };

    let builder_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        quote! {
            pub #name: Option<#ty>,
        }
    });

    let build_function_assign_fields = fields.iter().map(|f| {
        let name = &f.ident;
        quote! {
            #name: self.#name.take().ok_or_else(|| format!("{} is missing", stringify!(#name)))?,
        }
    });

    let builder_defaults = fields.iter().map(|f| {
        let name = &f.ident;
        quote! {
            #name: None,
        }
    });

    let builder_methods = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        quote! {
            pub fn #name(&mut self, #name: #ty) -> &mut Self {
                self.#name = Some(#name);
                self
            }
        }
    });

    let expanded = quote! {
        use std::error::Error;

        pub struct #builder_name {
            #(#builder_fields)*
        }

        impl #name {
            pub fn builder() -> #builder_name {
                #builder_name {
                    #(#builder_defaults)*
                }
            }
        }

        impl #builder_name {

            pub fn build(&mut self) -> Result<#name, Box<dyn Error>>  {
                Ok(#name {
                    #(#build_function_assign_fields)*
                })
            }

            #(#builder_methods)*
        }
    };

    proc_macro::TokenStream::from(expanded)
}
