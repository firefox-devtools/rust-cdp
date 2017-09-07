// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

#![cfg_attr(feature = "strict", deny(warnings))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

extern crate serde;

extern crate cdp;

mod helper;

use cdp::proto::{page, Empty, ParseEnumError};
use std::str::FromStr;

#[test]
fn test_empty() {
    let json = "{}";
    let rust = Empty;
    helper::do_test_json(json, &rust);
}

#[test]
fn test_parse_enum() {
    assert_eq!(
        Ok(page::CaptureScreenshotCommandFormat::Jpeg),
        page::CaptureScreenshotCommandFormat::from_str("jpeg")
    );
    assert_eq!(
        Ok(page::CaptureScreenshotCommandFormat::Png),
        page::CaptureScreenshotCommandFormat::from_str("png")
    );

    let err = ParseEnumError {
        expected: page::CaptureScreenshotCommandFormat::STR_VALUES,
        actual: "foo".into(),
    };
    assert_eq!(Err(err), page::CaptureScreenshotCommandFormat::from_str("foo"));
}

#[test]
fn test_display_enum() {
    assert_eq!("jpeg", page::CaptureScreenshotCommandFormat::Jpeg.to_string());
    assert_eq!("png", page::CaptureScreenshotCommandFormat::Png.to_string());
}

#[test]
fn test_command_page_navigate() {
    let json = r#"{"url":"https://www.mozilla.org"}"#;
    let rust = page::NavigateCommand {
        url: "https://www.mozilla.org".into(),
        referrer: None,
        transition_type: None,
    };
    helper::do_test_json(json, &rust);
}

#[test]
fn test_command_parse_page_navigate_extra_field() {
    let json = r#"{"url":"https://www.mozilla.org","foo":"bar"}"#;
    let rust = page::NavigateCommand {
        url: "https://www.mozilla.org".into(),
        referrer: None,
        transition_type: None,
    };
    helper::do_test_json_de(json, &rust);
}

#[test]
fn test_command_page_enable() {
    let json = "{}";
    let rust = page::EnableCommand;
    helper::do_test_json(json, &rust);
}

#[test]
fn test_command_parse_page_enable_extra_field() {
    let json = r#"{"foo":"bar"}"#;
    let rust = page::EnableCommand;
    helper::do_test_json_de(json, &rust);
}

#[test]
fn test_response_page_navigate() {
    let json = r#"{"frameId":"hello"}"#;
    let rust = page::NavigateResponse {
        frame_id: "hello".into(),
    };
    helper::do_test_json(json, &rust);
}

#[test]
fn test_response_parse_page_navigate_extra_field() {
    let json = r#"{"frameId":"hello","foo":"bar"}"#;
    let rust = page::NavigateResponse {
        frame_id: "hello".into(),
    };
    helper::do_test_json_de(json, &rust);
}

#[test]
fn test_event_page_dom_content_event_fired() {
    let json = r#"{"timestamp":12.7}"#;
    let rust = page::DomContentEventFiredEvent { timestamp: 12.7 };
    helper::do_test_json(json, &rust);
}

#[test]
fn test_event_parse_page_dom_content_event_fired_extra_field() {
    let json = r#"{"timestamp":12.7,"foo":"bar"}"#;
    let rust = page::DomContentEventFiredEvent { timestamp: 12.7 };
    helper::do_test_json_de(json, &rust);
}
