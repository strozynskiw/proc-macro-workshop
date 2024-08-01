use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse2, parse_macro_input, Data, DeriveInput, Fields, GenericArgument, Ident, LitStr, Meta,
    PathArguments, Token, Type, TypePath,
};

fn get_inner_type_of<'a, 'b>(typepath: &'a TypePath, wrapper: &'b str) -> Option<&'a Type> {
    let segment = &typepath.path.segments[0];
    if segment.ident == wrapper {
        if let PathArguments::AngleBracketed(ref args) = segment.arguments {
            if args.args.len() == 1 {
                if let GenericArgument::Type(ref inner_ty) = args.args[0] {
                    return Some(inner_ty);
                }
            }
        }
    }
    None
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let builder_name = syn::Ident::new(&format!("{}Builder", name), name.span());

    // Iterate through the fields of the struct
    let Data::Struct(data_struct) = &input.data else {
        return syn::Error::new_spanned(name, format!("invalid derive type"))
            .to_compile_error()
            .into();
    };

    let Fields::Named(named_fields) = &data_struct.fields else {
        return syn::Error::new_spanned(name, format!("expected named fields"))
            .to_compile_error()
            .into();
    };

    let mut builder_fields: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut build_function_assign_fields: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut builder_defaults: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut builder_methods: Vec<proc_macro2::TokenStream> = Vec::new();

    for field in &named_fields.named {
        let mut each: Option<Ident> = None;
        // Attributes like `#[builder(each = "arg")]` are parsed here
        for attr in &field.attrs {
            let p = attr.meta.path().segments.first();
            if p.is_none() || p.unwrap().ident != "builder" {
                continue;
            }

            // Process actual attributes  from here
            let Meta::List(meta) = &attr.meta else {
                continue;
            };

            let attr_input: AttrInput = parse2(meta.tokens.to_owned()).unwrap();

            if attr_input.ident != "each" {
                return syn::Error::new_spanned(
                    meta,
                    format!("expected `builder(each = \"...\")`"),
                )
                .to_compile_error()
                .into();
            }

            each = Some(attr_input.value);
        }

        let name = &field.ident;
        let ty = &field.ty;

        let Type::Path(path) = ty else { continue };

        if let Some(option_type) = get_inner_type_of(path, "Option") {
            builder_fields.push(quote! {
                pub #name: #ty,
            });

            build_function_assign_fields.push(quote! {
                #name: self.#name.take(),
            });

            builder_defaults.push(quote! {
                #name: core::option::Option::None,
            });

            builder_methods.push(quote! {
                pub fn #name(&mut self, #name: #option_type) -> &mut Self {
                self.#name = core::option::Option::Some(#name);
                self
                }
            });
        } else if let Some(each) = each {
            if let Some(vec_type) = get_inner_type_of(path, "Vec") {
                builder_fields.push(quote! {
                    pub #name: core::option::Option<#ty>,
                });

                build_function_assign_fields.push(quote! {
                    #name: self.#name.take().ok_or_else(|| format!("{} is missing", stringify!(#name)))?,
                });

                builder_defaults.push(quote! {
                    #name: core::option::Option::Some(Vec::new()),
                });

                builder_methods.push(quote! {
                    pub fn #each(&mut self, item: #vec_type) -> &mut Self {
                        self.#name.as_mut().map(|v| v.push(item));
                    self
                    }
                });
            } else {
                unimplemented!() //put error here
            }
        } else {
            builder_fields.push(quote! {
                pub #name: core::option::Option<#ty>,
            });

            build_function_assign_fields.push(quote! {
                #name: self.#name.take().ok_or_else(|| format!("{} is missing", stringify!(#name)))?,
            });

            builder_defaults.push(quote! {
                #name: core::option::Option::None,
            });

            builder_methods.push(quote! {
                pub fn #name(&mut self, #name: #ty) -> &mut Self {
                self.#name = core::option::Option::Some(#name);
                self
                }
            });
        }
    }

    let expanded = quote! {

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

            pub fn build(&mut self) -> core::result::Result<#name, std::boxed::Box<dyn std::error::Error>>  {
                Ok(#name {
                    #(#build_function_assign_fields)*
                })
            }

            #(#builder_methods)*
        }
    };
    expanded.into()
}

#[derive(Debug)]
struct AttrInput {
    ident: Ident,
    value: Ident,
}

impl syn::parse::Parse for AttrInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        let _eq_token: Token![=] = input.parse()?;
        let value: LitStr = input.parse()?;
        let value: Ident = Ident::new(value.value().as_str(), value.span());

        Ok(AttrInput { ident, value })
    }
}
