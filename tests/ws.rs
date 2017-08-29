// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

#![cfg_attr(feature = "strict", deny(warnings))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

extern crate cdp;

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
