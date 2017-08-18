// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

#![recursion_limit = "128"]

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

    let ws_src = generate_ws(browser_protocol.domains.iter().chain(js_protocol.domains.iter()));
    let ws_path = Path::new(&out_dir).join("ws_generated.rs");
    let mut ws_file = File::create(ws_path).expect("Error creating ws_generated.rs");
    ws_file.write_all(ws_src.as_bytes()).expect("Error writing ws_generated.rs");

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

struct CommandRefs<'a> {
    parse_command_arms: &'a mut Vec<Tokens>,
    response_variants: &'a mut Vec<Tokens>,
    response_name_arms: &'a mut Vec<Tokens>,
}

fn generate_ws<'a, T>(domains: T) -> String
where
    T: Iterator<Item = &'a Domain>,
{
    let mut command_variants = vec![];
    let mut command_name_arms = vec![];
    let mut parse_command_arms = vec![];

    let mut event_variants = vec![];
    let mut event_name_arms = vec![];

    let mut response_variants = vec![];
    let mut response_name_arms = vec![];

    let mut modules = vec![];

    {
        let mut command_refs = CommandRefs {
            parse_command_arms: &mut parse_command_arms,
            response_variants: &mut response_variants,
            response_name_arms: &mut response_name_arms,
        };
        for domain in domains {
            generate_domain(
                domain,
                &mut command_variants,
                &mut command_name_arms,
                &mut command_refs,
                &mut event_variants,
                &mut event_name_arms,
                &mut modules,
            );
        }
    }

    (quote! {
        #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
        #[serde(tag = "method", content = "params")]
        pub enum Command {
            #(#command_variants, )*
        }

        impl Command {
            pub fn name(&self) -> &'static str {
                match *self {
                    #(#command_name_arms, )*
                }
            }

            pub fn parse_command<'de, D, S>(command: S, deserializer: D)
                                            -> Option<Result<Self, D::Error>>
                where D: ::serde::Deserializer<'de>,
                      S: AsRef<str>
            {
                match command.as_ref() {
                    #(#parse_command_arms, )*
                    _ => None,
                }
            }
        }

        #[derive(Serialize, Clone, Debug, PartialEq)]
        #[serde(untagged)]
        pub enum Response {
            #(#response_variants, )*
        }

        impl Response {
            pub fn name(&self) -> &'static str {
                match *self {
                    #(#response_name_arms, )*
                }
            }
        }

        #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
        #[serde(tag = "method", content = "params")]
        pub enum Event {
            #(#event_variants, )*
        }

        impl Event {
            pub fn name(&self) -> &'static str {
                match *self {
                    #(#event_name_arms, )*
                }
            }
        }

        #(#modules)*
    }).to_string()
}

fn generate_domain<'a>(
    domain: &Domain,
    command_variants: &mut Vec<Tokens>,
    command_name_arms: &mut Vec<Tokens>,
    command_refs: &mut CommandRefs<'a>,
    event_variants: &mut Vec<Tokens>,
    event_name_arms: &mut Vec<Tokens>,
    modules: &mut Vec<Tokens>,
) {
    let domain_snake_case = snake_case_ident(&domain.name);
    let domain_pascal_case = pascal_case_ident(&domain.name);

    let deprecation_status = DeprecationStatus::new(domain.deprecated, &domain.description);

    let mut type_defs = vec![];
    let mut domain_index = format!("# {}\n\n", domain.name);

    if !domain.commands.is_empty() {
        domain_index.push_str("## Commands\n\n");
        for command in domain.commands.iter() {
            generate_method(
                &domain,
                &domain_snake_case,
                &domain_pascal_case,
                command,
                command_variants,
                command_name_arms,
                Some(command_refs),
                &mut domain_index,
                &mut type_defs,
            );
        }
    }

    if !domain.events.is_empty() {
        domain_index.push_str("\n## Events\n\n");
        for event in domain.events.iter() {
            generate_method(
                &domain,
                &domain_snake_case,
                &domain_pascal_case,
                event,
                event_variants,
                event_name_arms,
                None,
                &mut domain_index,
                &mut type_defs,
            );
        }
    }

    if !domain.type_defs.is_empty() {
        domain_index.push_str("\n##Types\n\n");
        for type_def in domain.type_defs.iter() {
            generate_type_def(
                &domain,
                &domain_snake_case,
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

    modules.push(quote! {
        #meta_attrs
        pub mod #domain_snake_case {
            #(#type_defs)*
        }
    });
}

fn generate_type_def(
    domain: &Domain,
    domain_snake_case: &Ident,
    type_def: &TypeDef,
    domain_index: &mut String,
    type_defs: &mut Vec<Tokens>,
) {
    let type_def_pascal_case = pascal_case_ident(&type_def.name);

    let deprecation_status = DeprecationStatus::new(type_def.deprecated, &type_def.description);
    let experimental = domain.experimental || type_def.experimental;

    let maybe_expr = generate_type_expr_impl(
        domain_snake_case,
        None,
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

    domain_index.push_str(
        &format!("- [`{}`]({}.{}.html)\n", type_def.name, category, type_def_pascal_case),
    );

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
    parent_variant_pascal_case: Option<&Ident>,
    parent_pascal_case: &Ident,
    field_name: Option<&String>,
    deprecation_status: &DeprecationStatus,
    experimental: bool,
    ty: &Type,
    type_defs: &mut Vec<Tokens>,
) -> Tokens {
    let maybe_expr = generate_type_expr_impl(
        domain_snake_case,
        parent_variant_pascal_case,
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
            quote! { ::ws::#domain_snake_case::#type_def_pascal_case }
        }
    }
}

fn generate_type_expr_impl(
    domain_snake_case: &Ident,
    parent_variant_pascal_case: Option<&Ident>,
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
            let note = generate_field_usage_note(
                domain_snake_case,
                parent_variant_pascal_case,
                parent_pascal_case,
                field_name,
            );
            let meta_attrs =
                generate_meta_attrs(deprecation_status, experimental, description, note);
            let variants: Vec<Tokens> = values.iter().map(generate_type_enum_variant).collect();

            type_defs.push(quote! {
                #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
                #meta_attrs
                pub enum #type_def_pascal_case {
                    #(#variants, )*
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
                parent_variant_pascal_case,
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
            Some(quote! { ::ws::Empty })
        } else {
            let type_def_pascal_case = combine_parent_field_idents(parent_pascal_case, field_name);
            let note = generate_field_usage_note(
                domain_snake_case,
                parent_variant_pascal_case,
                parent_pascal_case,
                field_name,
            );
            let meta_attrs =
                generate_meta_attrs(deprecation_status, experimental, description, note);
            let fields: Vec<Tokens> = properties
                .iter()
                .map(|field| {
                    generate_field(
                        domain_snake_case,
                        parent_variant_pascal_case,
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
    parent_variant_pascal_case: Option<&Ident>,
    parent_pascal_case: &Ident,
    field_name: Option<&String>,
) -> Option<String> {
    field_name.map(|field_name| {
        let field_snake_case = snake_case_ident(field_name);

        match parent_variant_pascal_case {
            Some(parent_variant_pascal_case) => format!(
                "Used in the type of \
                 [`cdp::ws::Command::{}::{}`](../enum.Command.html#variant.{}).",
                parent_variant_pascal_case,
                field_snake_case,
                parent_variant_pascal_case
            ),
            None => format!(
                "Used in the type of [`cdp::ws::{}::{}::{}`](struct.{}.html#structfield.{}).",
                domain_snake_case,
                parent_pascal_case,
                field_snake_case,
                parent_pascal_case,
                field_snake_case
            ),
        }
    })
}

fn generate_type_enum_variant(variant_name: &String) -> Tokens {
    let variant_pascal_case = pascal_case_ident(variant_name);
    let doc_text = format!(r#"Represented as `"{}"`."#, variant_name);

    quote! {
        #[serde(rename = #variant_name)]
        #[doc = #doc_text]
        #variant_pascal_case
    }
}

fn generate_method<'a>(
    domain: &Domain,
    domain_snake_case: &Ident,
    domain_pascal_case: &Ident,
    method: &Method,
    request_variants: &mut Vec<Tokens>,
    request_name_arms: &mut Vec<Tokens>,
    mut command_refs: Option<&mut CommandRefs<'a>>,
    domain_index: &mut String,
    type_defs: &mut Vec<Tokens>,
) {
    let category = if command_refs.is_some() {
        "Command"
    } else {
        "Event"
    };
    let enum_pascal_case = Ident::from(category);

    let method_qualified = format!("{}.{}", domain.name, method.name);
    let method_pascal_case = pascal_case_ident(&method.name);

    let variant_pascal_case = Ident::from(format!("{}{}", domain_pascal_case, method_pascal_case));
    let parameters_struct_pascal_case = if command_refs.is_some() && !method.parameters.is_empty()
    {
        Some(Ident::from(format!("{}Params", method_pascal_case)))
    } else {
        None
    };
    let response_struct_pascal_case = if command_refs.is_some() && !method.returns.is_empty() {
        Some(Ident::from(format!("{}Response", method_pascal_case)))
    } else {
        None
    };

    domain_index.push_str(&format!(
        "- [`{}`](../enum.{}.html#variant.{})\n",
        method_qualified,
        category,
        variant_pascal_case
    ));

    let deprecation_status = DeprecationStatus::new(method.deprecated, &method.description);
    let experimental = domain.experimental || method.experimental;

    let variant_note = generate_method_note(
        &domain_snake_case,
        &method_qualified,
        true,
        &variant_pascal_case,
        &parameters_struct_pascal_case,
        command_refs.is_some(),
        &response_struct_pascal_case,
        category,
    );
    let struct_note = generate_method_note(
        &domain_snake_case,
        &method_qualified,
        false,
        &variant_pascal_case,
        &parameters_struct_pascal_case,
        command_refs.is_some(),
        &response_struct_pascal_case,
        category,
    );

    let variant_meta_attrs = generate_meta_attrs(
        &deprecation_status,
        experimental,
        &method.description,
        Some(variant_note),
    );
    let struct_meta_attrs = generate_meta_attrs(
        &deprecation_status,
        experimental,
        &method.description,
        Some(struct_note),
    );

    let request_variant_content = if command_refs.is_some() {
        match parameters_struct_pascal_case {
            None => None,
            Some(ref parameters_struct_pascal_case) => {
                Some(quote! { (::ws::#domain_snake_case::#parameters_struct_pascal_case) })
            }
        }
    } else {
        let event_pascal_case = Ident::from(format!("{}Event", method_pascal_case));
        let fields: Vec<Tokens> = method
            .parameters
            .iter()
            .map(|field| {
                generate_field(
                    domain_snake_case,
                    Some(&variant_pascal_case),
                    &event_pascal_case,
                    &deprecation_status,
                    experimental,
                    field,
                    type_defs,
                )
            })
            .collect();
        if fields.is_empty() {
            None
        } else {
            Some(quote! { { #(#fields, )* } })
        }
    };
    let request_unit_variant_attrs = match request_variant_content {
        Some(_) => None,
        None => Some(unit_variant_attrs()),
    };
    request_variants.push(quote! {
        #[serde(rename = #method_qualified)]
        #request_unit_variant_attrs
        #variant_meta_attrs
        #variant_pascal_case #request_variant_content
    });
    request_name_arms.push(quote! {
        ::ws::#enum_pascal_case::#variant_pascal_case {..} => #method_qualified
    });

    if let Some(ref mut command_refs) = command_refs {
        let command_arm_body = match parameters_struct_pascal_case {
            None => quote! {
                ::ws::Empty::deserialize(deserializer).map(|_| {
                    ::ws::#enum_pascal_case::#variant_pascal_case
                })
            },
            Some(ref params_struct_pascal_case) => quote! {
                ::ws::#domain_snake_case::#params_struct_pascal_case::deserialize(deserializer)
                    .map(::ws::#enum_pascal_case::#variant_pascal_case)
            },
        };
        command_refs.parse_command_arms.push(quote! {
            #method_qualified => Some(#command_arm_body)
        });

        let response_unit_variant_attrs = match response_struct_pascal_case {
            None => Some(unit_variant_attrs()),
            Some(_) => None,
        };
        let response_variant_content = match response_struct_pascal_case {
            None => None,
            Some(ref response_struct_pascal_case) => {
                Some(quote!{(::ws::#domain_snake_case::#response_struct_pascal_case)})
            }
        };
        command_refs.response_variants.push(quote! {
            #[serde(rename = #method_qualified)]
            #response_unit_variant_attrs
            #variant_meta_attrs
            #variant_pascal_case #response_variant_content
        });

        command_refs.response_name_arms.push(quote! {
            ::ws::Response::#variant_pascal_case {..} => #method_qualified
        });
    }

    if let Some(ref parameters_struct_pascal_case) = parameters_struct_pascal_case {
        let fields: Vec<Tokens> = method
            .parameters
            .iter()
            .map(|field| {
                generate_field(
                    domain_snake_case,
                    None,
                    &parameters_struct_pascal_case,
                    &deprecation_status,
                    experimental,
                    field,
                    type_defs,
                )
            })
            .collect();
        type_defs.push(quote! {
            #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
            #struct_meta_attrs
            pub struct #parameters_struct_pascal_case {
                #(#fields, )*
            }
        });
    }

    if let Some(ref response_struct_pascal_case) = response_struct_pascal_case {
        let fields: Vec<Tokens> = method
            .returns
            .iter()
            .map(|field| {
                generate_field(
                    domain_snake_case,
                    None,
                    &response_struct_pascal_case,
                    &deprecation_status,
                    experimental,
                    field,
                    type_defs,
                )
            })
            .collect();
        type_defs.push(quote! {
            #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
            #struct_meta_attrs
            pub struct #response_struct_pascal_case {
                #(#fields, )*
            }
        });
    }
}

fn generate_field(
    domain_snake_case: &Ident,
    parent_variant_pascal_case: Option<&Ident>,
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
        parent_variant_pascal_case,
        parent_pascal_case,
        Some(&field_name),
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

    let visibility = if parent_variant_pascal_case.is_none() {
        Some(quote! { pub })
    } else {
        None
    };

    quote! {
        #[serde(rename = #field_name, #optional_attr)]
        #meta_attrs
        #visibility #field_snake_case: #wrapped_ty
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

    match description {
        &Some(ref desc) if !deprecation_status.has_warning() => {
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

    let deprecated_attr = match deprecation_status {
        &DeprecationStatus::NotDeprecated => None,
        &DeprecationStatus::Deprecated(None) => Some(quote! { #[deprecated] }),
        &DeprecationStatus::Deprecated(Some(ref warning)) => {
            Some(quote! { #[deprecated(note = #warning)] })
        }
    };

    quote! {
        #doc_attr
        #deprecated_attr
    }
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

fn replace_unsafe_chars<T>(src: T) -> String
where
    T: AsRef<str>,
{
    lazy_static! {
        static ref LEADING_DASH_RE: Regex = Regex::new(r"^-")
            .expect("cdp: LEADING_DASH_RE compilation failed");
    }

    LEADING_DASH_RE.replace(src.as_ref(), "Negative").into_owned()
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
        None => quote! { ::ws::#domain_snake_case::#target_pascal_case },
        Some(captures) => {
            let domain_snake_case = snake_case_ident(&captures[1]);
            let item_pascal_case = pascal_case_ident(&captures[2]);
            quote! { ::ws::#domain_snake_case::#item_pascal_case }
        }
    }
}

fn unit_variant_attrs() -> Tokens {
    quote! {
        #[serde(serialize_with = "serialize_unit_variant_as_empty_struct")]
        #[serde(deserialize_with = "deserialize_empty_struct_as_unit_variant")]
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

enum DeprecationStatus {
    NotDeprecated,
    Deprecated(Option<String>),
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
            |desc| if desc == "Deprecated." || !DEPRECATION_WARNING_RE.is_match(desc.as_str()) {
                None
            } else {
                Some(escape_for_markdown(DEPRECATION_PREFIX_RE.replace(desc.as_str(), "")))
            },
        );

        DeprecationStatus::Deprecated(warning)
    }

    fn has_warning(&self) -> bool {
        match self {
            &DeprecationStatus::NotDeprecated => false,
            &DeprecationStatus::Deprecated(ref warning) => warning.is_some(),
        }
    }
}

fn generate_method_note(
    domain_snake_case: &Ident,
    method_qualified: &String,
    is_variant_note: bool,
    variant_pascal_case: &Ident,
    parameters_struct_pascal_case: &Option<Ident>,
    has_response_variant: bool,
    response_struct_pascal_case: &Option<Ident>,
    category: &'static str,
) -> String {
    let domain_link_prefix = if is_variant_note {
        format!("{domain_snake_case}/", domain_snake_case = domain_snake_case)
    } else {
        "".into()
    };
    let variant_link_prefix = if is_variant_note { "" } else { "../" };

    let parameters_struct_line = match parameters_struct_pascal_case {
        &None => String::new(),
        &Some(ref parameters_struct_pascal_case) => format!(
            "  \n*Parameters Struct:* \
             [`cdp::ws::{domain_snake_case}::\
             {parameters_struct_pascal_case}`]({domain_link_prefix}struct.\
             {parameters_struct_pascal_case}.html)",
            domain_snake_case = domain_snake_case,
            parameters_struct_pascal_case = parameters_struct_pascal_case,
            domain_link_prefix = domain_link_prefix
        ),
    };

    let response_variant_line = if has_response_variant {
        format!(
            "  \n*Response Variant:* \
             [`cdp::ws::Response::{variant_pascal_case}`]({variant_link_prefix}enum.Response.\
             html#variant.{variant_pascal_case})",
            variant_pascal_case = variant_pascal_case,
            variant_link_prefix = variant_link_prefix
        )
    } else {
        String::new()
    };

    let response_struct_line = match response_struct_pascal_case {
        &None => String::new(),
        &Some(ref response_struct_pascal_case) => format!(
            "  \n*Response Struct:* \
             [`cdp::ws::{domain_snake_case}::\
             {response_struct_pascal_case}`]({domain_link_prefix}struct.\
             {response_struct_pascal_case}.html)",
            domain_snake_case = domain_snake_case,
            response_struct_pascal_case = response_struct_pascal_case,
            domain_link_prefix = domain_link_prefix
        ),
    };

    format!(
        "# {category} `{method_qualified}`\n\n\
         *Domain Module:* [`cdp::ws::{domain_snake_case}`]({domain_link_prefix}index.html)  \n\
         *{category} Variant:* [`cdp::ws::{category}::{variant_pascal_case}`]\
         ({variant_link_prefix}enum.{category}.html#variant.{variant_pascal_case})\
         {parameters_struct_line}{response_variant_line}{response_struct_line}",
        domain_snake_case = domain_snake_case,
        method_qualified = method_qualified,
        variant_pascal_case = variant_pascal_case,
        category = category,
        domain_link_prefix = domain_link_prefix,
        variant_link_prefix = variant_link_prefix,
        parameters_struct_line = parameters_struct_line,
        response_variant_line = response_variant_line,
        response_struct_line = response_struct_line
    )
}
