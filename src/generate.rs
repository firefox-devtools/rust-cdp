// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

#![recursion_limit = "128"]
#![cfg_attr(feature = "strict", deny(warnings))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
// Too may false positives.
#![cfg_attr(feature = "clippy", allow(trivial_regex))]

extern crate inflector;
extern crate regex;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate quote;

#[macro_use]
extern crate serde_derive;

mod definition;

use inflector::Inflector;
use quote::{Ident, Tokens};
use regex::Regex;
use std::env;
use std::fmt;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use definition::{Definition, Domain, Field, Method, Type, TypeDef, Version};

fn main() {
    let out_dir = env::var("OUT_DIR").expect("Error retrieving OUT_DIR environment variable");

    let browser_protocol =
        read_protocol_file("json/browser_protocol.json", "error reading browser_protocol.json");
    let js_protocol =
        read_protocol_file("json/js_protocol.json", "error reading js_protocol.json");

    if browser_protocol.version != js_protocol.version {
        panic!("json/browser_protocol.json and json/js_protocol.json versions don't match");
    }

    let constants_src = generate_constants(&browser_protocol.version);
    let constants_path = Path::new(&out_dir).join("constants.rs");
    let mut constants_file = File::create(constants_path).expect("Error creating constants.rs");
    constants_file
        .write_all(constants_src.as_bytes())
        .expect("Error writing generated constants.rs");

    let tools_src =
        generate_tools(browser_protocol.domains.iter().chain(js_protocol.domains.iter()));
    let tools_path = Path::new(&out_dir).join("tools_generated.rs");
    let mut tools_file = File::create(tools_path).expect("Error creating tools_generated.rs");
    tools_file.write_all(tools_src.as_bytes()).expect("Error writing tools_generated.rs");

    println!("cargo:rerun-if-changed=json/browser_protocol.json");
    println!("cargo:rerun-if-changed=json/js_protocol.json");
}

fn read_protocol_file(file: &str, msg: &str) -> Definition {
    let mut file = File::open(file).expect(msg);
    serde_json::from_reader(&mut file).expect(msg)
}

fn generate_constants(version: &Version) -> String {
    let version_string = version.to_string();

    (quote! {
        pub const STABLE_PROTOCOL_VERSION: &'static str = #version_string;
    }).to_string()
}

fn generate_tools<'a, T>(domains: T) -> String
where
    T: Iterator<Item = &'a Domain>,
{
    let modules = domains.map(generate_domain);
    quote!(#(#modules)*).to_string()
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

fn generate_domain(domain: &Domain) -> Tokens {
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
    domain_index: &mut String,
    type_defs: &mut Vec<Tokens>,
) {
    let type_def_pascal_case = pascal_case_ident(&type_def.name);

    let deprecation_status = DeprecationStatus::new(type_def.deprecated, &type_def.description)
        .add_parent(domain_deprecation_status);
    let experimental = domain.experimental || type_def.experimental;

    let maybe_expr = generate_type_expr_impl(
        domain_snake_case,
        &type_def_pascal_case,
        None,
        &deprecation_status,
        experimental,
        &type_def.description,
        &type_def.ty,
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
        type_defs.push(quote! {
            #meta_attrs
            pub type #type_def_pascal_case = #expr;
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
    type_defs: &mut Vec<Tokens>,
) -> Tokens {
    let maybe_expr = generate_type_expr_impl(
        domain_snake_case,
        parent_pascal_case,
        field_name,
        deprecation_status,
        experimental,
        &None,
        ty,
        type_defs,
    );

    match maybe_expr {
        Some(expr) => expr,
        None => {
            let type_def_pascal_case = combine_parent_field_idents(parent_pascal_case, field_name);
            quote! { ::tools::#domain_snake_case::#type_def_pascal_case }
        }
    }
}

#[cfg_attr(feature = "clippy", allow(too_many_arguments))]
fn generate_type_expr_impl(
    domain_snake_case: &Ident,
    parent_pascal_case: &Ident,
    field_name: Option<&String>,
    deprecation_status: &DeprecationStatus,
    experimental: bool,
    description: &Option<String>,
    ty: &Type,
    type_defs: &mut Vec<Tokens>,
) -> Option<Tokens> {
    match *ty {
        Type::Reference(ref target) => {
            Some(resolve_reference(domain_snake_case, parent_pascal_case, target))
        }
        Type::Boolean => Some(quote! { bool }),
        Type::Integer => Some(quote! { i32 }),
        Type::Number => Some(quote! { f64 }),
        Type::String => Some(quote! { String }),
        Type::Enum(ref values) => {
            let type_def_pascal_case = combine_parent_field_idents(parent_pascal_case, field_name);
            let note =
                generate_field_usage_note(domain_snake_case, parent_pascal_case, field_name);
            let meta_attrs =
                generate_meta_attrs(deprecation_status, experimental, description, note);
            let variants: Vec<Tokens> =
                values.iter().map(|s| generate_type_enum_variant(s)).collect();

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
                    type Err = ::tools::ParseEnumError;

                    fn from_str(s: &str) -> Result<Self, Self::Err> {
                        match s {
                            #(#parse_arms, )*
                            _ => Err(::tools::ParseEnumError {
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

            None
        }
        Type::Array {
            ref item,
            min_items,
            max_items,
        } => {
            let item_expr = generate_type_expr(
                domain_snake_case,
                parent_pascal_case,
                field_name,
                deprecation_status,
                experimental,
                &item.ty,
                type_defs,
            );

            Some(match (min_items, max_items) {
                (Some(min), Some(max)) if min == max => {
                    let n = max as usize;
                    quote! { [#item_expr; #n] }
                }
                _ => quote! { Vec<#item_expr> },
            })
        }
        Type::Object(ref properties) => if properties.is_empty() {
            Some(quote! { ::tools::Empty })
        } else {
            let type_def_pascal_case = combine_parent_field_idents(parent_pascal_case, field_name);
            let note =
                generate_field_usage_note(domain_snake_case, parent_pascal_case, field_name);
            let meta_attrs =
                generate_meta_attrs(deprecation_status, experimental, description, note);
            let fields: Vec<Tokens> = properties
                .iter()
                .map(|field| {
                    generate_field(
                        domain_snake_case,
                        &type_def_pascal_case,
                        deprecation_status,
                        experimental,
                        field,
                        type_defs,
                    )
                })
                .collect();

            type_defs.push(quote! {
                #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
                #meta_attrs
                pub struct #type_def_pascal_case {
                    #(#fields, )*
                }
            });

            None
        },
        Type::Any => Some(quote! { ::serde_json::Value }),
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
            "Used in the type of [`cdp::tools::{}::{}::{}`](struct.{}.html#structfield.{}).",
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

    generate_method_struct(
        domain_snake_case,
        &request_pascal_case,
        &meta_attrs,
        &deprecation_status,
        experimental,
        kind,
        &method_qualified,
        method.parameters.as_slice(),
        type_defs,
    );

    let request_serialize_trait = Ident::from(format!("SerializeTools{}", kind));
    let request_name_method = Ident::from(format!("{}_name", kind).to_lowercase());
    let request_serialize_params_method =
        Ident::from(format!("serialize_{}_params", kind).to_lowercase());
    type_defs.push(quote! {
        impl ::tools::#request_serialize_trait for #request_pascal_case {
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

    let request_deserialize_trait = Ident::from(format!("DeserializeTools{}", kind));
    let request_deserialize_method = Ident::from(format!("deserialize_{}", kind).to_lowercase());
    type_defs.push(quote! {
        impl<'de> ::tools::#request_deserialize_trait<'de> for #request_pascal_case {
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
        generate_method_struct(
            domain_snake_case,
            response_pascal_case,
            &meta_attrs,
            &deprecation_status,
            experimental,
            kind,
            &method_qualified,
            method.returns.as_slice(),
            type_defs,
        );

        type_defs.push(quote! {
            impl ::tools::HasToolsResponse for #request_pascal_case {
                type Response = #response_pascal_case;
            }
        });

        let has_request_trait = Ident::from(format!("HasTools{}", kind));
        let has_request_assoc_type = Ident::from(kind.to_string());
        type_defs.push(quote! {
            impl ::tools::#has_request_trait for #response_pascal_case {
                type #has_request_assoc_type = #request_pascal_case;
            }
        });
    }
}

#[cfg_attr(feature = "clippy", allow(too_many_arguments))]
fn generate_method_struct(
    domain_snake_case: &Ident,
    struct_pascal_case: &Ident,
    struct_meta_attrs: &Tokens,
    deprecation_status: &DeprecationStatus,
    experimental: bool,
    kind: MethodKind,
    method_qualified: &str,
    fields: &[Field],
    type_defs: &mut Vec<Tokens>,
) {
    let struct_def = if fields.is_empty() {
        quote! {
            #[derive(Clone, Debug, PartialEq)]
            #struct_meta_attrs
            pub struct #struct_pascal_case;

            impl ::serde::Serialize for #struct_pascal_case {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: ::serde::Serializer,
                {
                    ::serde::Serialize::serialize(&::tools::Empty, serializer)
                }
            }

            impl<'de> ::serde::Deserialize<'de> for #struct_pascal_case {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: ::serde::Deserializer<'de>,
                {
                    <::tools::Empty as ::serde::Deserialize<'de>>::deserialize(deserializer)
                        .map(|_| #struct_pascal_case)
                }
            }
        }
    } else {
        let struct_fields: Vec<Tokens> = fields
            .iter()
            .map(|field| {
                generate_field(
                    domain_snake_case,
                    struct_pascal_case,
                    deprecation_status,
                    experimental,
                    field,
                    type_defs,
                )
            })
            .collect();

        quote! {
            #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
            #struct_meta_attrs
            pub struct #struct_pascal_case {
                #(#struct_fields, )*
            }
        }
    };

    type_defs.push(struct_def);

    let name_const = Ident::from(format!("{}_NAME", kind.to_string().to_uppercase()));
    type_defs.push(quote! {
        impl #struct_pascal_case {
            pub const #name_const: &'static str = #method_qualified;
        }
    });
}

fn generate_field(
    domain_snake_case: &Ident,
    parent_pascal_case: &Ident,
    parent_deprecation_status: &DeprecationStatus,
    parent_experimental: bool,
    field: &Field,
    type_defs: &mut Vec<Tokens>,
) -> Tokens {
    let field_name = &field.name;
    let field_snake_case = snake_case_ident(field_name);

    let deprecation_status = DeprecationStatus::new(field.deprecated, &field.description);

    let meta_attrs =
        generate_meta_attrs(&deprecation_status, field.experimental, &field.description, None);

    let ty = generate_type_expr(
        domain_snake_case,
        parent_pascal_case,
        Some(field_name),
        parent_deprecation_status,
        parent_experimental,
        &field.ty,
        type_defs,
    );

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
    replace_unsafe_chars(src.as_ref()).to_snake_case().to_pascal_case()
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
    parent_pascal_case: &Ident,
    target: &str,
) -> Tokens {
    lazy_static! {
        static ref INTER_DOMAIN_RE: Regex = Regex::new(r"^([[:alnum:]]+)\.([[:alnum:]]+)$")
            .expect("cdp: INTER_DOMAIN_RE compilation failed");
    }

    let target_pascal_case = pascal_case_ident(target);
    if target_pascal_case == parent_pascal_case {
        return quote! { Box<#target_pascal_case> };
    }

    match INTER_DOMAIN_RE.captures(target) {
        None => quote! { ::tools::#domain_snake_case::#target_pascal_case },
        Some(captures) => {
            let domain_snake_case = snake_case_ident(&captures[1]);
            let item_pascal_case = pascal_case_ident(&captures[2]);
            quote! { ::tools::#domain_snake_case::#item_pascal_case }
        }
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

    MARKDOWN_HAZARD_RE.replace_all(src.as_ref(), "\\$0").into_owned()
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

        let warning = description.as_ref().and_then(
            |desc| if desc == "Deprecated." || !DEPRECATION_WARNING_RE.is_match(desc) {
                None
            } else {
                Some(escape_for_markdown(DEPRECATION_PREFIX_RE.replace(desc, "")))
            },
        );

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
             [`cdp::tools::{domain_snake_case}::\
             {response_pascal_case}`](struct.{response_pascal_case}.html)",
            domain_snake_case = domain_snake_case,
            response_pascal_case = response_pascal_case,
        ),
    };

    format!(
        "# {kind} `{method_qualified}`\n\n\
         *Domain Module:* \
         [`cdp::tools::{domain_snake_case}`](index.html)  \n\
         *{kind} Struct:* \
         [`cdp::tools::{domain_snake_case}::{request_pascal_case}`]\
         (struct.{request_pascal_case}.html){response_line}",
        domain_snake_case = domain_snake_case,
        method_qualified = method_qualified,
        kind = kind,
        request_pascal_case = request_pascal_case,
        response_line = response_line
    )
}
