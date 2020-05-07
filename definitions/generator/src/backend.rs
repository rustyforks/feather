use crate::frontend::{Data, Enum, Value};
use crate::model::Type;
use anyhow::Context;
use heck::CamelCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::process::Command;
use syn::export::ToTokens;

/// Given the `Data`, generates code.
pub fn generate(target_dir: &str, data: &Data) -> anyhow::Result<()> {
    let _ = std::fs::remove_dir(target_dir);
    std::fs::create_dir_all(target_dir)
        .with_context(|| format!("failed to create directory `{}`", target_dir))?;

    let mut open_files = HashMap::new();

    let mut module_names = HashSet::new();

    for e in data.enums.values() {
        let path = format!("{}/{}.rs", target_dir, e.file);
        module_names.insert(e.file.clone());

        let file = match open_files.get_mut(&path) {
            Some(file) => file,
            None => {
                let mut file = File::create(&path)
                    .with_context(|| format!("failed to create file `{}`", path))?;
                file.write_all(b"// This file is @generated\n")
                    .with_context(|| format!("failed to write to file `{}`", path))?;
                open_files.insert(path.clone(), file);
                open_files.get_mut(&path).unwrap()
            }
        };

        let tokens = generate_enum(e);

        file.write_all(tokens.to_string().as_bytes())
            .with_context(|| format!("failed to write bytes to `{}`", path))?;
    }

    // Write out mod.rs
    let lib_path = format!("{}/mod.rs", target_dir);
    let mut lib = File::create(&lib_path)?;
    lib.write_all(b"// This file is @generated\n")?;
    for module in module_names {
        lib.write_all(format!("mod {}; pub use {}::*;", module, module).as_bytes())?;
    }
    open_files.insert(lib_path, lib);

    for (path, mut file) in open_files {
        file.flush()?;

        if !Command::new("rustfmt").arg(&path).status()?.success() {
            anyhow::bail!("failed to run rustfmt on file {}", path);
        }
    }

    Ok(())
}

fn generate_enum(e: &Enum) -> TokenStream {
    let def = generate_enum_body(e);
    let imp = generate_enum_functions(e);

    quote! {
        #def
        #imp
    }
}

fn generate_enum_body(e: &Enum) -> TokenStream {
    let name = ident(&e.name_camel_case);
    let variants: Vec<_> = e.variants_camel_case.iter().map(ident).collect();

    quote! {
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ToPrimitive, FromPrimitive)]
        pub enum #name {
            #(#variants,)*
        }
    }
}

impl ToTokens for Type {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let t = match self {
            Type::Bool => quote! { bool },
            Type::Slice(inner) => quote! { &'static [#inner] },
            Type::U32 => quote! { u32 },
            Type::F64 => quote! { f64 },
            Type::String => quote! { &'static str },
            Type::Custom(name) => {
                let name = ident(name.to_camel_case());
                quote! { #name }
            }
        };
        tokens.extend(t);
    }
}

impl ToTokens for Value {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let t = match self {
            Value::Bool(x) => quote! { #x },
            Value::Slice(x) => quote! { [#(#x),*] },
            Value::U32(x) => quote! { #x },
            Value::F64(x) => quote! { #x },
            Value::String(x) => quote! { #x },
            Value::Custom(name) => {
                let name = ident(name.to_camel_case());
                quote! { #name }
            }
        };
        tokens.extend(t);
    }
}

fn generate_enum_functions(e: &Enum) -> TokenStream {
    let name = ident(&e.name_camel_case);

    let mut fns = vec![];

    for property in e.properties.values() {
        let property_name = ident(&property.name);
        let property_type = &property.typ;

        let exhaustive = property.mapping.len() == e.variants.len();

        let mut match_arms = vec![];
        for (variant, value) in &property.mapping {
            let variant = ident(variant.to_camel_case());

            let value = if exhaustive {
                quote! { #value }
            } else {
                quote! { Some(#value) }
            };

            match_arms.push(quote! {
                #variant => #value,
            });
        }

        if !exhaustive {
            match_arms.push(quote! {
                _ => None,
            });
        }

        let f = quote! {
            pub fn #property_name(self) -> crate::#property_type {
                use #name::*;
                match self {
                    #(#match_arms)*
                }
            }
        };
        fns.push(f);
    }

    let tokens = quote! {
        impl #name {
            #(#fns)*
        }
    };
    tokens
}

fn ident(s: impl AsRef<str>) -> Ident {
    Ident::new(s.as_ref(), Span::call_site())
}
