// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

#![recursion_limit = "128"]

#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

pub mod definition;
mod ws_generated;
#[macro_use]
pub mod ws;
pub mod http;

#[macro_export]
macro_rules! cdp_default_port {
    () => ( 9222 )
}
pub const DEFAULT_PORT: u16 = cdp_default_port!();

include!(concat!(env!("OUT_DIR"), "/constants.rs"));
