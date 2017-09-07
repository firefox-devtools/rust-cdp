// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

use serde::Deserialize;
use std::error::Error;

use traits::DeserializeCdpEvent;

pub trait CdpClientEvent: Sized {
    type Error: Error;

    fn event_name(&self) -> &str;

    fn deserialize_event<E>(self) -> Result<Result<E, Self::Error>, Self>
    where
        E: for<'de> DeserializeCdpEvent<'de>;
}

pub trait CdpClientResponse: Sized {
    type Error: Error;

    fn deserialize_response<R>(self) -> Result<R, Self::Error>
    where
        R: for<'de> Deserialize<'de>;
}
