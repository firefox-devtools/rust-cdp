// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

#![cfg_attr(feature = "strict", deny(warnings))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

extern crate serde_json;

extern crate cdp;

use std::fs::File;
use std::io::prelude::*;

use cdp::definition::Definition;

#[test]
fn test_browser_protocol() {
    do_test_protocol_file("json/browser_protocol.json");
}

#[test]
fn test_js_protocol() {
    do_test_protocol_file("json/js_protocol.json");
}

fn do_test_protocol_file(file: &str) {
    let mut orig_src = String::new();
    File::open(file).unwrap().read_to_string(&mut orig_src).expect("proto def read error");
    let orig_def: Definition =
        serde_json::from_str(orig_src.as_str()).expect("proto def parse error");

    let new_src = serde_json::to_string(&orig_def).expect("proto def serialize error");
    let new_def: Definition =
        serde_json::from_str(new_src.as_str()).expect("proto def (re-)parse error");

    assert_eq!(orig_def, new_def);
    assert_eq!(new_src, serde_json::to_string(&new_def).expect("proto def (re-)serialize error"));
}
