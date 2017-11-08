// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

#![cfg_attr(feature = "strict", deny(warnings))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

extern crate serde;
extern crate serde_json;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Debug;
use std::str::FromStr;

extern crate cdp;

use cdp::{page, CdpCommand, CdpError, CdpEvent, CdpIncoming, CdpOutgoing, DeserializeCdpCommand,
          DeserializeCdpEvent, Empty, ParseEnumError, SerializeCdpCommand, SerializeCdpEvent};

#[test]
fn test_empty() {
    let json = "{}";
    let rust = Empty;
    do_test_json(json, &rust);
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
    do_test_json(json, &rust);
}

#[test]
fn test_command_parse_page_navigate_extra_field() {
    let json = r#"{"url":"https://www.mozilla.org","foo":"bar"}"#;
    let rust = page::NavigateCommand {
        url: "https://www.mozilla.org".into(),
        referrer: None,
        transition_type: None,
    };
    do_test_json_de(json, &rust);
}

#[test]
fn test_command_page_navigate_cdp_command_impl() {
    assert_eq!("Page.navigate", page::NavigateCommand::COMMAND_NAME);
}

#[test]
fn test_command_page_navigate_serialize_cdp_command_impl() {
    let json = br#"{"url":"https://www.mozilla.org"}"#;
    let rust = page::NavigateCommand {
        url: "https://www.mozilla.org".into(),
        referrer: None,
        transition_type: None,
    };

    assert_eq!("Page.navigate", rust.command_name());

    let mut serializer = serde_json::Serializer::new(Vec::new());
    rust.serialize_command_params(&mut serializer).expect("serialize_command_params error");
    assert_eq!(&json[..], serializer.into_inner().as_slice());
}

#[test]
fn test_command_page_navigate_deserialize_cdp_command_impl() {
    let json = r#"{"url":"https://www.mozilla.org"}"#;
    let rust = page::NavigateCommand {
        url: "https://www.mozilla.org".into(),
        referrer: None,
        transition_type: None,
    };

    let mut deserializer = serde_json::Deserializer::from_str(json);
    assert_eq!(
        Some(rust),
        page::NavigateCommand::deserialize_command("Page.navigate", &mut deserializer)
            .ok()
            .map(|result| result.expect("deserialize_command error"))
    );
    assert_eq!(
        None,
        page::NavigateCommand::deserialize_command("Foo.bar", &mut deserializer)
            .ok()
            .map(|result| result.expect("deserialize_command error"))
    );
}

#[test]
fn test_command_page_enable() {
    let json = "{}";
    let rust = page::EnableCommand;
    do_test_json(json, &rust);
}

#[test]
fn test_command_parse_page_enable_extra_field() {
    let json = r#"{"foo":"bar"}"#;
    let rust = page::EnableCommand;
    do_test_json_de(json, &rust);
}

#[test]
fn test_command_page_enable_cdp_command_impl() {
    assert_eq!("Page.enable", page::EnableCommand::COMMAND_NAME);
}

#[test]
fn test_command_page_enable_serialize_cdp_command_impl() {
    let json = br#"{}"#;
    let rust = page::EnableCommand;

    assert_eq!("Page.enable", rust.command_name());

    let mut serializer = serde_json::Serializer::new(Vec::new());
    rust.serialize_command_params(&mut serializer).expect("serialize_command_params error");
    assert_eq!(&json[..], serializer.into_inner().as_slice());
}

#[test]
fn test_command_page_enable_deserialize_cdp_command_impl() {
    let json = r#"{}"#;
    let rust = page::EnableCommand;

    let mut deserializer = serde_json::Deserializer::from_str(json);
    assert_eq!(
        Some(rust),
        page::EnableCommand::deserialize_command("Page.enable", &mut deserializer)
            .ok()
            .map(|result| result.expect("deserialize_command error"))
    );
    assert_eq!(
        None,
        page::EnableCommand::deserialize_command("Foo.bar", &mut deserializer)
            .ok()
            .map(|result| result.expect("deserialize_command error"))
    );
}

#[test]
fn test_response_page_navigate() {
    let json = r#"{"frameId":"hello"}"#;
    let rust = page::NavigateResponse {
        frame_id: "hello".into(),
    };
    do_test_json(json, &rust);
}

#[test]
fn test_response_parse_page_navigate_extra_field() {
    let json = r#"{"frameId":"hello","foo":"bar"}"#;
    let rust = page::NavigateResponse {
        frame_id: "hello".into(),
    };
    do_test_json_de(json, &rust);
}

#[test]
fn test_event_page_dom_content_event_fired() {
    let json = r#"{"timestamp":12.7}"#;
    let rust = page::DomContentEventFiredEvent { timestamp: 12.7 };
    do_test_json(json, &rust);
}

#[test]
fn test_event_parse_page_dom_content_event_fired_extra_field() {
    let json = r#"{"timestamp":12.7,"foo":"bar"}"#;
    let rust = page::DomContentEventFiredEvent { timestamp: 12.7 };
    do_test_json_de(json, &rust);
}

#[test]
fn test_event_page_dom_content_event_fired_cdp_event_impl() {
    assert_eq!("Page.domContentEventFired", page::DomContentEventFiredEvent::EVENT_NAME);
}

#[test]
fn test_event_page_dom_content_event_fired_serialize_cdp_event_impl() {
    let json = br#"{"timestamp":12.7}"#;
    let rust = page::DomContentEventFiredEvent { timestamp: 12.7 };

    assert_eq!("Page.domContentEventFired", rust.event_name());

    let mut serializer = serde_json::Serializer::new(Vec::new());
    rust.serialize_event_params(&mut serializer).expect("serialize_event_params error");
    assert_eq!(&json[..], serializer.into_inner().as_slice());
}

#[test]
fn test_event_page_dom_content_event_fired_deserialize_cdp_event_impl() {
    let json = r#"{"timestamp":12.7}"#;
    let rust = page::DomContentEventFiredEvent { timestamp: 12.7 };

    let mut deserializer = serde_json::Deserializer::from_str(json);
    assert_eq!(
        Some(rust),
        page::DomContentEventFiredEvent::deserialize_event(
            "Page.domContentEventFired",
            &mut deserializer
        ).ok()
            .map(|result| result.expect("deserialize_event error"))
    );
    assert_eq!(
        None,
        page::DomContentEventFiredEvent::deserialize_event("Foo.bar", &mut deserializer)
            .ok()
            .map(|result| result.expect("deserialize_event error"))
    );
}

#[test]
fn test_incoming_page_navigate() {
    let json = r#"{"id":1,"method":"Page.navigate","params":{"url":"https://www.mozilla.org"}}"#;

    let command = page::NavigateCommand {
        url: "https://www.mozilla.org".into(),
        referrer: None,
        transition_type: None,
    };

    {
        let mut serialized = String::new();
        CdpIncoming::serialize_command_to_string(&mut serialized, 1, &command)
            .expect("serialize error");
        assert_eq!(json, &serialized);
    }

    let command_value = serde_json::to_value(&command).expect("to_value error");
    assert_eq!(
        command_value,
        Value::Object(
            CdpIncoming::parse_from_str(json).expect("parse error").command_params.into()
        )
    );
    assert_eq!(command, serde_json::from_value(command_value.clone()).expect("from_value error"));

    let rust = CdpIncoming {
        id: 1,
        command_name: page::NavigateCommand::COMMAND_NAME.into(),
        command_params: match command_value {
            Value::Object(params) => params.into(),
            _ => unreachable!("to_value produced a non-object"),
        },
    };
    do_test_json(json, &rust);
}

#[test]
fn test_parse_incoming_invalid() {
    let json = "hello";
    assert_eq!(CdpIncoming::parse_from_str(json), Err((CdpError::invalid_message(), None)));
}

#[test]
fn test_parse_incoming_not_object() {
    let json = "1";
    assert_eq!(CdpIncoming::parse_from_str(json), Err((CdpError::must_be_object(), None)));
}

#[test]
fn test_parse_incoming_missing_id() {
    let json = "{}";
    assert_eq!(CdpIncoming::parse_from_str(json), Err((CdpError::must_have_id(), None)));
}

#[test]
fn test_parse_incoming_non_integer_id() {
    let json = r#"{"id":"0"}"#;
    assert_eq!(CdpIncoming::parse_from_str(json), Err((CdpError::must_have_id(), None)));
}

#[test]
fn test_parse_incoming_missing_method() {
    let json = r#"{"id":0}"#;
    assert_eq!(CdpIncoming::parse_from_str(json), Err((CdpError::must_have_method(), Some(0))));
}

#[test]
fn test_parse_incoming_non_object_params() {
    let json = r#"{"id":0,"method":"Page.enable","params":7}"#;
    assert_eq!(
        CdpIncoming::parse_from_str(json),
        Ok(CdpIncoming {
            id: 0,
            command_name: "Page.enable".into(),
            command_params: Default::default(),
        })
    );
}

#[test]
fn test_parse_incoming_no_params_field() {
    let json = r#"{"id":0,"method":"Page.enable"}"#;
    let rust = CdpIncoming {
        id: 0,
        command_name: "Page.enable".into(),
        command_params: Default::default(),
    };
    assert_eq!(CdpIncoming::parse_from_str(json), Ok(rust));
}

#[test]
fn test_parse_incoming_empty_params_object() {
    let json = r#"{"id":0,"method":"Page.enable","params":{}}"#;
    let rust = CdpIncoming {
        id: 0,
        command_name: "Page.enable".into(),
        command_params: Default::default(),
    };
    assert_eq!(CdpIncoming::parse_from_str(json), Ok(rust));
}

#[test]
fn test_parse_incoming_incoming_extra_field() {
    let json = r#"{"id":0,"method":"Page.enable","params":{},"foo":"bar"}"#;
    let rust = CdpIncoming {
        id: 0,
        command_name: "Page.enable".into(),
        command_params: Default::default(),
    };
    assert_eq!(CdpIncoming::parse_from_str(json), Ok(rust));
}

#[test]
fn test_outgoing_success_result() {
    let json = r#"{"id":1,"result":{"frameId":"0"}}"#;

    let response = page::NavigateResponse {
        frame_id: "0".into(),
    };

    {
        let mut serialized = String::new();
        CdpOutgoing::serialize_response_to_string(&mut serialized, 1, &response)
            .expect("serialize error");
        assert_eq!(json, &serialized);
    }

    {
        let mut serialized = String::new();
        CdpOutgoing::serialize_result_to_string(&mut serialized, 1, Ok(&response))
            .expect("serialize error");
        assert_eq!(json, &serialized);
    }

    let response_value = serde_json::to_value(&response).expect("to_value error");
    assert_eq!(
        response,
        serde_json::from_value(response_value.clone()).expect("from_value error")
    );

    let rust = CdpOutgoing::Result {
        id: 1,
        result: Ok(match response_value {
            Value::Object(params) => params,
            _ => unreachable!("to_value produced a non-object"),
        }),
    };
    do_test_json(json, &rust);
}

#[test]
fn test_outgoing_error_result() {
    let json = r#"{"id":1,"error":{"code":-32601,"message":"'Foo.bar' wasn't found"}}"#;

    let rust = CdpOutgoing::Result {
        id: 1,
        result: Err(CdpError::method_not_found("Foo.bar")),
    };

    do_test_json_de(json, &rust);

    let error = CdpError::method_not_found("Foo.bar");

    {
        let mut serialized = String::new();
        CdpOutgoing::serialize_error_to_string(&mut serialized, Some(1), &error)
            .expect("serialize error");
        assert_eq!(json, &serialized);
    }

    {
        let mut serialized = String::new();
        CdpOutgoing::serialize_result_to_string(
            &mut serialized,
            1,
            Err(&error) as Result<&page::NavigateResponse, _>,
        ).expect("serialize error");
        assert_eq!(json, &serialized);
    }
}

#[test]
fn test_outgoing_event() {
    let json = r#"{"method":"Page.domContentEventFired","params":{"timestamp":12.7}}"#;

    let event = page::DomContentEventFiredEvent { timestamp: 12.7 };

    let mut serialized = String::new();
    CdpOutgoing::serialize_event_to_string(&mut serialized, &event).expect("serialize error");
    assert_eq!(json, &serialized);

    let event_value = serde_json::to_value(&event).expect("to_value error");
    assert_eq!(event, serde_json::from_value(event_value.clone()).expect("from_value error"));

    let rust = CdpOutgoing::Event {
        name: "Page.domContentEventFired".into(),
        params: match event_value {
            Value::Object(params) => params,
            _ => unreachable!("to_value produced a non-object"),
        },
    };
    do_test_json(json, &rust);
}

#[test]
fn test_outgoing_error() {
    let json = r#"{"error":{"code":-32700,"message":"Message must be a valid JSON"}}"#;

    let rust = CdpOutgoing::Error(CdpError::invalid_message());

    do_test_json_de(json, &rust);

    let error = CdpError::invalid_message();

    let mut serialized = String::new();
    CdpOutgoing::serialize_error_to_string(&mut serialized, None, &error)
        .expect("serialize error");
    assert_eq!(json, &serialized);
}

pub fn do_test_json<T>(json: &str, rust: &T)
where
    for<'de> T: Deserialize<'de> + Serialize + PartialEq + Debug,
{
    do_test_json_ser(json, rust);
    do_test_json_de(json, rust);
}

pub fn do_test_json_ser<T>(json: &str, rust: &T)
where
    T: Serialize + PartialEq + Debug,
{
    let ser = serde_json::to_string(rust).expect("serialize error");
    assert_eq!(json, ser, "json serialize mismatch\n\nexpected:\n{}\n\nactual:\n{}", json, ser);
}

pub fn do_test_json_de<T>(json: &str, rust: &T)
where
    for<'de> T: Deserialize<'de> + PartialEq + Debug,
{
    let de: T = serde_json::from_str(json).expect("deserialize error");
    assert_eq!(
        rust,
        &de,
        "json deserialize mismatch\n\nexpected:\n{:#?}\n\nactual:\n{:#?}",
        rust,
        &de
    );
}
