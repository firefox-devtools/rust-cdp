// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Definition {
    pub version: Version,
    pub domains: Vec<Domain>,
}

impl Serialize for Definition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let def = DefinitionImpl::from(self);
        def.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Definition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let def = DefinitionImpl::deserialize(deserializer)?;
        def.into_definition()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Version {
    pub major: String,
    pub minor: String,
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Domain {
    pub name: String,
    pub description: Option<String>,
    pub experimental: bool,
    pub deprecated: bool,
    pub dependencies: Vec<String>,
    pub type_defs: Vec<TypeDef>,
    pub commands: Vec<Method>,
    pub events: Vec<Method>,
}

impl Serialize for Domain {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let domain = DomainImpl::from(self);
        domain.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Domain {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let domain = DomainImpl::deserialize(deserializer)?;
        domain.into_domain()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Type {
    Reference(String),
    Boolean,
    Integer,
    Number,
    String,
    Enum(Vec<String>),
    Array {
        item: Box<Item>,
        min_items: Option<u64>,
        max_items: Option<u64>,
    },
    Object(Vec<Field>),
    Any,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeDef {
    pub name: String,
    pub description: Option<String>,
    pub experimental: bool,
    pub deprecated: bool,
    pub ty: Type,
}

impl Serialize for TypeDef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let type_def = TypeDefImpl::from(self);
        type_def.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TypeDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let type_def = TypeDefImpl::deserialize(deserializer)?;
        type_def.into_type_def()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Method {
    pub name: String,
    pub description: Option<String>,
    pub experimental: bool,
    pub deprecated: bool,
    pub handlers: Vec<String>,
    pub parameters: Vec<Field>,
    pub returns: Vec<Field>,
    pub redirect: Option<String>,
}

impl Serialize for Method {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let method = MethodImpl::from(self);
        method.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Method {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let method = MethodImpl::deserialize(deserializer)?;
        method.into_method()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Field {
    pub name: String,
    pub description: Option<String>,
    pub experimental: bool,
    pub deprecated: bool,
    pub optional: bool,
    pub ty: Type,
}

impl Serialize for Field {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let field = FieldImpl::from(self);
        field.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Field {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let field = FieldImpl::deserialize(deserializer)?;
        field.into_field()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Item {
    pub description: Option<String>,
    pub ty: Type,
}

impl Serialize for Item {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let item = ItemImpl::from(self);
        item.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Item {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let item = ItemImpl::deserialize(deserializer)?;
        item.into_item()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
struct DefinitionImpl {
    version: Version,
    domains: Vec<DomainImpl>,
}

impl DefinitionImpl {
    fn into_definition<E>(self) -> Result<Definition, E>
    where
        E: de::Error,
    {
        Ok(Definition {
            version: self.version,
            domains: self.domains
                .into_iter()
                .map(DomainImpl::into_domain)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl<'a> From<&'a Definition> for DefinitionImpl {
    fn from(def: &'a Definition) -> DefinitionImpl {
        DefinitionImpl {
            version: def.version.clone(),
            domains: def.domains.iter().map(From::from).collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
struct DomainImpl {
    #[serde(rename = "domain")]
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    experimental: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    deprecated: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    dependencies: Vec<String>,
    #[serde(default, rename = "types", skip_serializing_if = "Vec::is_empty")]
    type_defs: Vec<TypeDefImpl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    commands: Vec<MethodImpl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    events: Vec<MethodImpl>,
}

impl DomainImpl {
    fn into_domain<E>(self) -> Result<Domain, E>
    where
        E: de::Error,
    {
        Ok(Domain {
            name: self.name,
            description: self.description,
            experimental: self.experimental,
            deprecated: self.deprecated,
            dependencies: self.dependencies,
            type_defs: self.type_defs
                .into_iter()
                .map(TypeDefImpl::into_type_def)
                .collect::<Result<_, _>>()?,
            commands: self.commands
                .into_iter()
                .map(MethodImpl::into_method)
                .collect::<Result<_, _>>()?,
            events:
                self.events.into_iter().map(MethodImpl::into_method).collect::<Result<_, _>>()?,
        })
    }
}

impl<'a> From<&'a Domain> for DomainImpl {
    fn from(def: &'a Domain) -> DomainImpl {
        DomainImpl {
            name: def.name.clone(),
            description: def.description.clone(),
            experimental: def.experimental,
            deprecated: def.deprecated,
            dependencies: def.dependencies.clone(),
            type_defs: def.type_defs.iter().map(From::from).collect(),
            commands: def.commands.iter().map(From::from).collect(),
            events: def.events.iter().map(From::from).collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
struct TypeDefImpl {
    #[serde(rename = "id")]
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    experimental: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    deprecated: bool,
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    reference: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    primitive: Option<Primitive>,
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    enum_values: Option<Vec<String>>,
    #[serde(rename = "items", skip_serializing_if = "Option::is_none")]
    item: Option<ItemImpl>,
    #[serde(rename = "minItems", skip_serializing_if = "Option::is_none")]
    min_items: Option<u64>,
    #[serde(rename = "maxItems", skip_serializing_if = "Option::is_none")]
    max_items: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<Vec<FieldImpl>>,
}

impl TypeDefImpl {
    fn into_type_def<E>(self) -> Result<TypeDef, E>
    where
        E: de::Error,
    {
        let ty = TypeImpl {
            reference: self.reference,
            primitive: self.primitive,
            enum_values: self.enum_values,
            item: self.item,
            min_items: self.min_items,
            max_items: self.max_items,
            properties: self.properties,
        }.into_type(self.name.as_str())?;

        Ok(TypeDef {
            name: self.name,
            description: self.description,
            experimental: self.experimental,
            deprecated: self.deprecated,
            ty: ty,
        })
    }
}

impl<'a> From<&'a TypeDef> for TypeDefImpl {
    fn from(type_def: &'a TypeDef) -> TypeDefImpl {
        let ty = TypeImpl::from(&type_def.ty);
        TypeDefImpl {
            name: type_def.name.clone(),
            description: type_def.description.clone(),
            experimental: type_def.experimental,
            deprecated: type_def.deprecated,
            reference: ty.reference,
            primitive: ty.primitive,
            enum_values: ty.enum_values,
            item: ty.item,
            min_items: ty.min_items,
            max_items: ty.max_items,
            properties: ty.properties,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
struct MethodImpl {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    experimental: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    deprecated: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    handlers: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    parameters: Vec<FieldImpl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    returns: Vec<FieldImpl>,
    #[serde(skip_serializing_if = "Option::is_none")]
    redirect: Option<String>,
}

impl MethodImpl {
    fn into_method<E>(self) -> Result<Method, E>
    where
        E: de::Error,
    {
        Ok(Method {
            name: self.name,
            description: self.description,
            experimental: self.experimental,
            deprecated: self.deprecated,
            handlers: self.handlers,
            parameters: self.parameters
                .into_iter()
                .map(FieldImpl::into_field)
                .collect::<Result<_, _>>()?,
            returns:
                self.returns.into_iter().map(FieldImpl::into_field).collect::<Result<_, _>>()?,
            redirect: self.redirect,
        })
    }
}

impl<'a> From<&'a Method> for MethodImpl {
    fn from(method: &'a Method) -> MethodImpl {
        MethodImpl {
            name: method.name.clone(),
            description: method.description.clone(),
            experimental: method.experimental,
            deprecated: method.deprecated,
            handlers: method.handlers.clone(),
            parameters: method.parameters.iter().map(From::from).collect(),
            returns: method.returns.iter().map(From::from).collect(),
            redirect: method.redirect.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
struct FieldImpl {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    experimental: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    deprecated: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    optional: bool,
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    reference: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    primitive: Option<Primitive>,
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    enum_values: Option<Vec<String>>,
    #[serde(rename = "items", skip_serializing_if = "Option::is_none")]
    item: Option<ItemImpl>,
    #[serde(rename = "minItems", skip_serializing_if = "Option::is_none")]
    min_items: Option<u64>,
    #[serde(rename = "maxItems", skip_serializing_if = "Option::is_none")]
    max_items: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<Vec<FieldImpl>>,
}

impl FieldImpl {
    fn into_field<E>(self) -> Result<Field, E>
    where
        E: de::Error,
    {
        let ty = TypeImpl {
            reference: self.reference,
            primitive: self.primitive,
            enum_values: self.enum_values,
            item: self.item,
            min_items: self.min_items,
            max_items: self.max_items,
            properties: self.properties,
        }.into_type(self.name.as_str())?;

        Ok(Field {
            name: self.name,
            description: self.description,
            experimental: self.experimental,
            deprecated: self.deprecated,
            optional: self.optional,
            ty: ty,
        })
    }
}

impl<'a> From<&'a Field> for FieldImpl {
    fn from(field: &'a Field) -> FieldImpl {
        let ty = TypeImpl::from(&field.ty);
        FieldImpl {
            name: field.name.clone(),
            description: field.description.clone(),
            experimental: field.experimental,
            deprecated: field.deprecated,
            optional: field.optional,
            reference: ty.reference,
            primitive: ty.primitive,
            enum_values: ty.enum_values,
            item: ty.item,
            min_items: ty.min_items,
            max_items: ty.max_items,
            properties: ty.properties,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
struct ItemImpl {
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    reference: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    primitive: Option<Primitive>,
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    enum_values: Option<Vec<String>>,
    #[serde(rename = "items", skip_serializing_if = "Option::is_none")]
    item: Option<Box<ItemImpl>>,
    #[serde(rename = "minItems", skip_serializing_if = "Option::is_none")]
    min_items: Option<u64>,
    #[serde(rename = "maxItems", skip_serializing_if = "Option::is_none")]
    max_items: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    properties: Option<Vec<FieldImpl>>,
}

impl ItemImpl {
    fn into_item<E>(self) -> Result<Item, E>
    where
        E: de::Error,
    {
        let ty = TypeImpl {
            reference: self.reference,
            primitive: self.primitive,
            enum_values: self.enum_values,
            item: self.item.map(|x| *x),
            min_items: self.min_items,
            max_items: self.max_items,
            properties: self.properties,
        };

        Ok(Item {
            description: self.description,
            ty: ty.into_type("array item")?,
        })
    }
}

impl<'a> From<&'a Item> for ItemImpl {
    fn from(item: &'a Item) -> ItemImpl {
        let ty = TypeImpl::from(&item.ty);
        ItemImpl {
            description: item.description.clone(),
            reference: ty.reference,
            primitive: ty.primitive,
            enum_values: ty.enum_values,
            item: ty.item.map(Box::new),
            min_items: ty.min_items,
            max_items: ty.max_items,
            properties: ty.properties,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
enum Primitive {
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "integer")]
    Integer,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "array")]
    Array,
    #[serde(rename = "object")]
    Object,
    #[serde(rename = "any")]
    Any,
}

#[derive(Clone, Debug)]
struct TypeImpl {
    reference: Option<String>,
    primitive: Option<Primitive>,
    enum_values: Option<Vec<String>>,
    item: Option<ItemImpl>,
    min_items: Option<u64>,
    max_items: Option<u64>,
    properties: Option<Vec<FieldImpl>>,
}

impl TypeImpl {
    fn into_type<E>(self, name: &str) -> Result<Type, E>
    where
        E: de::Error,
    {
        if let Some(target) = self.reference {
            return Ok(Type::Reference(target));
        }

        if let Some(primitive) = self.primitive {
            return match primitive {
                Primitive::Boolean => Ok(Type::Boolean),
                Primitive::Integer => Ok(Type::Integer),
                Primitive::Number => Ok(Type::Number),
                Primitive::String => match self.enum_values {
                    None => Ok(Type::String),
                    Some(values) => Ok(Type::Enum(values)),
                },
                Primitive::Array => match self.item {
                    None => Err(de::Error::custom(
                        format!("'items' key not found in array type descriptor for '{}'", name),
                    )),
                    Some(item) => Ok(Type::Array {
                        item: Box::new(item.into_item()?),
                        min_items: self.min_items,
                        max_items: self.max_items,
                    }),
                },
                Primitive::Object => match self.properties {
                    None => Ok(Type::Object(vec![])),
                    Some(properties) => Ok(Type::Object(properties
                        .into_iter()
                        .map(FieldImpl::into_field)
                        .collect::<Result<_, _>>()?)),
                },
                Primitive::Any => Ok(Type::Any),
            };
        }

        Err(de::Error::custom(
            format!("neither 'type' nor '$ref' keys found in type descriptor for '{}'", name),
        ))
    }
}

impl<'a> From<&'a Type> for TypeImpl {
    fn from(ty: &'a Type) -> TypeImpl {
        match *ty {
            Type::Reference(ref target) => TypeImpl {
                reference: Some(target.clone()),
                primitive: None,
                enum_values: None,
                item: None,
                min_items: None,
                max_items: None,
                properties: None,
            },
            Type::Boolean => TypeImpl {
                primitive: Some(Primitive::Boolean),
                reference: None,
                enum_values: None,
                item: None,
                min_items: None,
                max_items: None,
                properties: None,
            },
            Type::Integer => TypeImpl {
                primitive: Some(Primitive::Integer),
                reference: None,
                enum_values: None,
                item: None,
                min_items: None,
                max_items: None,
                properties: None,
            },
            Type::Number => TypeImpl {
                primitive: Some(Primitive::Number),
                reference: None,
                enum_values: None,
                item: None,
                min_items: None,
                max_items: None,
                properties: None,
            },
            Type::String => TypeImpl {
                primitive: Some(Primitive::String),
                reference: None,
                enum_values: None,
                item: None,
                min_items: None,
                max_items: None,
                properties: None,
            },
            Type::Enum(ref values) => TypeImpl {
                primitive: Some(Primitive::String),
                enum_values: Some(values.clone()),
                reference: None,
                item: None,
                min_items: None,
                max_items: None,
                properties: None,
            },
            Type::Array {
                ref item,
                ref min_items,
                ref max_items,
            } => TypeImpl {
                primitive: Some(Primitive::Array),
                item: Some(item.as_ref().into()),
                min_items: *min_items,
                max_items: *max_items,
                reference: None,
                enum_values: None,
                properties: None,
            },
            Type::Object(ref properties) => TypeImpl {
                primitive: Some(Primitive::Object),
                properties: Some(properties.iter().map(From::from).collect()),
                reference: None,
                enum_values: None,
                item: None,
                min_items: None,
                max_items: None,
            },
            Type::Any => TypeImpl {
                primitive: Some(Primitive::Any),
                reference: None,
                enum_values: None,
                item: None,
                min_items: None,
                max_items: None,
                properties: None,
            },
        }
    }
}

fn is_false(x: &bool) -> bool {
    !x
}
