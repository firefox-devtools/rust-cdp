#![recursion_limit="128"]

extern crate regex;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;

pub mod definition;
pub mod http;
pub mod websocket;

#[macro_export]
macro_rules! cdp_default_port {
    () => ( 9222 )
}
pub const DEFAULT_PORT: u16 = cdp_default_port!();

include!(concat!(env!("OUT_DIR"), "/constants.rs"));
