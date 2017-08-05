extern crate serde_json;

extern crate cdp;

use std::fs::File;
use std::io::prelude::*;

use cdp::definition::Definition;

/*
#[test]
fn test_deserialize_definition() {
    let mut src = String::new();
    File::open("browser_protocol.json").unwrap().read_to_string(&mut src).unwrap();
    let def: Definition = serde_json::from_str(src.as_str()).unwrap();
    println!("{:#?}", def);
}
*/
