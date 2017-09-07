// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

use regex::Regex;

#[macro_export]
macro_rules! cdp_ws_path {
    () => ( "devtools/page")
}

#[macro_export]
macro_rules! cdp_ws_path_format {
    () => ( concat!(cdp_ws_path!(), "/{page_id}") )
}

#[macro_export]
macro_rules! cdp_ws_url_format {
    () => ( concat!("ws://{server_addr}/", cdp_ws_path_format!()) )
}

#[macro_export]
macro_rules! cdp_frontend_url_format {
    () => ( concat!("chrome-devtools://devtools/bundled/inspector.html?ws={server_addr}/", cdp_ws_path_format!()) )
}

pub fn parse_path(path: &str) -> Option<&str> {
    lazy_static! {
        static ref WS_PATH_RE: Regex =
            Regex::new(concat!(r"^", cdp_ws_path!(), "/(.*)$"))
                .expect("cdp: WS_PATH_RE compilation failed");
    }

    WS_PATH_RE.captures(path).map(|captures| captures.get(1).map(|m| m.as_str()).unwrap_or(""))
}

pub fn parse_path_with_slash(path: &str) -> Option<&str> {
    if let Some('/') = path.chars().next() {
        parse_path(&path[1..])
    } else {
        None
    }
}
