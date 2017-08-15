extern crate serde;
extern crate serde_json;

use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub fn do_test_json<T>(json: &str, rust: &T)
    where for<'de> T: Deserialize<'de> + Serialize + PartialEq + Debug
{
    do_test_json_ser(json, rust);
    do_test_json_de(json, rust);
}

pub fn do_test_json_ser<T>(json: &str, rust: &T)
    where T: Serialize + PartialEq + Debug
{
    let ser = serde_json::to_string_pretty(rust).expect("serialize error");
    assert_eq!(json,
               ser,
               "json serialize mismatch\n\nexpected:\n{}\n\nactual:\n{}",
               json,
               ser);
}

pub fn do_test_json_de<T>(json: &str, rust: &T)
    where for<'de> T: Deserialize<'de> + PartialEq + Debug
{
    let de: T = serde_json::from_str(json).expect("deserialize error");
    assert_eq!(rust,
               &de,
               "json deserialize mismatch\n\nexpected:\n{:#?}\n\nactual:\n{:#?}",
               rust,
               &de);
}
