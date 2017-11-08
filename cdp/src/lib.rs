// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

#![recursion_limit = "128"]
#![cfg_attr(feature = "strict", deny(warnings))]
#![cfg_attr(feature = "strict", deny(missing_debug_implementations))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![doc(html_root_url = "https://docs.rs/cdp/0.1.0")]

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Visitor;
use serde::ser::SerializeStruct;
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::io::{Read, Write};
use std::ops::{Deref, DerefMut};

mod generated;

pub use generated::*;

// Empty Objects

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Empty;

#[derive(Serialize, Deserialize)]
struct EmptyImpl {}

impl Serialize for Empty {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (EmptyImpl {}).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Empty {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        EmptyImpl::deserialize(deserializer).map(|_| Empty)
    }
}

// Enum-Parsing Error Type

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseEnumError {
    pub expected: &'static [&'static str],
    pub actual: String,
}

impl Error for ParseEnumError {
    fn description(&self) -> &str {
        "unexpected value for enum"
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

impl fmt::Display for ParseEnumError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "expected one of {:?}; actual: {:#?}", self.expected, self.actual)
    }
}

// Command Traits

pub trait CdpCommand {
    const COMMAND_NAME: &'static str;
}

pub trait SerializeCdpCommand {
    fn command_name(&self) -> &str;
    fn serialize_command_params<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
}

impl SerializeCdpCommand for (String, Map<String, Value>) {
    fn command_name(&self) -> &str {
        &self.0
    }

    fn serialize_command_params<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.1.serialize(serializer)
    }
}

impl<'a> SerializeCdpCommand for (&'a str, &'a Map<String, Value>) {
    fn command_name(&self) -> &str {
        self.0
    }

    fn serialize_command_params<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.1.serialize(serializer)
    }
}

pub trait DeserializeCdpCommand<'de>: Sized {
    fn deserialize_command<D>(
        command_name: &str,
        command_params: D,
    ) -> Result<Result<Self, D::Error>, D>
    where
        D: Deserializer<'de>;
}

impl<'de> DeserializeCdpCommand<'de> for (String, Map<String, Value>) {
    fn deserialize_command<D>(
        command_name: &str,
        command_params: D,
    ) -> Result<Result<Self, D::Error>, D>
    where
        D: Deserializer<'de>,
    {
        let deserialize_result = Map::<String, Value>::deserialize(command_params);
        Ok(deserialize_result.map(|params| (command_name.into(), params)))
    }
}

// Event Traits

pub trait CdpEvent {
    const EVENT_NAME: &'static str;
}

pub trait SerializeCdpEvent {
    fn event_name(&self) -> &str;
    fn serialize_event_params<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
}

impl SerializeCdpEvent for (String, Map<String, Value>) {
    fn event_name(&self) -> &str {
        &self.0
    }

    fn serialize_event_params<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.1.serialize(serializer)
    }
}

impl<'a> SerializeCdpEvent for (&'a str, &'a Map<String, Value>) {
    fn event_name(&self) -> &str {
        self.0
    }

    fn serialize_event_params<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.1.serialize(serializer)
    }
}

pub trait DeserializeCdpEvent<'de>: Sized {
    fn deserialize_event<D>(
        event_name: &str,
        event_params: D,
    ) -> Result<Result<Self, D::Error>, D>
    where
        D: Deserializer<'de>;
}

impl<'de> DeserializeCdpEvent<'de> for (String, Map<String, Value>) {
    fn deserialize_event<D>(event_name: &str, event_params: D) -> Result<Result<Self, D::Error>, D>
    where
        D: Deserializer<'de>,
    {
        let deserialize_result = Map::<String, Value>::deserialize(event_params);
        Ok(deserialize_result.map(|params| (event_name.into(), params)))
    }
}

// Relations

pub trait HasCdpCommand<'a> {
    type Command: 'a;
}

pub trait HasCdpResponse<'a> {
    type Response: 'a;
}

// Incoming Messages (to the server, from the client)

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CdpIncoming<'a> {
    pub id: u64,
    #[serde(rename = "method")]
    pub command_name: Cow<'a, str>,
    #[serde(rename = "params")]
    pub command_params: CdpParams,
}

impl<'a> CdpIncoming<'a> {
    pub fn parse<'de, D>(deserializer: D) -> Result<Self, (CdpError<'static>, Option<u64>)>
    where
        D: Deserializer<'de>,
    {
        let value =
            Value::deserialize(deserializer).map_err(|_| (CdpError::invalid_message(), None))?;
        let mut obj = match value {
            Value::Object(obj) => obj,
            _ => return Err((CdpError::must_be_object(), None)),
        };
        let id =
            obj.get("id").and_then(Value::as_u64).ok_or_else(|| (CdpError::must_have_id(), None))?;
        let method = obj.remove("method")
            .and_then(|value| match value {
                Value::String(method) => Some(method),
                _ => None,
            })
            .ok_or_else(|| (CdpError::must_have_method(), Some(id)))?;
        let params = match obj.remove("params") {
            Some(Value::Object(params)) => params,
            _ => Map::new(),
        };
        Ok(CdpIncoming {
            id: id,
            command_name: method.into(),
            command_params: params.into(),
        })
    }

    pub fn parse_from_reader<T>(reader: T) -> Result<Self, (CdpError<'static>, Option<u64>)>
    where
        T: Read,
    {
        CdpIncoming::parse(&mut serde_json::Deserializer::from_reader(reader))
    }

    pub fn parse_from_str(src: &str) -> Result<Self, (CdpError<'static>, Option<u64>)> {
        CdpIncoming::parse(&mut serde_json::Deserializer::from_str(src))
    }

    pub fn parse_from_slice(src: &[u8]) -> Result<Self, (CdpError<'static>, Option<u64>)> {
        CdpIncoming::parse(&mut serde_json::Deserializer::from_slice(src))
    }

    pub fn serialize_command<S, C>(serializer: S, id: u64, command: &C) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        C: SerializeCdpCommand,
    {
        struct CommandParams<'a, C: 'a>(&'a C);

        impl<'a, C> Serialize for CommandParams<'a, C>
        where
            C: SerializeCdpCommand,
        {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                self.0.serialize_command_params(serializer)
            }
        }

        let mut state = serializer.serialize_struct("CdpIncoming", 3)?;
        state.serialize_field("id", &id)?;
        state.serialize_field("method", command.command_name())?;
        state.serialize_field("params", &CommandParams(command))?;
        state.end()
    }

    pub fn serialize_command_to_writer<W, C>(
        writer: W,
        id: u64,
        command: &C,
    ) -> Result<(), serde_json::Error>
    where
        W: Write,
        C: SerializeCdpCommand,
    {
        let mut serializer = serde_json::Serializer::new(writer);
        CdpIncoming::serialize_command(&mut serializer, id, command)
    }

    pub fn serialize_command_to_string<C>(
        string: &mut String,
        id: u64,
        command: &C,
    ) -> Result<(), serde_json::Error>
    where
        C: SerializeCdpCommand,
    {
        // serde_json won't produce invalid UTF-8.
        CdpIncoming::serialize_command_to_writer(unsafe { string.as_mut_vec() }, id, command)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CdpParams {
    inner: Map<String, Value>,
}

impl CdpParams {
    pub fn new(inner: Map<String, Value>) -> Self {
        CdpParams { inner: inner }
    }

    pub fn into_inner(self) -> Map<String, Value> {
        self.inner
    }
}

impl Default for CdpParams {
    fn default() -> Self {
        CdpParams::new(Map::new())
    }
}

impl AsRef<Map<String, Value>> for CdpParams {
    fn as_ref(&self) -> &Map<String, Value> {
        &self.inner
    }
}

impl AsMut<Map<String, Value>> for CdpParams {
    fn as_mut(&mut self) -> &mut Map<String, Value> {
        &mut self.inner
    }
}

impl Deref for CdpParams {
    type Target = Map<String, Value>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for CdpParams {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl From<Map<String, Value>> for CdpParams {
    fn from(src: Map<String, Value>) -> Self {
        CdpParams::new(src)
    }
}

impl From<CdpParams> for Map<String, Value> {
    fn from(src: CdpParams) -> Self {
        src.into_inner()
    }
}

impl Serialize for CdpParams {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CdpParams {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(CdpParams::new)
    }
}

impl<'de> Deserializer<'de> for CdpParams {
    type Error = <Value as Deserializer<'de>>::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_any(visitor)
    }
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_bool(visitor)
    }
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_i8(visitor)
    }
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_i16(visitor)
    }
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_i32(visitor)
    }
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_i64(visitor)
    }
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_u8(visitor)
    }
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_u16(visitor)
    }
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_u32(visitor)
    }
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_u64(visitor)
    }
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_f32(visitor)
    }
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_f64(visitor)
    }
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_char(visitor)
    }
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_str(visitor)
    }
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_string(visitor)
    }
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_bytes(visitor)
    }
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_byte_buf(visitor)
    }
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_option(visitor)
    }
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_unit(visitor)
    }
    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_unit_struct(name, visitor)
    }
    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_newtype_struct(name, visitor)
    }
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_seq(visitor)
    }
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_tuple(len, visitor)
    }
    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_tuple_struct(name, len, visitor)
    }
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_map(visitor)
    }
    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_struct(name, fields, visitor)
    }
    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_enum(name, variants, visitor)
    }
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_identifier(visitor)
    }
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.inner).deserialize_ignored_any(visitor)
    }
}

// Outgoing Messages (from the server, to the client)

#[derive(Clone, Debug, PartialEq)]
pub enum CdpOutgoing<'a> {
    Result {
        id: u64,
        result: Result<Map<String, Value>, CdpError<'a>>,
    },
    Event {
        name: Cow<'a, str>,
        params: Map<String, Value>,
    },
    Error(CdpError<'a>),
}

impl<'a> CdpOutgoing<'a> {
    pub fn serialize_result<S, R>(
        serializer: S,
        id: u64,
        result: Result<&R, &CdpError>,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        R: Serialize,
    {
        match result {
            Ok(response) => CdpOutgoing::serialize_response(serializer, id, response),
            Err(error) => CdpOutgoing::serialize_error(serializer, Some(id), error),
        }
    }

    pub fn serialize_result_to_writer<W, R>(
        writer: W,
        id: u64,
        result: Result<&R, &CdpError>,
    ) -> Result<(), serde_json::Error>
    where
        W: Write,
        R: Serialize,
    {
        let mut serializer = serde_json::Serializer::new(writer);
        CdpOutgoing::serialize_result(&mut serializer, id, result)
    }

    pub fn serialize_result_to_string<R>(
        string: &mut String,
        id: u64,
        result: Result<&R, &CdpError>,
    ) -> Result<(), serde_json::Error>
    where
        R: Serialize,
    {
        // serde_json won't produce invalid UTF-8.
        CdpOutgoing::serialize_result_to_writer(unsafe { string.as_mut_vec() }, id, result)
    }

    pub fn serialize_response<S, R>(
        serializer: S,
        id: u64,
        response: &R,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        R: Serialize,
    {
        struct ResponseParams<'a, C: 'a>(&'a C);

        impl<'a, C> Serialize for ResponseParams<'a, C>
        where
            C: Serialize,
        {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                self.0.serialize(serializer)
            }
        }

        let mut state = serializer.serialize_struct("CdpOutgoing", 2)?;
        state.serialize_field("id", &id)?;
        state.serialize_field("result", &ResponseParams(response))?;
        state.end()
    }

    pub fn serialize_response_to_writer<W, R>(
        writer: W,
        id: u64,
        response: &R,
    ) -> Result<(), serde_json::Error>
    where
        W: Write,
        R: Serialize,
    {
        let mut serializer = serde_json::Serializer::new(writer);
        CdpOutgoing::serialize_response(&mut serializer, id, response)
    }

    pub fn serialize_response_to_string<R>(
        string: &mut String,
        id: u64,
        response: &R,
    ) -> Result<(), serde_json::Error>
    where
        R: Serialize,
    {
        CdpOutgoing::serialize_response_to_writer(
            // serde_json won't produce invalid UTF-8.
            unsafe { string.as_mut_vec() },
            id,
            response,
        )
    }

    pub fn serialize_error<S>(
        serializer: S,
        maybe_id: Option<u64>,
        error: &CdpError,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let n = if maybe_id.is_some() { 2 } else { 1 };
        let mut state = serializer.serialize_struct("CdpOutgoing", n)?;
        if let Some(id) = maybe_id {
            state.serialize_field("id", &id)?;
        }
        state.serialize_field("error", &error)?;
        state.end()
    }

    pub fn serialize_error_to_writer<W>(
        writer: W,
        maybe_id: Option<u64>,
        error: &CdpError,
    ) -> Result<(), serde_json::Error>
    where
        W: Write,
    {
        let mut serializer = serde_json::Serializer::new(writer);
        CdpOutgoing::serialize_error(&mut serializer, maybe_id, error)
    }

    pub fn serialize_error_to_string(
        string: &mut String,
        maybe_id: Option<u64>,
        error: &CdpError,
    ) -> Result<(), serde_json::Error> {
        // serde_json won't produce invalid UTF-8.
        CdpOutgoing::serialize_error_to_writer(unsafe { string.as_mut_vec() }, maybe_id, error)
    }

    pub fn serialize_event<E, S>(serializer: S, event: &E) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        E: SerializeCdpEvent,
    {
        struct EventParams<'a, C: 'a>(&'a C);

        impl<'a, C> Serialize for EventParams<'a, C>
        where
            C: SerializeCdpEvent,
        {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                self.0.serialize_event_params(serializer)
            }
        }

        let mut state = serializer.serialize_struct("CdpOutgoing", 2)?;
        state.serialize_field("method", event.event_name())?;
        state.serialize_field("params", &EventParams(event))?;
        state.end()
    }

    pub fn serialize_event_to_writer<W, E>(writer: W, event: &E) -> Result<(), serde_json::Error>
    where
        W: Write,
        E: SerializeCdpEvent,
    {
        let mut serializer = serde_json::Serializer::new(writer);
        CdpOutgoing::serialize_event(&mut serializer, event)
    }

    pub fn serialize_event_to_string<E>(
        string: &mut String,
        event: &E,
    ) -> Result<(), serde_json::Error>
    where
        E: SerializeCdpEvent,
    {
        // serde_json won't produce invalid UTF-8.
        CdpOutgoing::serialize_event_to_writer(unsafe { string.as_mut_vec() }, event)
    }
}

impl<'a> Serialize for CdpOutgoing<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            CdpOutgoing::Result { id, ref result } => {
                CdpOutgoing::serialize_result(serializer, id, result.as_ref())
            }
            CdpOutgoing::Event {
                ref name,
                ref params,
            } => CdpOutgoing::serialize_event(serializer, &(name.as_ref(), params)),
            CdpOutgoing::Error(ref error) => CdpOutgoing::serialize_error(serializer, None, error),
        }
    }
}

impl<'de, 'a> Deserialize<'de> for CdpOutgoing<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(CdpOutgoingImpl::deserialize(deserializer)?.into())
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
enum CdpOutgoingImpl<'a> {
    CommandSuccess { id: u64, result: Map<String, Value> },
    CommandFailure { id: u64, error: CdpError<'a> },
    GeneralFailure { error: CdpError<'a> },
    Event {
        method: Cow<'a, str>,
        params: Map<String, Value>,
    },
}

impl<'a> From<CdpOutgoingImpl<'a>> for CdpOutgoing<'a> {
    #[inline]
    fn from(message: CdpOutgoingImpl<'a>) -> Self {
        match message {
            CdpOutgoingImpl::GeneralFailure { error } => CdpOutgoing::Error(error),
            CdpOutgoingImpl::CommandSuccess { id, result } => CdpOutgoing::Result {
                id: id,
                result: Ok(result),
            },
            CdpOutgoingImpl::CommandFailure { id, error } => CdpOutgoing::Result {
                id: id,
                result: Err(error),
            },
            CdpOutgoingImpl::Event { method, params } => CdpOutgoing::Event {
                name: method,
                params: params,
            },
        }
    }
}

// Protocol Error

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CdpError<'a> {
    #[serde(rename = "code")]
    pub kind: CdpErrorKind,
    pub message: Cow<'a, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl<'a> CdpError<'a> {
    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L254
    pub fn invalid_message() -> Self {
        CdpError {
            kind: CdpErrorKind::ParseError,
            message: "Message must be a valid JSON".into(),
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L259
    pub fn must_be_object() -> Self {
        CdpError {
            kind: CdpErrorKind::InvalidRequest,
            message: "Message must be an object".into(),
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L267
    pub fn must_have_id() -> Self {
        CdpError {
            kind: CdpErrorKind::InvalidRequest,
            message: "Message must have integer 'id' porperty".into(), // not a typo
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L275
    pub fn must_have_method() -> Self {
        CdpError {
            kind: CdpErrorKind::InvalidRequest,
            message: "Message must have string 'method' porperty".into(), // not a typo
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L283
    pub fn method_not_found(method: &str) -> Self {
        CdpError {
            kind: CdpErrorKind::MethodNotFound,
            message: format!("'{}' wasn't found", method).into(),
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/d74a545535868380b028c27dfcdf54e2d5f7c563/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L61
    pub fn invalid_params(message: String) -> Self {
        CdpError {
            kind: CdpErrorKind::InvalidParams,
            message: "Invalid parameters".into(),
            data: Some(Value::String(message)),
        }
    }

    // https://github.com/nodejs/node/blob/d74a545535868380b028c27dfcdf54e2d5f7c563/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L21-L29
    pub fn server_error(message: Cow<'a, str>) -> Self {
        CdpError {
            kind: CdpErrorKind::ServerError,
            message: message,
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L31-L39
    pub fn internal_error(message: String) -> Self {
        CdpError {
            kind: CdpErrorKind::InternalError,
            message: "Internal error".into(),
            data: Some(Value::String(message)),
        }
    }
}

impl<'a> Error for CdpError<'a> {
    fn description(&self) -> &str {
        "cdp error"
    }
}

impl<'a> fmt::Display for CdpError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.data {
            None => write!(
                f,
                "{} (code {}): {}",
                self.description(),
                i32::from(self.kind),
                self.message
            ),
            Some(ref data) => write!(
                f,
                "{} (code {}): {}; {}",
                self.description(),
                i32::from(self.kind),
                self.message,
                data
            ),
        }
    }
}

// https://github.com/nodejs/node/blob/e506bcd899b3530ec69bdc00d5bac469b5753081/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_h.template#L28-L35
#[derive(Clone, Copy, Debug, Eq)]
pub enum CdpErrorKind {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    InternalError,
    ServerError,
    Other(i32),
}

impl PartialEq for CdpErrorKind {
    fn eq(&self, other: &Self) -> bool {
        i32::from(*self) == i32::from(*other)
    }
}

impl fmt::Display for CdpErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CdpErrorKind::ParseError => write!(f, "parse error"),
            CdpErrorKind::InvalidRequest => write!(f, "invalid request"),
            CdpErrorKind::MethodNotFound => write!(f, "method not found"),
            CdpErrorKind::InvalidParams => write!(f, "invalid parameters"),
            CdpErrorKind::InternalError => write!(f, "internal error"),
            CdpErrorKind::ServerError => write!(f, "server error"),
            CdpErrorKind::Other(code) => write!(f, "code {}", code),
        }
    }
}

impl From<i32> for CdpErrorKind {
    #[cfg_attr(feature = "clippy", allow(unreadable_literal))]
    fn from(code: i32) -> Self {
        match code {
            -32700 => CdpErrorKind::ParseError,
            -32600 => CdpErrorKind::InvalidRequest,
            -32601 => CdpErrorKind::MethodNotFound,
            -32602 => CdpErrorKind::InvalidParams,
            -32603 => CdpErrorKind::InternalError,
            -32000 => CdpErrorKind::ServerError,
            _ => CdpErrorKind::Other(code),
        }
    }
}

impl From<CdpErrorKind> for i32 {
    #[cfg_attr(feature = "clippy", allow(unreadable_literal))]
    fn from(kind: CdpErrorKind) -> Self {
        match kind {
            CdpErrorKind::ParseError => -32700,
            CdpErrorKind::InvalidRequest => -32600,
            CdpErrorKind::MethodNotFound => -32601,
            CdpErrorKind::InvalidParams => -32602,
            CdpErrorKind::InternalError => -32603,
            CdpErrorKind::ServerError => -32000,
            CdpErrorKind::Other(code) => code,
        }
    }
}

impl Serialize for CdpErrorKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        i32::from(*self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CdpErrorKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(i32::deserialize(deserializer)?.into())
    }
}
