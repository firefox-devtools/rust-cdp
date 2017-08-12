#![allow(deprecated)]

use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::SerializeStruct;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::mem;

pub use serde_json::Value;

#[macro_export]
macro_rules! cdp_websocket_devtools_path {
    () => ( "devtools/page")
}

#[macro_export]
macro_rules! cdp_websocket_devtools_path_format {
    () => ( concat!(cdp_websocket_devtools_path!(), "/{page_id}") )
}

#[macro_export]
macro_rules! cdp_websocket_url_format {
    () => ( concat!("ws://{server_addr}/", cdp_websocket_devtools_path_format!()) )
}

pub fn parse_websocket_path(path: &str) -> Option<&str> {
    lazy_static! {
        static ref WEBSOCKET_PATH_RE: Regex =
            Regex::new(concat!(r"^", cdp_websocket_devtools_path!(), "/(.*)$"))
                .expect("cdp: WEBSOCKET_PATH_RE compilation failed");
    }

    WEBSOCKET_PATH_RE.captures(path)
        .map(|captures| captures.get(1).map(|m| m.as_str()).unwrap_or(""))
}

pub fn parse_websocket_path_with_slash(path: &str) -> Option<&str> {
    if let Some('/') = path.chars().next() {
        parse_websocket_path(&path[1..])
    } else {
        None
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[serde(empty_struct)]
pub struct Empty;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClientMessage<'a> {
    pub id: u64,
    pub command: &'a Command,
}

impl<'a> From<&'a OwnedClientMessage> for ClientMessage<'a> {
    fn from(message: &'a OwnedClientMessage) -> Self {
        ClientMessage {
            id: message.id,
            command: &message.command,
        }
    }
}

impl<'a> From<(u64, &'a Command)> for ClientMessage<'a> {
    fn from((id, command): (u64, &'a Command)) -> Self {
        ClientMessage {
            id: id,
            command: command,
        }
    }
}

impl<'a> From<ClientMessage<'a>> for (u64, &'a Command) {
    fn from(message: ClientMessage<'a>) -> Self {
        (message.id, message.command)
    }
}

impl<'a> Serialize for ClientMessage<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        self.command.serialize(ClientMessageSerializer {
            id: self.id,
            inner: serializer,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct OwnedClientMessage {
    pub id: u64,
    pub command: Command,
}

impl OwnedClientMessage {
    pub fn parse_incoming<'de, D>(deserializer: D) -> Result<Self, (DevToolsError, Option<u64>)>
        where D: Deserializer<'de>
    {
        let value =
            Value::deserialize(deserializer).map_err(|_| (DevToolsError::invalid_json(), None))?;
        let obj = value.as_object().ok_or_else(|| (DevToolsError::must_be_object(), None))?;
        let id = obj.get("id")
            .and_then(Value::as_u64)
            .ok_or_else(|| (DevToolsError::must_have_id(), None))?;
        let method = obj.get("method")
            .and_then(Value::as_str)
            .ok_or_else(|| (DevToolsError::must_have_method(), Some(id)))?;
        let default_params = Value::Null;
        let params = obj.get("params").unwrap_or(&default_params);
        let command = Command::parse_command(method, params)
            .ok_or_else(|| (DevToolsError::method_not_found(method), Some(id)))?
            .map_err(|e| (DevToolsError::invalid_parameters(e.to_string()), Some(id)))?;
        Ok(OwnedClientMessage {
            id: id,
            command: command,
        })
    }
}

impl<'a, 'b> From<&'b ClientMessage<'a>> for OwnedClientMessage {
    fn from(message: &'b ClientMessage<'a>) -> Self {
        OwnedClientMessage {
            id: (*message).id,
            command: (*message).command.clone(),
        }
    }
}

impl<'a> From<ClientMessage<'a>> for OwnedClientMessage {
    fn from(message: ClientMessage<'a>) -> Self {
        OwnedClientMessage {
            id: message.id,
            command: message.command.clone(),
        }
    }
}

impl From<(u64, Command)> for OwnedClientMessage {
    fn from((id, command): (u64, Command)) -> Self {
        OwnedClientMessage {
            id: id,
            command: command,
        }
    }
}

impl From<OwnedClientMessage> for (u64, Command) {
    fn from(message: OwnedClientMessage) -> Self {
        (message.id, message.command)
    }
}

impl Serialize for OwnedClientMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        self.command.serialize(ClientMessageSerializer {
            id: self.id,
            inner: serializer,
        })
    }
}

impl<'de> Deserialize<'de> for OwnedClientMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let mut id = None;
        let command = Command::deserialize(ClientMessageDeserializer {
            id: &mut id,
            inner: deserializer,
        })?;
        Ok(OwnedClientMessage {
            id: id.ok_or(de::Error::missing_field("id"))?,
            command: command,
        })
    }
}

struct ClientMessageSerializer<S> {
    id: u64,
    inner: S,
}

impl<S> Serializer for ClientMessageSerializer<S>
    where S: Serializer
{
    type Ok = S::Ok;
    type Error = S::Error;
    type SerializeSeq = S::SerializeSeq;
    type SerializeTuple = S::SerializeTuple;
    type SerializeTupleStruct = S::SerializeTupleStruct;
    type SerializeTupleVariant = S::SerializeTupleVariant;
    type SerializeMap = S::SerializeMap;
    type SerializeStruct = S::SerializeStruct;
    type SerializeStructVariant = S::SerializeStructVariant;

    fn serialize_struct(self,
                        name: &'static str,
                        len: usize)
                        -> Result<Self::SerializeStruct, Self::Error> {
        let mut state = self.inner.serialize_struct(name, 1 + len)?;
        state.serialize_field("id", &self.id)?;
        Ok(state)
    }

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_bool(v)
    }
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_i8(v)
    }
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_i16(v)
    }
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_i32(v)
    }
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_i64(v)
    }
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_u8(v)
    }
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_u16(v)
    }
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_u32(v)
    }
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_u64(v)
    }
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_f32(v)
    }
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_f64(v)
    }
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_char(v)
    }
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_str(v)
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_bytes(v)
    }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_none()
    }
    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
        where T: Serialize
    {
        self.inner.serialize_some(value)
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_unit()
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_unit_struct(name)
    }
    fn serialize_unit_variant(self,
                              name: &'static str,
                              variant_index: u32,
                              variant: &'static str)
                              -> Result<Self::Ok, Self::Error> {
        self.inner.serialize_unit_variant(name, variant_index, variant)
    }
    fn serialize_newtype_struct<T: ?Sized>(self,
                                           name: &'static str,
                                           value: &T)
                                           -> Result<Self::Ok, Self::Error>
        where T: Serialize
    {
        self.inner.serialize_newtype_struct(name, value)
    }
    fn serialize_newtype_variant<T: ?Sized>(self,
                                            name: &'static str,
                                            variant_index: u32,
                                            variant: &'static str,
                                            value: &T)
                                            -> Result<Self::Ok, Self::Error>
        where T: Serialize
    {
        self.inner.serialize_newtype_variant(name, variant_index, variant, value)
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.inner.serialize_seq(len)
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.inner.serialize_tuple(len)
    }
    fn serialize_tuple_struct(self,
                              name: &'static str,
                              len: usize)
                              -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.inner.serialize_tuple_struct(name, len)
    }
    fn serialize_tuple_variant(self,
                               name: &'static str,
                               variant_index: u32,
                               variant: &'static str,
                               len: usize)
                               -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.inner.serialize_tuple_variant(name, variant_index, variant, len)
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.inner.serialize_map(len)
    }
    fn serialize_struct_variant(self,
                                name: &'static str,
                                variant_index: u32,
                                variant: &'static str,
                                len: usize)
                                -> Result<Self::SerializeStructVariant, Self::Error> {
        self.inner.serialize_struct_variant(name, variant_index, variant, len)
    }
}

#[derive(Deserialize)]
#[serde(field_identifier)]
enum ClientMessageField {
    #[serde(rename = "id")]
    Id,
    #[serde(rename = "method")]
    Method,
    #[serde(rename = "params")]
    Params,
    Other(String),
}

struct ClientMessageDeserializer<'a, D> {
    id: &'a mut Option<u64>,
    inner: D,
}

impl<'a, 'de, D> Deserializer<'de> for ClientMessageDeserializer<'a, D>
    where D: Deserializer<'de>
{
    type Error = D::Error;

    fn deserialize_struct<V>(self,
                             name: &'static str,
                             _fields: &'static [&'static str],
                             visitor: V)
                             -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        const FIELDS: &'static [&'static str] = &["id", "method", "params"];
        self.inner.deserialize_struct(name,
                                      FIELDS,
                                      ClientMessageVisitor {
                                          id: self.id,
                                          inner: visitor,
                                      })
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_any(visitor)
    }
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_bool(visitor)
    }
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_i8(visitor)
    }
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_i16(visitor)
    }
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_i32(visitor)
    }
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_i64(visitor)
    }
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_u8(visitor)
    }
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_u16(visitor)
    }
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_u32(visitor)
    }
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_u64(visitor)
    }
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_f32(visitor)
    }
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_f64(visitor)
    }
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_char(visitor)
    }
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_str(visitor)
    }
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_string(visitor)
    }
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_bytes(visitor)
    }
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_byte_buf(visitor)
    }
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_option(visitor)
    }
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_unit(visitor)
    }
    fn deserialize_unit_struct<V>(self,
                                  name: &'static str,
                                  visitor: V)
                                  -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_unit_struct(name, visitor)
    }
    fn deserialize_newtype_struct<V>(self,
                                     name: &'static str,
                                     visitor: V)
                                     -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_newtype_struct(name, visitor)
    }
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_seq(visitor)
    }
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_tuple(len, visitor)
    }
    fn deserialize_tuple_struct<V>(self,
                                   name: &'static str,
                                   len: usize,
                                   visitor: V)
                                   -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_tuple_struct(name, len, visitor)
    }
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_map(visitor)
    }
    fn deserialize_enum<V>(self,
                           name: &'static str,
                           variants: &'static [&'static str],
                           visitor: V)
                           -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_enum(name, variants, visitor)
    }
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_identifier(visitor)
    }
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: Visitor<'de>
    {
        self.inner.deserialize_ignored_any(visitor)
    }
}

struct ClientMessageVisitor<'a, V> {
    id: &'a mut Option<u64>,
    inner: V,
}

impl<'a, 'de, V> Visitor<'de> for ClientMessageVisitor<'a, V>
    where V: Visitor<'de>
{
    type Value = V::Value;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("struct OwnedClientMessage")
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
        where A: MapAccess<'de>
    {
        self.inner.visit_map(ClientMessageMapAccess {
            id: self.id,
            inner: map,
        })
    }

    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
        where A: SeqAccess<'de>
    {
        self.inner.visit_seq(ClientMessageSeqAccess {
            id: self.id,
            inner: seq,
        })
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
        where E: de::Error
    {
        self.inner.visit_bool(v)
    }
    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
        where E: de::Error
    {
        self.inner.visit_i8(v)
    }
    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
        where E: de::Error
    {
        self.inner.visit_i16(v)
    }
    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
        where E: de::Error
    {
        self.inner.visit_i32(v)
    }
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where E: de::Error
    {
        self.inner.visit_i64(v)
    }
    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
        where E: de::Error
    {
        self.inner.visit_u8(v)
    }
    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
        where E: de::Error
    {
        self.inner.visit_u16(v)
    }
    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
        where E: de::Error
    {
        self.inner.visit_u32(v)
    }
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where E: de::Error
    {
        self.inner.visit_u64(v)
    }
    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
        where E: de::Error
    {
        self.inner.visit_f32(v)
    }
    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
        where E: de::Error
    {
        self.inner.visit_f64(v)
    }
    fn visit_char<E>(self, v: char) -> Result<Self::Value, E>
        where E: de::Error
    {
        self.inner.visit_char(v)
    }
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where E: de::Error
    {
        self.inner.visit_str(v)
    }
    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
        where E: de::Error
    {
        self.inner.visit_borrowed_str(v)
    }
    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where E: de::Error
    {
        self.inner.visit_string(v)
    }
    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where E: de::Error
    {
        self.inner.visit_bytes(v)
    }
    fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
        where E: de::Error
    {
        self.inner.visit_borrowed_bytes(v)
    }
    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
        where E: de::Error
    {
        self.inner.visit_byte_buf(v)
    }
    fn visit_none<E>(self) -> Result<Self::Value, E>
        where E: de::Error
    {
        self.inner.visit_none()
    }
    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where D: Deserializer<'de>
    {
        self.inner.visit_some(deserializer)
    }
    fn visit_unit<E>(self) -> Result<Self::Value, E>
        where E: de::Error
    {
        self.inner.visit_unit()
    }
    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where D: Deserializer<'de>
    {
        self.inner.visit_newtype_struct(deserializer)
    }
    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
        where A: EnumAccess<'de>
    {
        self.inner.visit_enum(data)
    }
}

struct ClientMessageMapAccess<'a, A> {
    id: &'a mut Option<u64>,
    inner: A,
}

impl<'a, 'de, A> MapAccess<'de> for ClientMessageMapAccess<'a, A>
    where A: MapAccess<'de>
{
    type Error = A::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
        where K: DeserializeSeed<'de>
    {
        loop {
            match self.inner.next_key::<ClientMessageField>()? {
                None => return Ok(None),
                Some(ClientMessageField::Id) => {
                    if self.id.is_some() {
                        return Err(de::Error::duplicate_field("id"));
                    }
                    mem::replace(self.id, Some(self.inner.next_value::<u64>()?));
                }
                Some(ClientMessageField::Method) => {
                    return seed.deserialize("method".into_deserializer()).map(Some);
                }
                Some(ClientMessageField::Params) => {
                    return seed.deserialize("params".into_deserializer()).map(Some);
                }
                Some(ClientMessageField::Other(key)) => {
                    return seed.deserialize(key.into_deserializer()).map(Some);
                }
            }
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
        where V: DeserializeSeed<'de>
    {
        self.inner.next_value_seed(seed)
    }
}

struct ClientMessageSeqAccess<'a, A> {
    id: &'a mut Option<u64>,
    inner: A,
}

impl<'a, 'de, A> SeqAccess<'de> for ClientMessageSeqAccess<'a, A>
    where A: SeqAccess<'de>
{
    type Error = A::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
        where T: DeserializeSeed<'de>
    {
        if self.id.is_none() {
            match self.inner.next_element::<u64>()? {
                None => return Err(de::Error::invalid_length(0, &"tuple of 3 elements")),
                Some(value) => {
                    mem::replace(self.id, Some(value));
                }
            }
        }
        self.inner.next_element_seed(seed)
    }
}

#[derive(Copy, Debug, PartialEq)]
pub enum ServerMessage<'a, T: 'a = Value> {
    Response {
        id: u64,
        result: Result<&'a T, &'a DevToolsError>,
    },
    Event(&'a Event),
    Error(&'a DevToolsError),
}

impl<'a, T> From<&'a OwnedServerMessage<T>> for ServerMessage<'a, T> {
    fn from(message: &'a OwnedServerMessage<T>) -> Self {
        match *message {
            OwnedServerMessage::Response { id, ref result } => {
                ServerMessage::Response {
                    id: id,
                    result: result.as_ref(),
                }
            }
            OwnedServerMessage::Event(ref event) => ServerMessage::Event(event),
            OwnedServerMessage::Error(ref error) => ServerMessage::Error(error),
        }
    }
}

impl<'a, T> Clone for ServerMessage<'a, T> {
    fn clone(&self) -> Self {
        match *self {
            ServerMessage::Response { id, result } => {
                ServerMessage::Response {
                    id: id,
                    result: result,
                }
            }
            ServerMessage::Event(event) => ServerMessage::Event(event),
            ServerMessage::Error(error) => ServerMessage::Error(error),
        }
    }
}

impl<'a, T> From<(u64, Result<&'a T, &'a DevToolsError>)> for ServerMessage<'a, T> {
    fn from((id, result): (u64, Result<&'a T, &'a DevToolsError>)) -> Self {
        ServerMessage::Response {
            id: id,
            result: result,
        }
    }
}

impl<'a, T> From<&'a Event> for ServerMessage<'a, T> {
    fn from(event: &'a Event) -> Self {
        ServerMessage::Event(event)
    }
}

impl<'a, T> From<&'a DevToolsError> for ServerMessage<'a, T> {
    fn from(error: &'a DevToolsError) -> Self {
        ServerMessage::Error(error)
    }
}

impl<'a, T> Serialize for ServerMessage<'a, T>
    where T: Serialize
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        ServerMessageImpl::from(self).serialize(serializer)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum OwnedServerMessage<T = Value> {
    Response {
        id: u64,
        result: Result<T, DevToolsError>,
    },
    Event(Event),
    Error(DevToolsError),
}

impl<'a, 'b, T> From<&'b ServerMessage<'a, T>> for OwnedServerMessage<T>
    where T: Clone
{
    fn from(message: &'b ServerMessage<'a, T>) -> Self {
        match *message {
            ServerMessage::Response { id, result } => {
                OwnedServerMessage::Response {
                    id: id,
                    result: result.map(Clone::clone).map_err(Clone::clone),
                }
            }
            ServerMessage::Event(event) => OwnedServerMessage::Event(event.clone()),
            ServerMessage::Error(error) => OwnedServerMessage::Error(error.clone()),
        }
    }
}

impl<'a, T> From<ServerMessage<'a, T>> for OwnedServerMessage<T>
    where T: Clone
{
    fn from(message: ServerMessage<'a, T>) -> Self {
        match message {
            ServerMessage::Response { id, result } => {
                OwnedServerMessage::Response {
                    id: id,
                    result: result.map(Clone::clone).map_err(Clone::clone),
                }
            }
            ServerMessage::Event(event) => OwnedServerMessage::Event(event.clone()),
            ServerMessage::Error(error) => OwnedServerMessage::Error(error.clone()),
        }
    }
}

impl<T> From<(u64, Result<T, DevToolsError>)> for OwnedServerMessage<T> {
    fn from((id, result): (u64, Result<T, DevToolsError>)) -> Self {
        OwnedServerMessage::Response {
            id: id,
            result: result,
        }
    }
}

impl<T> From<Event> for OwnedServerMessage<T> {
    fn from(event: Event) -> Self {
        OwnedServerMessage::Event(event)
    }
}

impl<T> From<DevToolsError> for OwnedServerMessage<T> {
    fn from(error: DevToolsError) -> Self {
        OwnedServerMessage::Error(error)
    }
}

impl<T> Serialize for OwnedServerMessage<T>
    where T: Serialize
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        ServerMessageImpl::from(self).serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for OwnedServerMessage<T>
    where T: Deserialize<'de>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        Ok(ServerMessageImpl::deserialize(deserializer)?.into())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
enum ServerMessageImpl<E, T1, T2> {
    GeneralFailure { error: E },
    CommandSuccess {
        id: u64,
        #[serde(rename = "result")]
        response: T1,
    },
    CommandFailure { id: u64, error: E },
    Event(T2),
}

impl<'a, 'b, T> From<&'b ServerMessage<'a, T>>
    for ServerMessageImpl<&'a DevToolsError, &'a T, &'a Event> {
    #[inline]
    fn from(message: &'b ServerMessage<'a, T>) -> Self {
        match *message {
            ServerMessage::Response { id, result } => {
                match result {
                    Ok(response) => {
                        ServerMessageImpl::CommandSuccess {
                            id: id,
                            response: response,
                        }
                    }
                    Err(error) => {
                        ServerMessageImpl::CommandFailure {
                            id: id,
                            error: error,
                        }
                    }
                }
            }
            ServerMessage::Event(event) => ServerMessageImpl::Event(event),
            ServerMessage::Error(error) => ServerMessageImpl::GeneralFailure { error: error },
        }
    }
}

impl<'a, T> From<&'a OwnedServerMessage<T>>
    for ServerMessageImpl<&'a DevToolsError, &'a T, &'a Event> {
    #[inline]
    fn from(message: &'a OwnedServerMessage<T>) -> Self {
        match *message {
            OwnedServerMessage::Response { id, ref result } => {
                match *result {
                    Ok(ref response) => {
                        ServerMessageImpl::CommandSuccess {
                            id: id,
                            response: response,
                        }
                    }
                    Err(ref error) => {
                        ServerMessageImpl::CommandFailure {
                            id: id,
                            error: error,
                        }
                    }
                }
            }
            OwnedServerMessage::Event(ref event) => ServerMessageImpl::Event(event),
            OwnedServerMessage::Error(ref error) => {
                ServerMessageImpl::GeneralFailure { error: error }
            }
        }
    }
}

impl<T> From<ServerMessageImpl<DevToolsError, T, Event>> for OwnedServerMessage<T> {
    #[inline]
    fn from(message: ServerMessageImpl<DevToolsError, T, Event>) -> Self {
        match message {
            ServerMessageImpl::GeneralFailure { error } => OwnedServerMessage::Error(error),
            ServerMessageImpl::CommandSuccess { id, response } => {
                OwnedServerMessage::Response {
                    id: id,
                    result: Ok(response),
                }
            }
            ServerMessageImpl::CommandFailure { id, error } => {
                OwnedServerMessage::Response {
                    id: id,
                    result: Err(error),
                }
            }
            ServerMessageImpl::Event(event) => OwnedServerMessage::Event(event),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DevToolsError {
    #[serde(rename = "code")]
    pub kind: DevToolsErrorKind,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl DevToolsError {
    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L254
    pub fn invalid_json() -> Self {
        DevToolsError {
            kind: DevToolsErrorKind::ParseError,
            message: "Message must be a valid JSON".into(),
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L259
    pub fn must_be_object() -> Self {
        DevToolsError {
            kind: DevToolsErrorKind::InvalidRequest,
            message: "Message must be an object".into(),
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L267
    pub fn must_have_id() -> Self {
        DevToolsError {
            kind: DevToolsErrorKind::InvalidRequest,
            message: "Message must have integer 'id' porperty".into(), // not a typo
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L275
    pub fn must_have_method() -> Self {
        DevToolsError {
            kind: DevToolsErrorKind::InvalidRequest,
            message: "Message must have string 'method' porperty".into(), // not a typo
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L283
    pub fn method_not_found<S>(method: S) -> Self
        where S: AsRef<str>
    {
        DevToolsError {
            kind: DevToolsErrorKind::MethodNotFound,
            message: format!("'{}' wasn't found", method.as_ref()),
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/d74a545535868380b028c27dfcdf54e2d5f7c563/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L61
    pub fn invalid_parameters<S>(message: S) -> Self
        where S: AsRef<str>
    {
        DevToolsError {
            kind: DevToolsErrorKind::InvalidParams,
            message: "Invalid parameters".into(),
            data: Some(Value::String(message.as_ref().into())),
        }
    }

    // https://github.com/nodejs/node/blob/d74a545535868380b028c27dfcdf54e2d5f7c563/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L21-L29
    pub fn server_error<S>(message: S, data: Option<Value>) -> Self
        where S: AsRef<str>
    {
        DevToolsError {
            kind: DevToolsErrorKind::ServerError,
            message: message.as_ref().into(),
            data: data,
        }
    }

    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L31-L39
    pub fn internal_error(data: Option<Value>) -> Self {
        DevToolsError {
            kind: DevToolsErrorKind::InternalError,
            message: "Internal error".into(),
            data: data,
        }
    }
}

impl Error for DevToolsError {
    fn description(&self) -> &str {
        "DevTools error"
    }
}

impl Display for DevToolsError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.data {
            None => {
                write!(f,
                       "{} (code {}): {}",
                       self.description(),
                       i32::from(self.kind),
                       self.message)
            }
            Some(ref data) => {
                write!(f,
                       "{} (code {}): {}; {}",
                       self.description(),
                       i32::from(self.kind),
                       self.message,
                       data)
            }
        }
    }
}

// https://github.com/nodejs/node/blob/e506bcd899b3530ec69bdc00d5bac469b5753081/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_h.template#L28-L35
#[derive(Clone, Copy, Debug, Eq)]
pub enum DevToolsErrorKind {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    InternalError,
    ServerError,
    Other(i32),
}

impl PartialEq for DevToolsErrorKind {
    fn eq(&self, other: &Self) -> bool {
        i32::from(*self) == i32::from(*other)
    }
}

impl Display for DevToolsErrorKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            DevToolsErrorKind::ParseError => write!(f, "parse error"),
            DevToolsErrorKind::InvalidRequest => write!(f, "invalid request"),
            DevToolsErrorKind::MethodNotFound => write!(f, "method not found"),
            DevToolsErrorKind::InvalidParams => write!(f, "invalid parameters"),
            DevToolsErrorKind::InternalError => write!(f, "internal error"),
            DevToolsErrorKind::ServerError => write!(f, "server error"),
            DevToolsErrorKind::Other(code) => write!(f, "code {}", code),
        }
    }
}

impl From<i32> for DevToolsErrorKind {
    fn from(code: i32) -> Self {
        match code {
            -32700 => DevToolsErrorKind::ParseError,
            -32600 => DevToolsErrorKind::InvalidRequest,
            -32601 => DevToolsErrorKind::MethodNotFound,
            -32602 => DevToolsErrorKind::InvalidParams,
            -32603 => DevToolsErrorKind::InternalError,
            -32000 => DevToolsErrorKind::ServerError,
            _ => DevToolsErrorKind::Other(code),
        }
    }
}

impl From<DevToolsErrorKind> for i32 {
    fn from(kind: DevToolsErrorKind) -> Self {
        match kind {
            DevToolsErrorKind::ParseError => -32700,
            DevToolsErrorKind::InvalidRequest => -32600,
            DevToolsErrorKind::MethodNotFound => -32601,
            DevToolsErrorKind::InvalidParams => -32602,
            DevToolsErrorKind::InternalError => -32603,
            DevToolsErrorKind::ServerError => -32000,
            DevToolsErrorKind::Other(code) => code,
        }
    }
}

impl Serialize for DevToolsErrorKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        i32::from(*self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DevToolsErrorKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        Ok(i32::deserialize(deserializer)?.into())
    }
}

include!(concat!(env!("OUT_DIR"), "/websocket.rs"));
