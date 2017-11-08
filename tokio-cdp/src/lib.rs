// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

#![cfg_attr(feature = "strict", deny(warnings))]
#![cfg_attr(feature = "strict", deny(missing_debug_implementations))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![doc(html_root_url = "https://docs.rs/tokio-cdp/0.1.0")]

#[macro_use]
extern crate futures;
extern crate hyper;
extern crate hyper_websocket;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate quick_error;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_service;
extern crate websocket;
extern crate websocket_transport;

mod common;
mod private;

pub mod greeter;
pub mod tools;
