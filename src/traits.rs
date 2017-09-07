// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

use serde::{Deserializer, Serializer};

// * disabled until Firefox moves to Rust 1.20
// pub trait CdpCommand {
// const COMMAND_NAME: &'static str;
// }

pub trait SerializeCdpCommand {
    fn command_name(&self) -> &str;
    fn serialize_command_params<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
}

pub trait DeserializeCdpCommand<'de>: Sized {
    fn deserialize_command<D>(
        command_name: &str,
        command_params: D,
    ) -> Result<Result<Self, D::Error>, D>
    where
        D: Deserializer<'de>;
}

// * disabled until Firefox moves to Rust 1.20
// pub trait CdpEvent {
// const EVENT_NAME: &'static str;
// }

pub trait SerializeCdpEvent {
    fn event_name(&self) -> &str;
    fn serialize_event_params<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
}

pub trait DeserializeCdpEvent<'de>: Sized {
    fn deserialize_event<D>(
        event_name: &str,
        event_params: D,
    ) -> Result<Result<Self, D::Error>, D>
    where
        D: Deserializer<'de>;
}

pub trait HasCdpCommand {
    type Command;
}

pub trait HasCdpResponse {
    type Response;
}
