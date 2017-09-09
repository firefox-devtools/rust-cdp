// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Visitor;
use serde::ser::SerializeStruct;
use serde_json::{self, Map, Value};
use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::io::{Read, Write};

use server::CdpServerCommand;
use traits::{DeserializeCdpCommand, DeserializeCdpEvent, SerializeCdpCommand, SerializeCdpEvent};

// JSON Serialization Impls

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

impl<'de> DeserializeCdpEvent<'de> for (String, Map<String, Value>) {
    fn deserialize_event<D>(event_name: &str, event_params: D) -> Result<Result<Self, D::Error>, D>
    where
        D: Deserializer<'de>,
    {
        let deserialize_result = Map::<String, Value>::deserialize(event_params);
        Ok(deserialize_result.map(|params| (event_name.into(), params)))
    }
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

// JSON Server Command Implementation

#[derive(Clone, Debug)]
pub struct JsonCdpServerCommand {
    name: String,
    params: Map<String, Value>,
}

impl JsonCdpServerCommand {
    pub fn new(name: String, params: Map<String, Value>) -> Self {
        JsonCdpServerCommand {
            name: name,
            params: params,
        }
    }
}

impl CdpServerCommand for JsonCdpServerCommand {
    type Error = serde_json::Error;

    fn command_name(&self) -> &str {
        &self.name
    }

    fn deserialize_command<C>(self) -> Result<Result<C, Self::Error>, Self>
    where
        C: for<'de> DeserializeCdpCommand<'de>,
    {
        let JsonCdpServerCommand { name, params } = self;
        C::deserialize_command(&name, MapDeserializer(params)).map_err(move |params| {
            JsonCdpServerCommand {
                name: name,
                params: params.0,
            }
        })
    }
}

// Incoming Messages (to the server, from the client)

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct JsonCdpIncoming<'a> {
    pub id: u64,
    #[serde(rename = "method")]
    pub command_name: Cow<'a, str>,
    #[serde(rename = "params")]
    pub command_params: Map<String, Value>,
}

impl<'a> JsonCdpIncoming<'a> {
    pub fn parse<'de, D>(deserializer: D) -> Result<Self, (JsonCdpError<'static>, Option<u64>)>
    where
        D: Deserializer<'de>,
    {
        let value =
            Value::deserialize(deserializer).map_err(|_| (JsonCdpError::invalid_json(), None))?;
        let mut obj = match value {
            Value::Object(obj) => obj,
            _ => return Err((JsonCdpError::must_be_object(), None)),
        };
        let id = obj.get("id")
            .and_then(Value::as_u64)
            .ok_or_else(|| (JsonCdpError::must_have_id(), None))?;
        let method = obj.remove("method")
            .and_then(|value| match value {
                Value::String(method) => Some(method),
                _ => None,
            })
            .ok_or_else(|| (JsonCdpError::must_have_method(), Some(id)))?;
        let params = match obj.remove("params") {
            Some(Value::Object(params)) => params,
            _ => Map::new(),
        };
        Ok(JsonCdpIncoming {
            id: id,
            command_name: method.into(),
            command_params: params,
        })
    }

    pub fn parse_from_reader<T>(reader: T) -> Result<Self, (JsonCdpError<'static>, Option<u64>)>
    where
        T: Read,
    {
        JsonCdpIncoming::parse(&mut serde_json::Deserializer::from_reader(reader))
    }

    pub fn parse_from_str(src: &str) -> Result<Self, (JsonCdpError<'static>, Option<u64>)> {
        JsonCdpIncoming::parse(&mut serde_json::Deserializer::from_str(src))
    }

    pub fn parse_from_slice(src: &[u8]) -> Result<Self, (JsonCdpError<'static>, Option<u64>)> {
        JsonCdpIncoming::parse(&mut serde_json::Deserializer::from_slice(src))
    }

    pub fn serialize<S, C>(serializer: S, id: u64, command: &C) -> Result<S::Ok, S::Error>
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

        let mut state = serializer.serialize_struct("JsonCdpIncoming", 3)?;
        state.serialize_field("id", &id)?;
        state.serialize_field("method", command.command_name())?;
        state.serialize_field("params", &CommandParams(command))?;
        state.end()
    }

    pub fn serialize_to_writer<W, C>(
        writer: W,
        id: u64,
        command: &C,
    ) -> Result<(), serde_json::Error>
    where
        W: Write,
        C: SerializeCdpCommand,
    {
        let mut serializer = serde_json::Serializer::new(writer);
        JsonCdpIncoming::serialize(&mut serializer, id, command)
    }

    pub fn serialize_to_string<C>(
        string: &mut String,
        id: u64,
        command: &C,
    ) -> Result<(), serde_json::Error>
    where
        C: SerializeCdpCommand,
    {
        // serde_json won't produce invalid UTF-8.
        JsonCdpIncoming::serialize_to_writer(unsafe { string.as_mut_vec() }, id, command)
    }
}

// Outgoing Messages (from the server, to the client)

#[derive(Clone, Debug, PartialEq)]
pub enum JsonCdpOutgoing<'a> {
    Result {
        id: u64,
        result: Result<Map<String, Value>, JsonCdpError<'a>>,
    },
    Event {
        name: Cow<'a, str>,
        params: Map<String, Value>,
    },
    Error(JsonCdpError<'a>),
}

impl<'a> JsonCdpOutgoing<'a> {
    pub fn serialize_result<S, R>(
        serializer: S,
        id: u64,
        result: Result<&R, &JsonCdpError>,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        R: Serialize,
    {
        match result {
            Ok(response) => JsonCdpOutgoing::serialize_response(serializer, id, response),
            Err(error) => JsonCdpOutgoing::serialize_error(serializer, Some(id), error),
        }
    }

    pub fn serialize_result_to_writer<W, R>(
        writer: W,
        id: u64,
        result: Result<&R, &JsonCdpError>,
    ) -> Result<(), serde_json::Error>
    where
        W: Write,
        R: Serialize,
    {
        let mut serializer = serde_json::Serializer::new(writer);
        JsonCdpOutgoing::serialize_result(&mut serializer, id, result)
    }

    pub fn serialize_result_to_string<R>(
        string: &mut String,
        id: u64,
        result: Result<&R, &JsonCdpError>,
    ) -> Result<(), serde_json::Error>
    where
        R: Serialize,
    {
        // serde_json won't produce invalid UTF-8.
        JsonCdpOutgoing::serialize_result_to_writer(unsafe { string.as_mut_vec() }, id, result)
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

        let mut state = serializer.serialize_struct("JsonCdpOutgoing", 2)?;
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
        JsonCdpOutgoing::serialize_response(&mut serializer, id, response)
    }

    pub fn serialize_response_to_string<R>(
        string: &mut String,
        id: u64,
        response: &R,
    ) -> Result<(), serde_json::Error>
    where
        R: Serialize,
    {
        JsonCdpOutgoing::serialize_response_to_writer(
            // serde_json won't produce invalid UTF-8.
            unsafe { string.as_mut_vec() },
            id,
            response,
        )
    }

    pub fn serialize_error<S>(
        serializer: S,
        id: Option<u64>,
        error: &JsonCdpError,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let n = if id.is_some() { 2 } else { 1 };
        let mut state = serializer.serialize_struct("JsonCdpOutgoing", n)?;
        if let Some(id) = id {
            state.serialize_field("id", &id)?;
        }
        state.serialize_field("error", &error)?;
        state.end()
    }

    pub fn serialize_error_to_writer<W>(
        writer: W,
        id: Option<u64>,
        error: &JsonCdpError,
    ) -> Result<(), serde_json::Error>
    where
        W: Write,
    {
        let mut serializer = serde_json::Serializer::new(writer);
        JsonCdpOutgoing::serialize_error(&mut serializer, id, error)
    }

    pub fn serialize_error_to_string(
        string: &mut String,
        id: Option<u64>,
        error: &JsonCdpError,
    ) -> Result<(), serde_json::Error> {
        // serde_json won't produce invalid UTF-8.
        JsonCdpOutgoing::serialize_error_to_writer(unsafe { string.as_mut_vec() }, id, error)
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

        let mut state = serializer.serialize_struct("JsonCdpOutgoing", 2)?;
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
        JsonCdpOutgoing::serialize_event(&mut serializer, event)
    }

    pub fn serialize_event_to_string<E>(
        string: &mut String,
        event: &E,
    ) -> Result<(), serde_json::Error>
    where
        E: SerializeCdpEvent,
    {
        // serde_json won't produce invalid UTF-8.
        JsonCdpOutgoing::serialize_event_to_writer(unsafe { string.as_mut_vec() }, event)
    }
}

impl<'a> Serialize for JsonCdpOutgoing<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            JsonCdpOutgoing::Result { id, ref result } => {
                JsonCdpOutgoing::serialize_result(serializer, id, result.as_ref())
            }
            JsonCdpOutgoing::Event {
                ref name,
                ref params,
            } => JsonCdpOutgoing::serialize_event(serializer, &(name.as_ref(), params)),
            JsonCdpOutgoing::Error(ref error) => {
                JsonCdpOutgoing::serialize_error(serializer, None, error)
            }
        }
    }
}

impl<'de, 'a> Deserialize<'de> for JsonCdpOutgoing<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(JsonCdpOutgoingImpl::deserialize(deserializer)?.into())
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
enum JsonCdpOutgoingImpl<'a> {
    CommandSuccess { id: u64, result: Map<String, Value> },
    CommandFailure { id: u64, error: JsonCdpError<'a> },
    GeneralFailure { error: JsonCdpError<'a> },
    Event {
        method: Cow<'a, str>,
        params: Map<String, Value>,
    },
}

impl<'a> From<JsonCdpOutgoingImpl<'a>> for JsonCdpOutgoing<'a> {
    #[inline]
    fn from(message: JsonCdpOutgoingImpl<'a>) -> Self {
        match message {
            JsonCdpOutgoingImpl::GeneralFailure { error } => JsonCdpOutgoing::Error(error),
            JsonCdpOutgoingImpl::CommandSuccess { id, result } => JsonCdpOutgoing::Result {
                id: id,
                result: Ok(result),
            },
            JsonCdpOutgoingImpl::CommandFailure { id, error } => JsonCdpOutgoing::Result {
                id: id,
                result: Err(error),
            },
            JsonCdpOutgoingImpl::Event { method, params } => JsonCdpOutgoing::Event {
                name: method,
                params: params,
            },
        }
    }
}

// Error

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct JsonCdpError<'a> {
    #[serde(rename = "code")]
    pub kind: JsonCdpErrorKind,
    pub message: Cow<'a, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl<'a> JsonCdpError<'a> {
    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L254
    pub fn invalid_json() -> Self {
        JsonCdpError {
            kind: JsonCdpErrorKind::ParseError,
            message: "Message must be a valid JSON".into(),
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L259
    pub fn must_be_object() -> Self {
        JsonCdpError {
            kind: JsonCdpErrorKind::InvalidRequest,
            message: "Message must be an object".into(),
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L267
    pub fn must_have_id() -> Self {
        JsonCdpError {
            kind: JsonCdpErrorKind::InvalidRequest,
            message: "Message must have integer 'id' porperty".into(), // not a typo
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L275
    pub fn must_have_method() -> Self {
        JsonCdpError {
            kind: JsonCdpErrorKind::InvalidRequest,
            message: "Message must have string 'method' porperty".into(), // not a typo
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L283
    pub fn method_not_found(method: &str) -> Self {
        JsonCdpError {
            kind: JsonCdpErrorKind::MethodNotFound,
            message: format!("'{}' wasn't found", method).into(),
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/d74a545535868380b028c27dfcdf54e2d5f7c563/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L61
    pub fn invalid_params(message: String) -> Self {
        JsonCdpError {
            kind: JsonCdpErrorKind::InvalidParams,
            message: "Invalid parameters".into(),
            data: Some(Value::String(message)),
        }
    }

    // https://github.com/nodejs/node/blob/d74a545535868380b028c27dfcdf54e2d5f7c563/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L21-L29
    pub fn server_error(message: Cow<'a, str>) -> Self {
        JsonCdpError {
            kind: JsonCdpErrorKind::ServerError,
            message: message,
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L31-L39
    pub fn internal_error() -> Self {
        JsonCdpError {
            kind: JsonCdpErrorKind::InternalError,
            message: "Internal error".into(),
            data: None,
        }
    }
}

impl<'a> Error for JsonCdpError<'a> {
    fn description(&self) -> &str {
        "DevTools error"
    }
}

impl<'a> fmt::Display for JsonCdpError<'a> {
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
pub enum JsonCdpErrorKind {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    InternalError,
    ServerError,
    Other(i32),
}

impl PartialEq for JsonCdpErrorKind {
    fn eq(&self, other: &Self) -> bool {
        i32::from(*self) == i32::from(*other)
    }
}

impl fmt::Display for JsonCdpErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            JsonCdpErrorKind::ParseError => write!(f, "parse error"),
            JsonCdpErrorKind::InvalidRequest => write!(f, "invalid request"),
            JsonCdpErrorKind::MethodNotFound => write!(f, "method not found"),
            JsonCdpErrorKind::InvalidParams => write!(f, "invalid parameters"),
            JsonCdpErrorKind::InternalError => write!(f, "internal error"),
            JsonCdpErrorKind::ServerError => write!(f, "server error"),
            JsonCdpErrorKind::Other(code) => write!(f, "code {}", code),
        }
    }
}

impl From<i32> for JsonCdpErrorKind {
    #[cfg_attr(feature = "clippy", allow(unreadable_literal))]
    fn from(code: i32) -> Self {
        match code {
            -32700 => JsonCdpErrorKind::ParseError,
            -32600 => JsonCdpErrorKind::InvalidRequest,
            -32601 => JsonCdpErrorKind::MethodNotFound,
            -32602 => JsonCdpErrorKind::InvalidParams,
            -32603 => JsonCdpErrorKind::InternalError,
            -32000 => JsonCdpErrorKind::ServerError,
            _ => JsonCdpErrorKind::Other(code),
        }
    }
}

impl From<JsonCdpErrorKind> for i32 {
    #[cfg_attr(feature = "clippy", allow(unreadable_literal))]
    fn from(kind: JsonCdpErrorKind) -> Self {
        match kind {
            JsonCdpErrorKind::ParseError => -32700,
            JsonCdpErrorKind::InvalidRequest => -32600,
            JsonCdpErrorKind::MethodNotFound => -32601,
            JsonCdpErrorKind::InvalidParams => -32602,
            JsonCdpErrorKind::InternalError => -32603,
            JsonCdpErrorKind::ServerError => -32000,
            JsonCdpErrorKind::Other(code) => code,
        }
    }
}

impl Serialize for JsonCdpErrorKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        i32::from(*self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for JsonCdpErrorKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(i32::deserialize(deserializer)?.into())
    }
}

// Deserialize directly to a Map<String, Value>

struct MapDeserializer(Map<String, Value>);

impl<'de> Deserializer<'de> for MapDeserializer {
    type Error = <Value as Deserializer<'de>>::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_any(visitor)
    }
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_bool(visitor)
    }
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_i8(visitor)
    }
    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_i16(visitor)
    }
    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_i32(visitor)
    }
    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_i64(visitor)
    }
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_u8(visitor)
    }
    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_u16(visitor)
    }
    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_u32(visitor)
    }
    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_u64(visitor)
    }
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_f32(visitor)
    }
    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_f64(visitor)
    }
    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_char(visitor)
    }
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_str(visitor)
    }
    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_string(visitor)
    }
    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_bytes(visitor)
    }
    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_byte_buf(visitor)
    }
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_option(visitor)
    }
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_unit(visitor)
    }
    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_unit_struct(name, visitor)
    }
    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_newtype_struct(name, visitor)
    }
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_seq(visitor)
    }
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_tuple(len, visitor)
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
        Value::Object(self.0).deserialize_tuple_struct(name, len, visitor)
    }
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_map(visitor)
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
        Value::Object(self.0).deserialize_struct(name, fields, visitor)
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
        Value::Object(self.0).deserialize_enum(name, variants, visitor)
    }
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_identifier(visitor)
    }
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Value::Object(self.0).deserialize_ignored_any(visitor)
    }
}
