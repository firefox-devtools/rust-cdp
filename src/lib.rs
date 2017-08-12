#![recursion_limit="128"]

#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

pub mod definition;
#[macro_use]
pub mod websocket;
pub mod http;

#[macro_export]
macro_rules! cdp_default_port {
    () => ( 9222 )
}
pub const DEFAULT_PORT: u16 = cdp_default_port!();

include!(concat!(env!("OUT_DIR"), "/constants.rs"));
