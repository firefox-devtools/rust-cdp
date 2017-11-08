// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

#![recursion_limit = "128"]
#![cfg_attr(feature = "strict", deny(warnings))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![cfg_attr(feature = "clippy", allow(too_many_arguments))]
#![cfg_attr(feature = "clippy", allow(trivial_regex))] // false positive

extern crate inflector;
#[macro_use]
extern crate lazy_static;
extern crate petgraph;
#[macro_use]
extern crate quote;
extern crate regex;
extern crate rustfmt;
extern crate serde_json;

use inflector::Inflector;
use petgraph::Directed;
use petgraph::graph::{Graph, NodeIndex};
use petgraph::visit::{Control, DfsEvent};
use quote::{Ident, Tokens};
use regex::Regex;
use rustfmt::Input;
use rustfmt::config::Config;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fmt;
use std::fs::File;
use std::io::{self, Write};
use std::iter;
use std::path::Path;

extern crate cdp_definition;

use cdp_definition::{Definition, Domain, Field, Method, Type, TypeDef, Version};

fn main() {
    let out_dir = env::var("OUT_DIR").expect("error retrieving OUT_DIR environment variable");

    let mut browser_protocol: Definition = serde_json::from_str(
        include_str!("../../json/browser_protocol.json"),
    ).expect("error parsing browser_protocol.json");

    let js_protocol: Definition = serde_json::from_str(
        include_str!("../../json/js_protocol.json"),
    ).expect("error parsing js_protocol.json");

    if browser_protocol.version != js_protocol.version {
        panic!("json/browser_protocol.json and json/js_protocol.json versions don't match");
    }

    browser_protocol
        .domains
        .extend_from_slice(&js_protocol.domains);

    let generated_src = generate_rust_source(&browser_protocol);
    let generated_path = Path::new(&out_dir).join("generated.rs");
    let mut generated_file = File::create(generated_path).expect("error creating generated.rs");
    write_generated_source(generated_src, &mut generated_file)
        .expect("error writing generated.rs");

    println!("cargo:rerun-if-changed=../json/browser_protocol.json");
    println!("cargo:rerun-if-changed=../json/js_protocol.json");
}

fn generate_rust_source(def: &Definition) -> String {
    let version = generate_version(&def.version);
    let domains = generate_domains(&def.domains);

    quote!(#version #domains).to_string()
}

fn write_generated_source<T>(src: String, out: &mut T) -> Result<(), io::Error>
where
    T: Write,
{
    let mut config = Config::default();
    config.override_value("error_on_line_overflow", "false");
    config.override_value("skip_children", "true");
    config.override_value("write_mode", "plain");

    let result = rustfmt::format_input(Input::Text(src), &config, Some(out));
    let (summary, _, report) = result.map_err(|x| x.0)?;
    if !summary.has_no_errors() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "rustfmt error"));
    }
    if report.has_warnings() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, report.to_string()));
    }

    Ok(())
}

fn generate_version(version: &Version) -> Tokens {
    let version_string = version.to_string();

    (quote! {
        #[macro_export]
        macro_rules! cdp_stable_protocol_version {
            () => ( #version_string )
        }

        pub const STABLE_PROTOCOL_VERSION: &str = cdp_stable_protocol_version!();
    })
}

fn generate_domains(domains: &[Domain]) -> Tokens {
    let uses_lifetime_set = generate_uses_lifetime_set(domains);
    let modules = domains
        .iter()
        .map(|domain| generate_domain(domain, &uses_lifetime_set));
    quote!(#(#modules)*)
}

fn generate_uses_lifetime_set(domains: &[Domain]) -> HashSet<Ident> {
    let mut reference_graph = Graph::new();
    let mut item_indices = HashMap::new();

    let string_index = reference_graph.add_node(None);

    fn item_index(
        item_fully_qualified: Ident,
        reference_graph: &mut Graph<Option<Ident>, (), Directed>,
        item_indices: &mut HashMap<Ident, NodeIndex>,
    ) -> NodeIndex {
        if let Some(index) = item_indices.get(&item_fully_qualified) {
            return *index;
        }

        let index = reference_graph.add_node(Some(item_fully_qualified.clone()));
        item_indices.insert(item_fully_qualified, index);
        index
    }

    fn traverse_fields<'a, T>(
        domain_snake_case: &Ident,
        parent_pascal_case: &Ident,
        fields: T,
        string_index: NodeIndex,
        reference_graph: &mut Graph<Option<Ident>, (), Directed>,
        item_indices: &mut HashMap<Ident, NodeIndex>,
    ) where
        T: Iterator<Item = &'a Field>,
    {
        for field in fields {
            traverse_type(
                domain_snake_case,
                parent_pascal_case,
                &field.ty,
                string_index,
                reference_graph,
                item_indices,
            )
        }
    }

    fn traverse_type(
        domain_snake_case: &Ident,
        parent_pascal_case: &Ident,
        ty: &Type,
        string_index: NodeIndex,
        reference_graph: &mut Graph<Option<Ident>, (), Directed>,
        item_indices: &mut HashMap<Ident, NodeIndex>,
    ) {
        match *ty {
            Type::String => {
                let parent_fully_qualified =
                    fully_qualified_ident(domain_snake_case, parent_pascal_case);
                let parent_index =
                    item_index(parent_fully_qualified, reference_graph, item_indices);
                reference_graph.add_edge(string_index, parent_index, ());
            }
            Type::Reference(ref target) => {
                let target_pascal_case = pascal_case_ident(target);
                if target_pascal_case != parent_pascal_case {
                    let target_fully_qualified =
                        resolve_reference(domain_snake_case, target, &target_pascal_case);
                    let target_index =
                        item_index(target_fully_qualified, reference_graph, item_indices);

                    let parent_fully_qualified =
                        fully_qualified_ident(domain_snake_case, parent_pascal_case);
                    let parent_index =
                        item_index(parent_fully_qualified, reference_graph, item_indices);

                    reference_graph.add_edge(target_index, parent_index, ());
                }
            }
            Type::Array { ref item, .. } => {
                traverse_type(
                    domain_snake_case,
                    parent_pascal_case,
                    &item.ty,
                    string_index,
                    reference_graph,
                    item_indices,
                );
            }
            Type::Object(ref fields) => traverse_fields(
                domain_snake_case,
                parent_pascal_case,
                fields.iter(),
                string_index,
                reference_graph,
                item_indices,
            ),
            Type::Boolean | Type::Integer | Type::Number | Type::Any | Type::Enum(_) => (),
        }
    }

    for domain in domains.iter() {
        let domain_snake_case = snake_case_ident(&domain.name);
        let domain_methods = domain.commands.iter().chain(domain.events.iter());
        for method in domain_methods {
            let method_pascal_case = pascal_case_ident(&method.name);
            let method_fields = method.parameters.iter().chain(method.returns.iter());
            traverse_fields(
                &domain_snake_case,
                &method_pascal_case,
                method_fields,
                string_index,
                &mut reference_graph,
                &mut item_indices,
            );
        }
        for type_def in &domain.type_defs {
            let type_def_pascal_case = pascal_case_ident(&type_def.name);
            traverse_type(
                &domain_snake_case,
                &type_def_pascal_case,
                &type_def.ty,
                string_index,
                &mut reference_graph,
                &mut item_indices,
            );
        }
    }

    // 1 = starting String node which won't make it into the final set
    let mut uses_lifetime_set = HashSet::with_capacity(reference_graph.node_count() - 1);

    petgraph::visit::depth_first_search(&reference_graph, iter::once(string_index), |event| {
        if let DfsEvent::Discover(item_index, _) = event {
            if let Some(ref item_fully_qualified) =
                *reference_graph.node_weight(item_index).unwrap()
            {
                uses_lifetime_set.insert(item_fully_qualified.clone());
            }
        }
        Control::Continue::<()>
    });

    uses_lifetime_set.shrink_to_fit();
    uses_lifetime_set
}

#[derive(Clone, Copy)]
enum MethodKind {
    Command,
    Event,
}

impl fmt::Display for MethodKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MethodKind::Command => write!(f, "Command"),
            MethodKind::Event => write!(f, "Event"),
        }
    }
}

fn generate_domain(domain: &Domain, uses_lifetime_set: &HashSet<Ident>) -> Tokens {
    let domain_snake_case = snake_case_ident(&domain.name);

    let deprecation_status = DeprecationStatus::new(domain.deprecated, &domain.description);

    let mut type_defs = vec![];
    let mut domain_index = format!("# {}\n\n", domain.name);

    if !domain.commands.is_empty() {
        domain_index.push_str("## Commands\n\n");
        for command in &domain.commands {
            generate_method(
                domain,
                &domain_snake_case,
                &deprecation_status,
                MethodKind::Command,
                command,
                uses_lifetime_set,
                &mut domain_index,
                &mut type_defs,
            );
        }
    }

    if !domain.events.is_empty() {
        domain_index.push_str("\n## Events\n\n");
        for event in &domain.events {
            generate_method(
                domain,
                &domain_snake_case,
                &deprecation_status,
                MethodKind::Event,
                event,
                uses_lifetime_set,
                &mut domain_index,
                &mut type_defs,
            );
        }
    }

    if !domain.type_defs.is_empty() {
        domain_index.push_str("\n##Types\n\n");
        for type_def in &domain.type_defs {
            generate_type_def(
                domain,
                &domain_snake_case,
                &deprecation_status,
                type_def,
                uses_lifetime_set,
                &mut domain_index,
                &mut type_defs,
            );
        }
    }

    let meta_attrs = generate_meta_attrs(
        &deprecation_status,
        domain.experimental,
        &domain.description,
        if domain_index.is_empty() {
            None
        } else {
            Some(domain_index)
        },
    );

    quote! {
        #meta_attrs
        pub mod #domain_snake_case {
            #(#type_defs)*
        }
    }
}

fn generate_type_def(
    domain: &Domain,
    domain_snake_case: &Ident,
    domain_deprecation_status: &DeprecationStatus,
    type_def: &TypeDef,
    uses_lifetime_set: &HashSet<Ident>,
    domain_index: &mut String,
    type_defs: &mut Vec<Tokens>,
) {
    let type_def_pascal_case = pascal_case_ident(&type_def.name);

    let deprecation_status = DeprecationStatus::new(type_def.deprecated, &type_def.description)
        .add_parent(domain_deprecation_status);
    let experimental = domain.experimental || type_def.experimental;

    let (maybe_expr, uses_lifetime) = generate_type_expr_impl(
        domain_snake_case,
        &type_def_pascal_case,
        None,
        &deprecation_status,
        experimental,
        &type_def.description,
        &type_def.ty,
        uses_lifetime_set,
        type_defs,
    );

    let category = match type_def.ty {
        Type::Object(_) => "struct",
        Type::Enum(_) => "enum",
        _ => "type",
    };

    let index_entry = generate_index_entry(
        &type_def.name,
        &format!("{}.{}.html", category, type_def_pascal_case),
        &deprecation_status,
        experimental,
        &type_def.description,
    );
    domain_index.push_str(&index_entry);

    if let Some(expr) = maybe_expr {
        let meta_attrs =
            generate_meta_attrs(&deprecation_status, experimental, &type_def.description, None);
        let lifetime_generics = generate_lifetime_generics(uses_lifetime);
        type_defs.push(quote! {
            #meta_attrs
            pub type #type_def_pascal_case#lifetime_generics = #expr;
        });
    }
}

fn generate_type_expr(
    domain_snake_case: &Ident,
    parent_pascal_case: &Ident,
    field_name: Option<&String>,
    deprecation_status: &DeprecationStatus,
    experimental: bool,
    ty: &Type,
    uses_lifetime_set: &HashSet<Ident>,
    type_defs: &mut Vec<Tokens>,
) -> (Tokens, bool) {
    let (maybe_expr, uses_lifetime) = generate_type_expr_impl(
        domain_snake_case,
        parent_pascal_case,
        field_name,
        deprecation_status,
        experimental,
        &None,
        ty,
        uses_lifetime_set,
        type_defs,
    );

    let expr = match maybe_expr {
        Some(expr) => expr,
        None => {
            let type_def_pascal_case = combine_parent_field_idents(parent_pascal_case, field_name);
            let type_def_lifetime_generics = generate_lifetime_generics(uses_lifetime);
            quote!(::#domain_snake_case::#type_def_pascal_case#type_def_lifetime_generics)
        }
    };
    (expr, uses_lifetime)
}

fn generate_type_expr_impl(
    domain_snake_case: &Ident,
    parent_pascal_case: &Ident,
    field_name: Option<&String>,
    deprecation_status: &DeprecationStatus,
    experimental: bool,
    description: &Option<String>,
    ty: &Type,
    uses_lifetime_set: &HashSet<Ident>,
    type_defs: &mut Vec<Tokens>,
) -> (Option<Tokens>, bool) {
    match *ty {
        Type::Reference(ref target) => {
            let target_pascal_case = pascal_case_ident(target);
            let target_fully_qualified =
                resolve_reference(domain_snake_case, target, &target_pascal_case);
            let target_uses_lifetime = uses_lifetime_set.contains(&target_fully_qualified);
            let target_lifetime_generics = generate_lifetime_generics(target_uses_lifetime);
            let target_expr = if target_pascal_case == parent_pascal_case {
                quote! { Box<#target_fully_qualified#target_lifetime_generics> }
            } else {
                quote! { #target_fully_qualified#target_lifetime_generics }
            };
            (Some(target_expr), target_uses_lifetime)
        }
        Type::Boolean => (Some(quote! { bool }), false),
        Type::Integer => (Some(quote! { i32 }), false),
        Type::Number => (Some(quote! { f64 }), false),
        Type::String => (Some(quote! { ::std::borrow::Cow<'a, str> }), true),
        Type::Enum(ref values) => {
            let type_def_pascal_case = combine_parent_field_idents(parent_pascal_case, field_name);
            let note =
                generate_field_usage_note(domain_snake_case, parent_pascal_case, field_name);
            let meta_attrs =
                generate_meta_attrs(deprecation_status, experimental, description, note);
            let variants: Vec<Tokens> = values
                .iter()
                .map(|s| generate_type_enum_variant(s))
                .collect();

            let variant_ctors: Vec<Tokens> = values
                .iter()
                .map(|value| {
                    let value_pascal_case = pascal_case_ident(value);
                    quote! { #type_def_pascal_case::#value_pascal_case }
                })
                .collect();

            let parse_arms: Vec<Tokens> = values
                .iter()
                .zip(variant_ctors.iter())
                .map(|(value, ctor)| {
                    quote! { #value => { Ok(#ctor) } }
                })
                .collect();
            let fmt_arms: Vec<Tokens> = values
                .iter()
                .zip(variant_ctors.iter())
                .map(|(value, ctor)| {
                    quote! { #ctor => { #value } }
                })
                .collect();

            type_defs.push(quote! {
                #[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, Ord,
                         PartialOrd, Hash)]
                #meta_attrs
                pub enum #type_def_pascal_case {
                    #(#variants, )*
                }

                impl #type_def_pascal_case {
                    pub const ENUM_VALUES: &'static [#type_def_pascal_case] =
                        &[#(#variant_ctors),*];
                    pub const STR_VALUES: &'static [&'static str] = &[#(#values),*];
                }

                impl ::std::str::FromStr for #type_def_pascal_case {
                    type Err = ::ParseEnumError;

                    fn from_str(s: &str) -> Result<Self, Self::Err> {
                        match s {
                            #(#parse_arms, )*
                            _ => Err(::ParseEnumError {
                                expected: #type_def_pascal_case::STR_VALUES,
                                actual: s.into(),
                            }),
                        }
                    }
                }

                impl ::std::fmt::Display for #type_def_pascal_case {
                    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                        write!(f, "{}", match *self {
                            #(#fmt_arms, )*
                        })
                    }
                }
            });

            (None, false)
        }
        Type::Array {
            ref item,
            min_items,
            max_items,
        } => {
            let (item_expr, item_uses_lifetime) = generate_type_expr(
                domain_snake_case,
                parent_pascal_case,
                field_name,
                deprecation_status,
                experimental,
                &item.ty,
                uses_lifetime_set,
                type_defs,
            );

            let array_expr = match (min_items, max_items) {
                (Some(min), Some(max)) if min == max => {
                    let n = max as usize;
                    quote! { [#item_expr; #n] }
                }
                _ => quote! { Vec<#item_expr> },
            };
            (Some(array_expr), item_uses_lifetime)
        }
        Type::Object(ref properties) => if properties.is_empty() {
            (Some(quote! { ::Empty }), false)
        } else {
            let type_def_pascal_case = combine_parent_field_idents(parent_pascal_case, field_name);
            let note =
                generate_field_usage_note(domain_snake_case, parent_pascal_case, field_name);
            let meta_attrs =
                generate_meta_attrs(deprecation_status, experimental, description, note);

            let mut fields_use_lifetime = false;
            let fields: Vec<Tokens> = properties
                .iter()
                .map(|field| {
                    generate_field(
                        domain_snake_case,
                        &type_def_pascal_case,
                        deprecation_status,
                        experimental,
                        field,
                        uses_lifetime_set,
                        &mut fields_use_lifetime,
                        type_defs,
                    )
                })
                .collect();

            let type_def_lifetime_generics = generate_lifetime_generics(fields_use_lifetime);
            type_defs.push(quote! {
                #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
                #meta_attrs
                pub struct #type_def_pascal_case#type_def_lifetime_generics {
                    #(#fields, )*
                }
            });

            (None, fields_use_lifetime)
        },
        Type::Any => (Some(quote! { ::serde_json::Value }), false),
    }
}

fn generate_field_usage_note(
    domain_snake_case: &Ident,
    parent_pascal_case: &Ident,
    field_name: Option<&String>,
) -> Option<String> {
    field_name.map(|field_name| {
        let field_snake_case = snake_case_ident(field_name);
        format!(
            "Used in the type of [`cdp::{}::{}::{}`](struct.{}.html#structfield.{}).",
            domain_snake_case,
            parent_pascal_case,
            field_snake_case,
            parent_pascal_case,
            field_snake_case,
        )
    })
}

fn generate_type_enum_variant(variant_name: &str) -> Tokens {
    let variant_pascal_case = pascal_case_ident(variant_name);
    let doc_text = format!(r#"Represented as `"{}"`."#, variant_name);

    quote! {
        #[serde(rename = #variant_name)]
        #[doc = #doc_text]
        #variant_pascal_case
    }
}

fn generate_method(
    domain: &Domain,
    domain_snake_case: &Ident,
    domain_deprecation_status: &DeprecationStatus,
    kind: MethodKind,
    method: &Method,
    uses_lifetime_set: &HashSet<Ident>,
    domain_index: &mut String,
    type_defs: &mut Vec<Tokens>,
) {
    let method_qualified = format!("{}.{}", domain.name, method.name);
    let method_pascal_case = pascal_case_ident(&method.name);

    let request_pascal_case = Ident::from(format!("{}{}", method_pascal_case, kind));
    let maybe_response_pascal_case = if let MethodKind::Command = kind {
        Some(Ident::from(format!("{}Response", method_pascal_case)))
    } else {
        None
    };

    let deprecation_status = DeprecationStatus::new(method.deprecated, &method.description)
        .add_parent(domain_deprecation_status);
    let experimental = domain.experimental || method.experimental;

    let index_entry = generate_index_entry(
        &method_qualified,
        &format!("struct.{}.html", request_pascal_case),
        &deprecation_status,
        experimental,
        &method.description,
    );
    domain_index.push_str(&index_entry);

    let note = generate_method_note(
        domain_snake_case,
        &method_qualified,
        &request_pascal_case,
        &maybe_response_pascal_case,
        kind,
    );

    let meta_attrs =
        generate_meta_attrs(&deprecation_status, experimental, &method.description, Some(note));

    let request_lifetime_template = quote!('a);
    let request_uses_lifetime = generate_method_struct(
        domain_snake_case,
        &request_pascal_case,
        &meta_attrs,
        &deprecation_status,
        experimental,
        kind,
        &method_qualified,
        method.parameters.as_slice(),
        uses_lifetime_set,
        type_defs,
    );
    let maybe_request_lifetime = if request_uses_lifetime {
        Some(&request_lifetime_template)
    } else {
        None
    };
    let request_lifetime_generics = maybe_request_lifetime.map(|request_lifetime| {
        quote! { <#request_lifetime> }
    });

    let request_serialize_trait = Ident::from(format!("SerializeCdp{}", kind));
    let request_name_method = Ident::from(format!("{}_name", kind).to_lowercase());
    let request_serialize_params_method =
        Ident::from(format!("serialize_{}_params", kind).to_lowercase());
    type_defs.push(quote! {
        impl#request_lifetime_generics ::#request_serialize_trait
                for #request_pascal_case#request_lifetime_generics {
            fn #request_name_method(&self) -> &str {
                #method_qualified
            }

            fn #request_serialize_params_method<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                ::serde::Serialize::serialize(self, serializer)
            }
        }
    });

    let request_deserialize_trait = Ident::from(format!("DeserializeCdp{}", kind));
    let request_deserialize_method = Ident::from(format!("deserialize_{}", kind).to_lowercase());
    type_defs.push(quote! {
        impl<'de, #maybe_request_lifetime> ::#request_deserialize_trait<'de>
                for #request_pascal_case#request_lifetime_generics {
            fn #request_deserialize_method<D>(
                name: &str,
                params: D,
            ) -> Result<Result<Self, D::Error>, D>
            where
                D: ::serde::Deserializer<'de>,
            {
                if name == #method_qualified {
                    Ok(<#request_pascal_case as ::serde::Deserialize<'de>>::deserialize(params))
                } else {
                    Err(params)
                }
            }
        }
    });

    if let Some(ref response_pascal_case) = maybe_response_pascal_case {
        let response_lifetime_template = quote!('b);
        let response_uses_lifetime = generate_method_struct(
            domain_snake_case,
            response_pascal_case,
            &meta_attrs,
            &deprecation_status,
            experimental,
            kind,
            &method_qualified,
            method.returns.as_slice(),
            uses_lifetime_set,
            type_defs,
        );
        let maybe_response_lifetime = if response_uses_lifetime {
            Some(&response_lifetime_template)
        } else {
            None
        };
        let response_lifetime_generics = maybe_response_lifetime.map(|response_lifetime| {
            quote! { <#response_lifetime> }
        });

        type_defs.push(quote! {
            impl<#response_lifetime_template, #maybe_request_lifetime>
                    ::HasCdpResponse<#response_lifetime_template>
                    for #request_pascal_case#request_lifetime_generics {
                type Response = #response_pascal_case#response_lifetime_generics;
            }
        });

        let has_request_trait = Ident::from(format!("HasCdp{}", kind));
        let has_request_assoc_type = Ident::from(kind.to_string());
        type_defs.push(quote! {
            impl<#request_lifetime_template, #maybe_response_lifetime>
                    ::#has_request_trait<#request_lifetime_template>
                    for #response_pascal_case#response_lifetime_generics {
                type #has_request_assoc_type = #request_pascal_case#request_lifetime_generics;
            }
        });
    }
}

fn generate_method_struct(
    domain_snake_case: &Ident,
    struct_pascal_case: &Ident,
    struct_meta_attrs: &Tokens,
    deprecation_status: &DeprecationStatus,
    experimental: bool,
    kind: MethodKind,
    method_qualified: &str,
    fields: &[Field],
    uses_lifetime_set: &HashSet<Ident>,
    type_defs: &mut Vec<Tokens>,
) -> bool {
    let (struct_def, struct_lifetime_generics) = if fields.is_empty() {
        let struct_def = quote! {
            #[derive(Clone, Debug, PartialEq)]
            #struct_meta_attrs
            pub struct #struct_pascal_case;

            impl ::serde::Serialize for #struct_pascal_case {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: ::serde::Serializer,
                {
                    ::serde::Serialize::serialize(&::Empty, serializer)
                }
            }

            impl<'de> ::serde::Deserialize<'de> for #struct_pascal_case {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: ::serde::Deserializer<'de>,
                {
                    <::Empty as ::serde::Deserialize<'de>>::deserialize(deserializer)
                        .map(|_| #struct_pascal_case)
                }
            }
        };
        (struct_def, None)
    } else {
        let mut fields_use_lifetime = false;
        let struct_fields: Vec<Tokens> = fields
            .iter()
            .map(|field| {
                generate_field(
                    domain_snake_case,
                    struct_pascal_case,
                    deprecation_status,
                    experimental,
                    field,
                    uses_lifetime_set,
                    &mut fields_use_lifetime,
                    type_defs,
                )
            })
            .collect();

        let struct_lifetime_generics = generate_lifetime_generics(fields_use_lifetime);
        let struct_def = quote! {
            #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
            #struct_meta_attrs
            pub struct #struct_pascal_case#struct_lifetime_generics {
                #(#struct_fields, )*
            }
        };
        (struct_def, struct_lifetime_generics)
    };

    type_defs.push(struct_def);

    let kind_trait = Ident::from(format!("Cdp{}", kind));
    let name_const = Ident::from(format!("{}_NAME", kind.to_string().to_uppercase()));
    type_defs.push(quote! {
        impl#struct_lifetime_generics ::#kind_trait
                for #struct_pascal_case#struct_lifetime_generics {
            const #name_const: &'static str = #method_qualified;
        }
    });

    struct_lifetime_generics.is_some()
}

fn generate_field(
    domain_snake_case: &Ident,
    parent_pascal_case: &Ident,
    parent_deprecation_status: &DeprecationStatus,
    parent_experimental: bool,
    field: &Field,
    uses_lifetime_set: &HashSet<Ident>,
    fields_use_lifetime: &mut bool,
    type_defs: &mut Vec<Tokens>,
) -> Tokens {
    let field_name = &field.name;
    let field_snake_case = snake_case_ident(field_name);

    let deprecation_status = DeprecationStatus::new(field.deprecated, &field.description);

    let meta_attrs =
        generate_meta_attrs(&deprecation_status, field.experimental, &field.description, None);

    let (ty, uses_lifetime) = generate_type_expr(
        domain_snake_case,
        parent_pascal_case,
        Some(field_name),
        parent_deprecation_status,
        parent_experimental,
        &field.ty,
        uses_lifetime_set,
        type_defs,
    );
    if uses_lifetime {
        *fields_use_lifetime = true;
    }

    let (optional_attr, wrapped_ty) = if field.optional {
        (Some(quote! { skip_serializing_if = "Option::is_none" }), quote! { Option<#ty> })
    } else {
        (None, ty)
    };

    quote! {
        #[serde(rename = #field_name, #optional_attr)]
        #meta_attrs
        pub #field_snake_case: #wrapped_ty
    }
}

fn generate_meta_attrs(
    deprecation_status: &DeprecationStatus,
    experimental: bool,
    description: &Option<String>,
    note: Option<String>,
) -> Tokens {
    let mut doc_str = String::new();

    if experimental {
        doc_str.push_str(r#"<span class="stab unstable">[Experimental]</span>"#);
    }

    match *description {
        Some(ref desc) if !deprecation_status.has_own_warning() => {
            if !doc_str.is_empty() {
                doc_str.push_str(" ");
            }
            doc_str.push_str(&escape_for_markdown(desc));
        }
        _ => (),
    }

    if let Some(note) = note {
        if !doc_str.is_empty() {
            doc_str.push_str("\n\n");
        }
        doc_str.push_str(&note);
    }

    let doc_attr = if doc_str.is_empty() {
        None
    } else {
        Some(quote! { #[doc = #doc_str] })
    };

    let deprecated_attr = if deprecation_status.is_deprecated() {
        match deprecation_status.warning() {
            None => Some(quote! { #[deprecated] }),
            Some(warning) => Some(quote! { #[deprecated(note = #warning)] }),
        }
    } else {
        None
    };

    quote! {
        #doc_attr
        #deprecated_attr
    }
}

fn generate_index_entry(
    name: &str,
    link: &str,
    deprecation_status: &DeprecationStatus,
    experimental: bool,
    description: &Option<String>,
) -> String {
    let mut badges_str = String::new();
    if experimental {
        badges_str.push_str(
            " <span class=\"stab unstable\" style=\"display: inline-block\">Experimental</span>",
        );
    }
    if deprecation_status.is_deprecated() {
        match deprecation_status.warning() {
            None => {
                badges_str.push_str(
                    " <span class=\"stab deprecated\" \
                            style=\"display: inline-block\">[Deprecated]</span>",
                );
            }
            Some(warning) => {
                badges_str.push_str(
                    &format!("\n  \n  <span class=\"stab deprecated\">{}</span>", warning),
                );
            }
        }
    }

    let mut desc_str = String::new();
    match *description {
        Some(ref desc) if !deprecation_status.has_own_warning() => {
            desc_str.push_str("\n");
            for line in desc.split('\n') {
                desc_str.push_str(&format!("\n  {}", escape_for_markdown(line)));
            }
        }
        _ => (),
    }

    format!("- [`{}`]({}){}{}\n", name, link, badges_str, desc_str)
}

fn snake_case_ident<T>(src: T) -> Ident
where
    T: AsRef<str>,
{
    Ident::from(snake_case(src))
}

fn pascal_case_ident<T>(src: T) -> Ident
where
    T: AsRef<str>,
{
    Ident::from(pascal_case(src))
}

fn snake_case<T>(src: T) -> String
where
    T: AsRef<str>,
{
    let snake_case = replace_unsafe_chars(src.as_ref()).to_snake_case();
    match snake_case.as_str() {
        "type" => "ty".into(),
        "override" => "overridden".into(),
        _ => snake_case,
    }
}

fn pascal_case<T>(src: T) -> String
where
    T: AsRef<str>,
{
    replace_unsafe_chars(src.as_ref())
        .to_snake_case()
        .to_pascal_case()
}

fn replace_unsafe_chars(src: &str) -> String {
    lazy_static! {
        #[cfg_attr(feature = "clippy", allow(trivial_regex))]
        static ref LEADING_DASH_RE: Regex = Regex::new(r"^-")
            .expect("cdp: LEADING_DASH_RE compilation failed");
    }

    LEADING_DASH_RE.replace(src, "Negative").into_owned()
}

fn resolve_reference(
    domain_snake_case: &Ident,
    target: &str,
    target_pascal_case: &Ident,
) -> Ident {
    lazy_static! {
        static ref INTER_DOMAIN_RE: Regex = Regex::new(r"^([[:alnum:]]+)\.([[:alnum:]]+)$")
            .expect("cdp: INTER_DOMAIN_RE compilation failed");
    }

    match INTER_DOMAIN_RE.captures(target) {
        None => fully_qualified_ident(domain_snake_case, target_pascal_case),
        Some(captures) => {
            let domain_snake_case = snake_case_ident(&captures[1]);
            let item_pascal_case = pascal_case_ident(&captures[2]);
            fully_qualified_ident(&domain_snake_case, &item_pascal_case)
        }
    }
}

fn fully_qualified_ident(domain_snake_case: &Ident, item_ident: &Ident) -> Ident {
    Ident::from(format!("::{}::{}", domain_snake_case, item_ident))
}

fn generate_lifetime_generics(uses_lifetime: bool) -> Option<Tokens> {
    if uses_lifetime {
        Some(quote! { <'a> })
    } else {
        None
    }
}

fn combine_parent_field_idents(parent_pascal_case: &Ident, field_name: Option<&String>) -> Ident {
    match field_name {
        None => parent_pascal_case.clone(),
        Some(field_name) => {
            Ident::from(format!("{}{}", parent_pascal_case, pascal_case_ident(field_name)))
        }
    }
}

fn escape_for_markdown<T>(src: T) -> String
where
    T: AsRef<str>,
{
    lazy_static! {
        static ref MARKDOWN_HAZARD_RE: Regex = Regex::new(r"[*\[\]()]")
            .expect("cdp: MARKDOWN_HAZARD_RE compilation failed");
    }

    MARKDOWN_HAZARD_RE
        .replace_all(src.as_ref(), "\\$0")
        .into_owned()
}

#[derive(Clone)]
enum DeprecationStatus {
    NotDeprecated,
    Deprecated,
    DeprecatedWithWarning(String),
    DeprecatedWithWarningFromParent(String),
}

impl DeprecationStatus {
    fn new(deprecated: bool, description: &Option<String>) -> Self {
        if !deprecated {
            return DeprecationStatus::NotDeprecated;
        }

        lazy_static! {
            static ref DEPRECATION_WARNING_RE: Regex = Regex::new(r"(?i)deprecat")
                .expect("cdp: DEPRECATION_MESSAGE_RE compilation failed");
            static ref DEPRECATION_PREFIX_RE: Regex = Regex::new(r"^Deprecated, ")
                .expect("cdp: DEPRECATION_MESSAGE_RE compilation failed");
        }

        let warning = description.as_ref().and_then(|desc| {
            if desc == "Deprecated." || !DEPRECATION_WARNING_RE.is_match(desc) {
                None
            } else {
                Some(escape_for_markdown(DEPRECATION_PREFIX_RE.replace(desc, "")))
            }
        });

        match warning {
            None => DeprecationStatus::Deprecated,
            Some(warning) => DeprecationStatus::DeprecatedWithWarning(warning),
        }
    }

    fn is_deprecated(&self) -> bool {
        match *self {
            DeprecationStatus::NotDeprecated => false,
            _ => true,
        }
    }

    fn has_own_warning(&self) -> bool {
        match *self {
            DeprecationStatus::DeprecatedWithWarning(_) => true,
            _ => false,
        }
    }

    fn warning(&self) -> Option<&str> {
        match *self {
            DeprecationStatus::DeprecatedWithWarning(ref warning) |
            DeprecationStatus::DeprecatedWithWarningFromParent(ref warning) => {
                Some(warning.as_str())
            }
            _ => None,
        }
    }

    fn add_parent(self, parent: &DeprecationStatus) -> Self {
        if !parent.is_deprecated() || self.has_own_warning() {
            return self;
        }

        match parent.warning() {
            None => DeprecationStatus::Deprecated,
            Some(warning) => {
                DeprecationStatus::DeprecatedWithWarningFromParent(warning.to_string())
            }
        }
    }
}

fn generate_method_note(
    domain_snake_case: &Ident,
    method_qualified: &str,
    request_pascal_case: &Ident,
    maybe_response_pascal_case: &Option<Ident>,
    kind: MethodKind,
) -> String {
    let response_line = match *maybe_response_pascal_case {
        None => String::new(),
        Some(ref response_pascal_case) => format!(
            "  \n*Response Struct:* \
             [`cdp::{domain_snake_case}::{response_pascal_case}`]\
             (struct.{response_pascal_case}.html)",
            domain_snake_case = domain_snake_case,
            response_pascal_case = response_pascal_case,
        ),
    };

    format!(
        "# {kind} `{method_qualified}`\n\n\
         *Domain Module:* [`cdp::{domain_snake_case}`](index.html)  \n\
         *{kind} Struct:* \
         [`cdp::{domain_snake_case}::{request_pascal_case}`]\
         (struct.{request_pascal_case}.html){response_line}",
        domain_snake_case = domain_snake_case,
        method_qualified = method_qualified,
        kind = kind,
        request_pascal_case = request_pascal_case,
        response_line = response_line
    )
}
