// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

#![cfg_attr(feature = "strict", deny(warnings))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

#[macro_use]
extern crate cdp;

#[test]
fn test_ws_url_format() {
    let expected_ws_url = "ws://localhost:9222/devtools/page/0";
    let actual_ws_url =
        format!(cdp_ws_url_format!(), server_addr = "localhost:9222", page_id = "0");
    assert_eq!(expected_ws_url, actual_ws_url);
}

#[test]
fn test_frontend_url_format() {
    let expected_frontend_url =
        "chrome-devtools://devtools/bundled/inspector.html?ws=localhost:9222/devtools/page/0";
    let actual_frontend_url =
        format!(cdp_frontend_url_format!(), server_addr = "localhost:9222", page_id = "0");
    assert_eq!(expected_frontend_url, actual_frontend_url);
}

#[test]
fn test_parse_path() {
    assert_eq!(cdp::ws::parse_path(""), None);
    assert_eq!(cdp::ws::parse_path("foo"), None);
    assert_eq!(cdp::ws::parse_path("devtools/page"), None);
    assert_eq!(cdp::ws::parse_path("devtools/page/"), Some(""));
    assert_eq!(cdp::ws::parse_path("devtools/page/0"), Some("0"));
    assert_eq!(cdp::ws::parse_path("devtools/page/0/"), Some("0/"));
    assert_eq!(cdp::ws::parse_path("devtools/page/0/1"), Some("0/1"));

    assert_eq!(cdp::ws::parse_path("/"), None);
    assert_eq!(cdp::ws::parse_path("/foo"), None);
    assert_eq!(cdp::ws::parse_path("/devtools/page"), None);
    assert_eq!(cdp::ws::parse_path("/devtools/page/"), None);
    assert_eq!(cdp::ws::parse_path("/devtools/page/0"), None);
    assert_eq!(cdp::ws::parse_path("/devtools/page/0/"), None);
    assert_eq!(cdp::ws::parse_path("/devtools/page/0/1"), None);
}

#[test]
fn test_parse_path_with_slash() {
    assert_eq!(cdp::ws::parse_path_with_slash("/"), None);
    assert_eq!(cdp::ws::parse_path_with_slash("/foo"), None);
    assert_eq!(cdp::ws::parse_path_with_slash("/devtools/page"), None);
    assert_eq!(cdp::ws::parse_path_with_slash("/devtools/page/"), Some(""));
    assert_eq!(cdp::ws::parse_path_with_slash("/devtools/page/0"), Some("0"));
    assert_eq!(cdp::ws::parse_path_with_slash("/devtools/page/0/"), Some("0/"));
    assert_eq!(cdp::ws::parse_path_with_slash("/devtools/page/0/1"), Some("0/1"));

    assert_eq!(cdp::ws::parse_path_with_slash(""), None);
    assert_eq!(cdp::ws::parse_path_with_slash("foo"), None);
    assert_eq!(cdp::ws::parse_path_with_slash("devtools/page"), None);
    assert_eq!(cdp::ws::parse_path_with_slash("devtools/page/"), None);
    assert_eq!(cdp::ws::parse_path_with_slash("devtools/page/0"), None);
    assert_eq!(cdp::ws::parse_path_with_slash("devtools/page/0/"), None);
    assert_eq!(cdp::ws::parse_path_with_slash("devtools/page/0/1"), None);
}
