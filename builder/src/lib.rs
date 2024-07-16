use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse2, parse_macro_input, DeriveInput, GenericArgument, Ident, LitStr, Meta, MetaList,
    PathArguments, Token, Type,
};

fn is_option(ty: &Type) -> bool {
    if let Type::Path(typepath) = ty {
        if typepath.qself.is_none() && typepath.path.segments.len() == 1 {
            let segment = &typepath.path.segments[0];
            if segment.ident == "Option" {
                if let PathArguments::AngleBracketed(ref args) = segment.arguments {
                    return args.args.len() == 1;
                }
            }
        }
    }
    false
}

fn get_option_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(typepath) = ty {
        if typepath.qself.is_none() && typepath.path.segments.len() == 1 {
            let segment = &typepath.path.segments[0];
            if segment.ident == "Option" {
                if let PathArguments::AngleBracketed(ref args) = segment.arguments {
                    if args.args.len() == 1 {
                        if let GenericArgument::Type(ref inner_ty) = args.args[0] {
                            return Some(inner_ty);
                        }
                    }
                }
            }
        }
    }
    None
}

fn get_vec_inner_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(typepath) = ty {
        if typepath.qself.is_none() && typepath.path.segments.len() == 1 {
            let segment = &typepath.path.segments[0];
            if segment.ident == "Vec" {
                if let PathArguments::AngleBracketed(ref args) = segment.arguments {
                    if args.args.len() == 1 {
                        if let GenericArgument::Type(ref inner_ty) = args.args[0] {
                            return Some(inner_ty);
                        }
                    }
                }
            }
        }
    }
    None
}

fn is_builder_of(f: &syn::Field) -> Option<&syn::Attribute> {
    for attr in &f.attrs {
        if attr.path().segments.len() == 1 && attr.path().segments[0].ident == "builder" {
            return Some(attr);
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
    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = input.data
    {
        named
    } else {
        unimplemented!();
    };

    let builder_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;

        if is_option(&ty) || is_builder_of(&f).is_some() && get_each_value(f).is_none() {
            quote! {
                pub #name: #ty,
            }
        } else {
            quote! {
                pub #name: Option<#ty>,
            }
        }
    });

    let build_function_assign_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;

     if is_option(&ty) || is_builder_of(&f).is_some() && get_each_value(&f).is_none() {
            quote! {
                #name: self.#name.take(),
            }
        } else {
            quote! {
                #name: self.#name.take().ok_or_else(|| format!("{} is missing", stringify!(#name)))?,
            }
        }
    });

    let builder_defaults = fields.iter().map(|f| {
        let name = &f.ident;

        if let Some(_) = get_vec_inner_type(&f.ty) {
            quote! {
                #name: Some(Vec::new()),
            }
        } else {
            quote! {
                #name: None,
            }
        }
    });

    let builder_methods = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;

        if let Some(each) = get_each_value(f) {
            let inner = get_vec_inner_type(&ty).unwrap_or(&ty);
            quote! {
                pub fn #each(&mut self, item: #inner) -> &mut Self {
                    self.#name.as_mut().map(|v| v.push(item));
                self
                }
            }
        } else {
            let inner = get_option_inner_type(&ty).unwrap_or(&ty);
            quote! {
                pub fn #name(&mut self, #name: #inner) -> &mut Self {
                self.#name = Some(#name);
                self
                }
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

fn get_each_value(f: &syn::Field) -> Option<Ident> {
    let attr = f.attrs.first()?;
    let meta = &attr.meta;

    match meta {
        Meta::List(MetaList {
            path,
            delimiter: _,
            tokens,
        }) if path.is_ident("builder") => {
            let each_input: EachInput = parse2(tokens.to_owned()).unwrap();
            if each_input.each == "each" {
                Some(each_input.value)
            } else {
                None
            }
        }
        _ => None,
    }
}

#[derive(Debug)]
struct EachInput {
    each: Ident,
    value: Ident,
}

impl syn::parse::Parse for EachInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let each: Ident = input.parse()?;
        let _eq_token: Token![=] = input.parse()?;
        let value: LitStr = input.parse()?;
        let value: Ident = Ident::new(value.value().as_str(), value.span());

        Ok(EachInput { each, value })
    }
}
