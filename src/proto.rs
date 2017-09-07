// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

#![cfg(feature = "proto")]

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::error::Error;
use std::fmt;

// Protocol

pub use proto_generated::*;

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
