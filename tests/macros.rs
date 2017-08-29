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
fn test_version_info_url_format() {
    let expected_version_info_url = "http://localhost:9222/json/version";
    let actual_version_info_url =
        format!(cdp_greeter_version_info_url_format!(), server_addr = "localhost:9222");
    assert_eq!(expected_version_info_url, actual_version_info_url);
}

#[test]
fn test_page_list_url_format() {
    let expected_page_list_url = "http://localhost:9222/json/list";
    let actual_page_list_url =
        format!(cdp_greeter_page_list_url_format!(), server_addr = "localhost:9222");
    assert_eq!(expected_page_list_url, actual_page_list_url);
}

#[test]
fn test_new_page_url_format() {
    let expected_new_page_url = "http://localhost:9222/json/new";
    let actual_new_page_url =
        format!(cdp_greeter_new_page_url_format!(), server_addr = "localhost:9222");
    assert_eq!(expected_new_page_url, actual_new_page_url);
}

#[test]
fn test_new_page_and_nav_url_format() {
    let expected_new_page_and_nav_url = "http://localhost:9222/json/new?https://www.mozilla.org";
    let actual_new_page_and_nav_url = format!(
        cdp_greeter_new_page_and_navigate_url_format!(),
        server_addr = "localhost:9222",
        url = "https://www.mozilla.org"
    );
    assert_eq!(expected_new_page_and_nav_url, actual_new_page_and_nav_url);
}

#[test]
fn test_activate_page_url_format() {
    let expected_activate_page_url = "http://localhost:9222/json/activate/0";
    let actual_activate_page_url = format!(
        cdp_greeter_activate_page_url_format!(),
        server_addr = "localhost:9222",
        page_id = "0"
    );
    assert_eq!(expected_activate_page_url, actual_activate_page_url);
}
