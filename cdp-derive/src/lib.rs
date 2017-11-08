// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

#![recursion_limit = "128"]
#![cfg_attr(feature = "strict", deny(warnings))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![doc(html_root_url = "https://docs.rs/cdp-derive/0.1.0")]

extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use quote::Tokens;
use std::mem;
use syn::{Attribute, Body, DeriveInput, Generics, Ident, LifetimeDef, Lit, MetaItem, TyGenerics,
          Variant, VariantData, WhereClause};

#[proc_macro_derive(DeserializeCdpCommand, attributes(cdp))]
pub fn derive_deserialize_cdp_command(input: TokenStream) -> TokenStream {
    let input = syn::parse_derive_input(&input.to_string()).unwrap();
    match generate_cdp_deserialize_impl(&input, "Command") {
        Ok(expanded) => expanded.parse().unwrap(),
        Err(msg) => panic!(msg),
    }
}

#[proc_macro_derive(DeserializeCdpEvent, attributes(cdp))]
pub fn derive_deserialize_cdp_event(input: TokenStream) -> TokenStream {
    let input = syn::parse_derive_input(&input.to_string()).unwrap();
    match generate_cdp_deserialize_impl(&input, "Event") {
        Ok(expanded) => expanded.parse().unwrap(),
        Err(msg) => panic!(msg),
    }
}

fn generate_cdp_deserialize_impl(input: &DeriveInput, kind: &str) -> Result<Tokens, String> {
    let DeriveInput {
        ref ident,
        ref generics,
        ref body,
        ..
    } = *input;

    let (quantification, ty_generics, where_clause, unique_lifetime_prefix) =
        generate_cdp_deserialize_impl_generics(generics);
    let de_lifetime = Ident::from(format!("'{}de", unique_lifetime_prefix));

    let variants = match *body {
        Body::Enum(ref variants) => variants,
        _ => return Err("expected an enum definition".into()),
    };

    let mut match_arms = Vec::new();
    let mut maybe_wildcard_arm = None;
    let mut new_predicates = Vec::new();
    for variant in variants {
        generate_cdp_deserialize_impl_arm(
            kind,
            ident,
            variant,
            &unique_lifetime_prefix,
            &de_lifetime,
            &mut match_arms,
            &mut maybe_wildcard_arm,
            &mut new_predicates,
        )?;
    }

    let new_where_clause = if where_clause.predicates.is_empty() {
        if new_predicates.is_empty() {
            None
        } else {
            Some(quote! { where #(#new_predicates, )* })
        }
    } else {
        Some(quote! { #where_clause, #(#new_predicates, )* })
    };

    let wildcard_arm = match maybe_wildcard_arm {
        Some(wildcard_arm) => wildcard_arm,
        None => quote! { _ => { Err(params) } },
    };

    let const_ident =
        Ident::new(format!("_IMPL_DESERIALIZE_CDP_{}_FOR_{}", kind.to_uppercase(), ident));
    let trait_name = Ident::from(format!("DeserializeCdp{}", kind));
    let deserialize_fn_name = Ident::from(format!("deserialize_{}", kind.to_lowercase()));

    Ok(quote! {
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const #const_ident: () = {
            extern crate cdp;
            extern crate serde;

            impl #quantification cdp::#trait_name <#de_lifetime>
                    for #ident #ty_generics #new_where_clause {
                fn #deserialize_fn_name<D>(
                    name: &str,
                    params: D,
                ) -> ::std::result::Result<::std::result::Result<Self, D::Error>, D>
                where
                    D: serde::Deserializer<#de_lifetime>
                {
                    match name {
                        #(#match_arms, )*
                        #wildcard_arm,
                    }
                }
            }
        };
    })
}

fn generate_cdp_deserialize_impl_generics(
    generics: &Generics,
) -> (Tokens, TyGenerics, &WhereClause, String) {
    let max_lifetime_len = generics
        .lifetimes
        .iter()
        .map(|x| x.lifetime.ident.as_ref().len() - 1)
        .max()
        .unwrap_or(0);
    let unique_lifetime_prefix = "_".repeat(max_lifetime_len);

    let mut quant_generics = generics.clone();
    quant_generics
        .lifetimes
        .insert(0, LifetimeDef::new(format!("'{}de", unique_lifetime_prefix)));
    let (quantification, _, _) = quant_generics.split_for_impl();

    let (_, ty_generics, where_clause) = generics.split_for_impl();
    (quote!(#quantification), ty_generics, where_clause, unique_lifetime_prefix)
}

#[cfg_attr(feature = "clippy", allow(too_many_arguments))]
fn generate_cdp_deserialize_impl_arm(
    kind: &str,
    ident: &Ident,
    variant: &Variant,
    unique_lifetime_prefix: &str,
    de_lifetime: &Ident,
    match_arms: &mut Vec<Tokens>,
    maybe_wildcard_arm: &mut Option<Tokens>,
    new_predicates: &mut Vec<Tokens>,
) -> Result<(), String> {
    if maybe_wildcard_arm.is_some() {
        return Err(format!(
            "any 'wildcard' {} variant (with 2 fields) must come last in the enumeration",
            kind.to_lowercase()
        ));
    }

    let variant_ident = &variant.ident;
    let variant_fields = variant.data.fields();
    let ctor = quote! { #ident::#variant_ident };

    let mut maybe_method_name = None;
    extract_method_name_from_attrs(&ctor, &variant.attrs, &mut maybe_method_name)?;

    match variant_fields.len() {
        0 => {
            let method_name = match maybe_method_name {
                Some(method_name) => method_name,
                None => {
                    return Err(format!(
                        "unit variant `{}` is missing a #[cdp = \"...\"] attribute to specify \
                         the {} name",
                        ctor,
                        kind.to_lowercase()
                    ))
                }
            };

            let suffix = match variant.data {
                VariantData::Struct(_) => Some(quote!({})),
                VariantData::Tuple(_) => Some(quote!(())),
                VariantData::Unit => None,
            };
            match_arms.push(quote! {
                #method_name => {
                    Ok(serde::Deserialize::deserialize(params).map(|cdp::Empty| {
                        #ctor#suffix
                    }))
                }
            });

            Ok(())
        }
        1 => {
            let params = &variant_fields[0];
            let params_type = &params.ty;

            let (pattern, prefix_bound) = match maybe_method_name {
                Some(method_name) => (quote!(#method_name), None),
                None => {
                    let kind_trait = Ident::from(format!("Cdp{}", kind));
                    let name_const = Ident::from(format!("{}_NAME", kind.to_uppercase()));

                    let pattern = quote!(<#params_type as cdp::#kind_trait>::#name_const);
                    let prefix_bound = quote! { cdp::#kind_trait + };
                    (pattern, Some(prefix_bound))
                }
            };

            let populate = match params.ident {
                None => quote! { #ctor },
                Some(ref params_ident) => {
                    quote! { |params| { #ctor { #params_ident: params } } }
                }
            };
            match_arms.push(quote! {
                #pattern => { Ok(serde::Deserialize::deserialize(params).map(#populate)) }
            });

            new_predicates.push(quote! {
                #params_type: #prefix_bound serde::Deserialize<#de_lifetime>
            });

            Ok(())
        }
        2 => {
            let name = &variant_fields[0];
            let name_ty = &name.ty;

            let params = &variant_fields[1];
            let params_ty = &params.ty;

            let convert_name = quote! { ::std::convert::From::from(name) };
            let field_idents = name.ident.as_ref().and_then(|name_ident| {
                params
                    .ident
                    .as_ref()
                    .map(|params_ident| (name_ident, params_ident))
            });
            let populate = match field_idents {
                None => quote! { |params| { #ctor(#convert_name, params) } },
                Some((name_ident, params_ident)) => {
                    quote! {
                        |params| { #ctor { #name_ident: #convert_name, #params_ident: params } }
                    }
                }
            };
            mem::replace(
                maybe_wildcard_arm,
                Some(quote! {
                    _ => { Ok(serde::Deserialize::deserialize(params).map(#populate)) }
                }),
            );

            let str_lifetime = Ident::from(format!("'{}a", unique_lifetime_prefix));
            new_predicates.push(quote! { #name_ty: for<#str_lifetime> From<&#str_lifetime str> });
            new_predicates.push(quote! { #params_ty: serde::Deserialize<#de_lifetime> });

            Ok(())
        }
        n => Err(format!("expected 0, 1, or 2 fields on {}, but found {}", ctor, n)),
    }
}

fn extract_method_name_from_attrs(
    target: &Tokens,
    attrs: &[Attribute],
    maybe_method_name: &mut Option<String>,
) -> Result<(), String> {
    for attr in attrs {
        match attr.value {
            MetaItem::NameValue(ref ident, Lit::Str(ref text, _)) => if ident.as_ref() == "cdp" {
                if maybe_method_name.is_none() {
                    mem::replace(maybe_method_name, Some(text.clone()));
                } else {
                    return Err(format!("multiple `cdp` attributes attached to `{}`", target));
                }
            },
            MetaItem::Word(ref ident) |
            MetaItem::List(ref ident, _) |
            MetaItem::NameValue(ref ident, _) => if ident.as_ref() == "cdp" {
                return Err("`cdp` attribute must be used in #[cdp = \"...\"] form".into());
            },
        }
    }
    Ok(())
}
