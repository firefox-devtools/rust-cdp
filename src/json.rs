// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::ser::SerializeStruct;
use serde_json::{self, Map, Value};
use std::error::Error;
use std::fmt;
use std::io::{Read, Write};

use tools::{SerializeToolsCommand, SerializeToolsEvent};

// Incoming Messages (to the server, from the client)

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct JsonToolsIncoming {
    pub id: u64,
    #[serde(rename = "method")]
    pub command_name: String,
    #[serde(rename = "params")]
    pub command_params: Map<String, Value>,
}

impl JsonToolsIncoming {
    pub fn parse<'de, D>(deserializer: D) -> Result<Self, (JsonToolsError, Option<u64>)>
    where
        D: Deserializer<'de>,
    {
        let value =
            Value::deserialize(deserializer).map_err(|_| (JsonToolsError::invalid_json(), None))?;
        let mut obj = match value {
            Value::Object(obj) => obj,
            _ => return Err((JsonToolsError::must_be_object(), None)),
        };
        let id = obj.get("id")
            .and_then(Value::as_u64)
            .ok_or_else(|| (JsonToolsError::must_have_id(), None))?;
        let method = obj.remove("method")
            .and_then(|value| match value {
                Value::String(method) => Some(method),
                _ => None,
            })
            .ok_or_else(|| (JsonToolsError::must_have_method(), Some(id)))?;
        let params = match obj.remove("params") {
            Some(Value::Object(params)) => params,
            _ => Map::new(),
        };
        Ok(JsonToolsIncoming {
            id: id,
            command_name: method,
            command_params: params,
        })
    }

    pub fn parse_from_reader<T>(reader: T) -> Result<Self, (JsonToolsError, Option<u64>)>
    where
        T: Read,
    {
        JsonToolsIncoming::parse(&mut serde_json::Deserializer::from_reader(reader))
    }

    pub fn parse_from_str(src: &str) -> Result<Self, (JsonToolsError, Option<u64>)> {
        JsonToolsIncoming::parse(&mut serde_json::Deserializer::from_str(src))
    }

    pub fn parse_from_slice(src: &[u8]) -> Result<Self, (JsonToolsError, Option<u64>)> {
        JsonToolsIncoming::parse(&mut serde_json::Deserializer::from_slice(src))
    }

    pub fn serialize<S, C>(serializer: S, id: u64, command: &C) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        C: SerializeToolsCommand,
    {
        struct CommandParams<'a, C: 'a>(&'a C);

        impl<'a, C> Serialize for CommandParams<'a, C>
        where
            C: SerializeToolsCommand,
        {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                self.0.serialize_command_params(serializer)
            }
        }

        let mut state = serializer.serialize_struct("JsonToolsIncoming", 3)?;
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
        C: SerializeToolsCommand,
    {
        let mut serializer = serde_json::Serializer::new(writer);
        JsonToolsIncoming::serialize(&mut serializer, id, command)
    }

    pub fn serialize_to_string<C>(
        string: &mut String,
        id: u64,
        command: &C,
    ) -> Result<(), serde_json::Error>
    where
        C: SerializeToolsCommand,
    {
        // serde_json won't produce invalid UTF-8.
        JsonToolsIncoming::serialize_to_writer(unsafe { string.as_mut_vec() }, id, command)
    }
}

// Outgoing Messages (from the server, to the client)

#[derive(Clone, Debug, PartialEq)]
pub enum JsonToolsOutgoing {
    Result {
        id: u64,
        result: Result<Map<String, Value>, JsonToolsError>,
    },
    Event {
        name: String,
        params: Map<String, Value>,
    },
    Error(JsonToolsError),
}

impl JsonToolsOutgoing {
    pub fn serialize_result<S, R>(
        serializer: S,
        id: u64,
        result: Result<&R, &JsonToolsError>,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        R: Serialize,
    {
        match result {
            Ok(response) => JsonToolsOutgoing::serialize_response(serializer, id, response),
            Err(error) => JsonToolsOutgoing::serialize_error(serializer, Some(id), error),
        }
    }

    pub fn serialize_result_to_writer<W, R>(
        writer: W,
        id: u64,
        result: Result<&R, &JsonToolsError>,
    ) -> Result<(), serde_json::Error>
    where
        W: Write,
        R: Serialize,
    {
        let mut serializer = serde_json::Serializer::new(writer);
        JsonToolsOutgoing::serialize_result(&mut serializer, id, result)
    }

    pub fn serialize_result_to_string<R>(
        string: &mut String,
        id: u64,
        result: Result<&R, &JsonToolsError>,
    ) -> Result<(), serde_json::Error>
    where
        R: Serialize,
    {
        // serde_json won't produce invalid UTF-8.
        JsonToolsOutgoing::serialize_result_to_writer(unsafe { string.as_mut_vec() }, id, result)
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

        let mut state = serializer.serialize_struct("JsonToolsOutgoing", 2)?;
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
        JsonToolsOutgoing::serialize_response(&mut serializer, id, response)
    }

    pub fn serialize_response_to_string<R>(
        string: &mut String,
        id: u64,
        response: &R,
    ) -> Result<(), serde_json::Error>
    where
        R: Serialize,
    {
        JsonToolsOutgoing::serialize_response_to_writer(
            // serde_json won't produce invalid UTF-8.
            unsafe { string.as_mut_vec() },
            id,
            response,
        )
    }

    pub fn serialize_error<S>(
        serializer: S,
        id: Option<u64>,
        error: &JsonToolsError,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let n = if id.is_some() { 2 } else { 1 };
        let mut state = serializer.serialize_struct("JsonToolsOutgoing", n)?;
        if let Some(id) = id {
            state.serialize_field("id", &id)?;
        }
        state.serialize_field("error", &error)?;
        state.end()
    }

    pub fn serialize_error_to_writer<W>(
        writer: W,
        id: Option<u64>,
        error: &JsonToolsError,
    ) -> Result<(), serde_json::Error>
    where
        W: Write,
    {
        let mut serializer = serde_json::Serializer::new(writer);
        JsonToolsOutgoing::serialize_error(&mut serializer, id, error)
    }

    pub fn serialize_error_to_string(
        string: &mut String,
        id: Option<u64>,
        error: &JsonToolsError,
    ) -> Result<(), serde_json::Error> {
        // serde_json won't produce invalid UTF-8.
        JsonToolsOutgoing::serialize_error_to_writer(unsafe { string.as_mut_vec() }, id, error)
    }

    pub fn serialize_event<E, S>(serializer: S, event: &E) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        E: SerializeToolsEvent,
    {
        struct EventParams<'a, C: 'a>(&'a C);

        impl<'a, C> Serialize for EventParams<'a, C>
        where
            C: SerializeToolsEvent,
        {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                self.0.serialize_event_params(serializer)
            }
        }

        let mut state = serializer.serialize_struct("JsonToolsOutgoing", 2)?;
        state.serialize_field("method", event.event_name())?;
        state.serialize_field("params", &EventParams(event))?;
        state.end()
    }

    pub fn serialize_event_to_writer<W, E>(writer: W, event: &E) -> Result<(), serde_json::Error>
    where
        W: Write,
        E: SerializeToolsEvent,
    {
        let mut serializer = serde_json::Serializer::new(writer);
        JsonToolsOutgoing::serialize_event(&mut serializer, event)
    }

    pub fn serialize_event_to_string<E>(
        string: &mut String,
        event: &E,
    ) -> Result<(), serde_json::Error>
    where
        E: SerializeToolsEvent,
    {
        // serde_json won't produce invalid UTF-8.
        JsonToolsOutgoing::serialize_event_to_writer(unsafe { string.as_mut_vec() }, event)
    }
}

impl Serialize for JsonToolsOutgoing {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            JsonToolsOutgoing::Result { id, ref result } => {
                JsonToolsOutgoing::serialize_result(serializer, id, result.as_ref())
            }
            JsonToolsOutgoing::Event {
                ref name,
                ref params,
            } => JsonToolsOutgoing::serialize_event(serializer, &(name.as_str(), params)),
            JsonToolsOutgoing::Error(ref error) => {
                JsonToolsOutgoing::serialize_error(serializer, None, error)
            }
        }
    }
}

impl<'de> Deserialize<'de> for JsonToolsOutgoing {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(JsonToolsOutgoingImpl::deserialize(deserializer)?.into())
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
enum JsonToolsOutgoingImpl {
    CommandSuccess { id: u64, result: Map<String, Value> },
    CommandFailure { id: u64, error: JsonToolsError },
    GeneralFailure { error: JsonToolsError },
    Event {
        method: String,
        params: Map<String, Value>,
    },
}

impl From<JsonToolsOutgoingImpl> for JsonToolsOutgoing {
    #[inline]
    fn from(message: JsonToolsOutgoingImpl) -> Self {
        match message {
            JsonToolsOutgoingImpl::GeneralFailure { error } => JsonToolsOutgoing::Error(error),
            JsonToolsOutgoingImpl::CommandSuccess { id, result } => JsonToolsOutgoing::Result {
                id: id,
                result: Ok(result),
            },
            JsonToolsOutgoingImpl::CommandFailure { id, error } => JsonToolsOutgoing::Result {
                id: id,
                result: Err(error),
            },
            JsonToolsOutgoingImpl::Event { method, params } => JsonToolsOutgoing::Event {
                name: method,
                params: params,
            },
        }
    }
}

// Error

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct JsonToolsError {
    #[serde(rename = "code")]
    pub kind: JsonToolsErrorKind,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonToolsError {
    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L254
    pub fn invalid_json() -> Self {
        JsonToolsError {
            kind: JsonToolsErrorKind::ParseError,
            message: "Message must be a valid JSON".into(),
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L259
    pub fn must_be_object() -> Self {
        JsonToolsError {
            kind: JsonToolsErrorKind::InvalidRequest,
            message: "Message must be an object".into(),
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L267
    pub fn must_have_id() -> Self {
        JsonToolsError {
            kind: JsonToolsErrorKind::InvalidRequest,
            message: "Message must have integer 'id' porperty".into(), // not a typo
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L275
    pub fn must_have_method() -> Self {
        JsonToolsError {
            kind: JsonToolsErrorKind::InvalidRequest,
            message: "Message must have string 'method' porperty".into(), // not a typo
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L283
    pub fn method_not_found(method: &str) -> Self {
        JsonToolsError {
            kind: JsonToolsErrorKind::MethodNotFound,
            message: format!("'{}' wasn't found", method),
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/d74a545535868380b028c27dfcdf54e2d5f7c563/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L61
    pub fn invalid_params(message: String) -> Self {
        JsonToolsError {
            kind: JsonToolsErrorKind::InvalidParams,
            message: "Invalid parameters".into(),
            data: Some(Value::String(message)),
        }
    }

    // https://github.com/nodejs/node/blob/d74a545535868380b028c27dfcdf54e2d5f7c563/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L21-L29
    pub fn server_error(message: String) -> Self {
        JsonToolsError {
            kind: JsonToolsErrorKind::ServerError,
            message: message,
            data: None,
        }
    }

    // https://github.com/nodejs/node/blob/8a8a6865c092637515b286cd9575ea592b5f501e/deps/v8/third_party/inspector_protocol/lib/DispatcherBase_cpp.template#L31-L39
    pub fn internal_error() -> Self {
        JsonToolsError {
            kind: JsonToolsErrorKind::InternalError,
            message: "Internal error".into(),
            data: None,
        }
    }
}

impl Error for JsonToolsError {
    fn description(&self) -> &str {
        "DevTools error"
    }
}

impl fmt::Display for JsonToolsError {
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
pub enum JsonToolsErrorKind {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    InternalError,
    ServerError,
    Other(i32),
}

impl PartialEq for JsonToolsErrorKind {
    fn eq(&self, other: &Self) -> bool {
        i32::from(*self) == i32::from(*other)
    }
}

impl fmt::Display for JsonToolsErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            JsonToolsErrorKind::ParseError => write!(f, "parse error"),
            JsonToolsErrorKind::InvalidRequest => write!(f, "invalid request"),
            JsonToolsErrorKind::MethodNotFound => write!(f, "method not found"),
            JsonToolsErrorKind::InvalidParams => write!(f, "invalid parameters"),
            JsonToolsErrorKind::InternalError => write!(f, "internal error"),
            JsonToolsErrorKind::ServerError => write!(f, "server error"),
            JsonToolsErrorKind::Other(code) => write!(f, "code {}", code),
        }
    }
}

impl From<i32> for JsonToolsErrorKind {
    #[cfg_attr(feature = "clippy", allow(unreadable_literal))]
    fn from(code: i32) -> Self {
        match code {
            -32700 => JsonToolsErrorKind::ParseError,
            -32600 => JsonToolsErrorKind::InvalidRequest,
            -32601 => JsonToolsErrorKind::MethodNotFound,
            -32602 => JsonToolsErrorKind::InvalidParams,
            -32603 => JsonToolsErrorKind::InternalError,
            -32000 => JsonToolsErrorKind::ServerError,
            _ => JsonToolsErrorKind::Other(code),
        }
    }
}

impl From<JsonToolsErrorKind> for i32 {
    #[cfg_attr(feature = "clippy", allow(unreadable_literal))]
    fn from(kind: JsonToolsErrorKind) -> Self {
        match kind {
            JsonToolsErrorKind::ParseError => -32700,
            JsonToolsErrorKind::InvalidRequest => -32600,
            JsonToolsErrorKind::MethodNotFound => -32601,
            JsonToolsErrorKind::InvalidParams => -32602,
            JsonToolsErrorKind::InternalError => -32603,
            JsonToolsErrorKind::ServerError => -32000,
            JsonToolsErrorKind::Other(code) => code,
        }
    }
}

impl Serialize for JsonToolsErrorKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        i32::from(*self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for JsonToolsErrorKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(i32::deserialize(deserializer)?.into())
    }
}
