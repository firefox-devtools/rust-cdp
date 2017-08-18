#![allow(deprecated)]

// TODO: Review this later
#![cfg_attr(feature = "cargo-clippy", allow(large_enum_variant))]

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use ws::*;

fn serialize_unit_variant_as_empty_struct<S>(serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    (Empty {}).serialize(serializer)
}

fn deserialize_empty_struct_as_unit_variant<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    Empty::deserialize(deserializer).map(|_| ())
}

include!(concat!(env!("OUT_DIR"), "/ws_generated.rs"));
