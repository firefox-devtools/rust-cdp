// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};
use std::error::Error;
use std::fmt;

// Protocol

pub use tools_generated::*;

pub trait SerializeToolsCommand {
    fn command_name(&self) -> &str;
    fn serialize_command_params<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
}

impl SerializeToolsCommand for (String, Map<String, Value>) {
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

impl<'a> SerializeToolsCommand for (&'a str, &'a Map<String, Value>) {
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

pub trait DeserializeToolsCommand<'de>: Sized {
    fn deserialize_command<D>(
        command_name: &str,
        command_params: D,
    ) -> Result<Result<Self, D::Error>, D>
    where
        D: Deserializer<'de>;
}

pub trait SerializeToolsEvent {
    fn event_name(&self) -> &str;
    fn serialize_event_params<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
}

impl SerializeToolsEvent for (String, Map<String, Value>) {
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

impl<'a> SerializeToolsEvent for (&'a str, &'a Map<String, Value>) {
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

pub trait DeserializeToolsEvent<'de>: Sized {
    fn deserialize_event<D>(
        event_name: &str,
        event_params: D,
    ) -> Result<Result<Self, D::Error>, D>
    where
        D: Deserializer<'de>;
}

pub trait HasToolsCommand {
    type Command;
}

pub trait HasToolsResponse {
    type Response;
}

// Types

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
