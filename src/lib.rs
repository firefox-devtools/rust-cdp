// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

#![recursion_limit = "128"]
#![cfg_attr(feature = "strict", deny(warnings))]
#![cfg_attr(feature = "strict", deny(missing_debug_implementations))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

#[cfg(feature = "ws")]
#[macro_use]
extern crate lazy_static;

pub mod client;
pub mod definition;
pub mod json;
pub mod server;
pub mod traits;
pub mod ws;

pub mod proto;
mod proto_generated;

#[macro_export]
macro_rules! cdp_default_remote_port {
    () => ( 9222 )
}

pub const DEFAULT_REMOTE_PORT: u16 = cdp_default_remote_port!();

#[macro_export]
macro_rules ! cdp_stable_protocol_version { ( ) => ( "1.2" ) }
pub const STABLE_PROTOCOL_VERSION: &'static str = cdp_stable_protocol_version!();
