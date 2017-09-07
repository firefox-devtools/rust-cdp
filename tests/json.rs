// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

#![cfg_attr(feature = "strict", deny(warnings))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

extern crate serde;
extern crate serde_json;

extern crate cdp;

mod helper;

use serde_json::{Map, Value};

use cdp::json::{JsonCdpError, JsonCdpIncoming, JsonCdpOutgoing};
use cdp::proto::page;

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
        JsonCdpIncoming::serialize_to_string(&mut serialized, 1, &command)
            .expect("serialize error");
        assert_eq!(json, &serialized);
    }

    let command_value = serde_json::to_value(&command).expect("to_value error");
    assert_eq!(
        command_value,
        Value::Object(JsonCdpIncoming::parse_from_str(json).expect("parse error").command_params)
    );
    assert_eq!(command, serde_json::from_value(command_value.clone()).expect("from_value error"));

    let rust = JsonCdpIncoming {
        id: 1,
        command_name: "Page.navigate".into(), // page::NavigateCommand::COMMAND_NAME.into(),
        command_params: match command_value {
            Value::Object(params) => params,
            _ => unreachable!("to_value produced a non-object"),
        },
    };
    helper::do_test_json(json, &rust);
}

#[test]
fn test_parse_incoming_invalid() {
    let json = "hello";
    assert_eq!(JsonCdpIncoming::parse_from_str(json), Err((JsonCdpError::invalid_json(), None)));
}

#[test]
fn test_parse_incoming_not_object() {
    let json = "1";
    assert_eq!(JsonCdpIncoming::parse_from_str(json), Err((JsonCdpError::must_be_object(), None)));
}

#[test]
fn test_parse_incoming_missing_id() {
    let json = "{}";
    assert_eq!(JsonCdpIncoming::parse_from_str(json), Err((JsonCdpError::must_have_id(), None)));
}

#[test]
fn test_parse_incoming_non_integer_id() {
    let json = r#"{"id":"0"}"#;
    assert_eq!(JsonCdpIncoming::parse_from_str(json), Err((JsonCdpError::must_have_id(), None)));
}

#[test]
fn test_parse_incoming_missing_method() {
    let json = r#"{"id":0}"#;
    assert_eq!(
        JsonCdpIncoming::parse_from_str(json),
        Err((JsonCdpError::must_have_method(), Some(0)))
    );
}

#[test]
fn test_parse_incoming_non_object_params() {
    let json = r#"{"id":0,"method":"Page.enable","params":7}"#;
    assert_eq!(
        JsonCdpIncoming::parse_from_str(json),
        Ok(JsonCdpIncoming {
            id: 0,
            command_name: "Page.enable".into(),
            command_params: Map::new(),
        })
    );
}

#[test]
fn test_parse_incoming_no_params_field() {
    let json = r#"{"id":0,"method":"Page.enable"}"#;
    let rust = JsonCdpIncoming {
        id: 0,
        command_name: "Page.enable".into(),
        command_params: Map::new(),
    };
    assert_eq!(JsonCdpIncoming::parse_from_str(json), Ok(rust));
}

#[test]
fn test_parse_incoming_empty_params_object() {
    let json = r#"{"id":0,"method":"Page.enable","params":{}}"#;
    let rust = JsonCdpIncoming {
        id: 0,
        command_name: "Page.enable".into(),
        command_params: Map::new(),
    };
    assert_eq!(JsonCdpIncoming::parse_from_str(json), Ok(rust));
}

#[test]
fn test_parse_incoming_incoming_extra_field() {
    let json = r#"{"id":0,"method":"Page.enable","params":{},"foo":"bar"}"#;
    let rust = JsonCdpIncoming {
        id: 0,
        command_name: "Page.enable".into(),
        command_params: Map::new(),
    };
    assert_eq!(JsonCdpIncoming::parse_from_str(json), Ok(rust));
}

#[test]
fn test_outgoing_success_result() {
    let json = r#"{"id":1,"result":{"frameId":"0"}}"#;

    let response = page::NavigateResponse {
        frame_id: "0".into(),
    };

    {
        let mut serialized = String::new();
        JsonCdpOutgoing::serialize_response_to_string(&mut serialized, 1, &response)
            .expect("serialize error");
        assert_eq!(json, &serialized);
    }

    {
        let mut serialized = String::new();
        JsonCdpOutgoing::serialize_result_to_string(&mut serialized, 1, Ok(&response))
            .expect("serialize error");
        assert_eq!(json, &serialized);
    }

    let response_value = serde_json::to_value(&response).expect("to_value error");
    assert_eq!(
        response,
        serde_json::from_value(response_value.clone()).expect("from_value error")
    );

    let rust = JsonCdpOutgoing::Result {
        id: 1,
        result: Ok(match response_value {
            Value::Object(params) => params,
            _ => unreachable!("to_value produced a non-object"),
        }),
    };
    helper::do_test_json(json, &rust);
}

#[test]
fn test_outgoing_error_result() {
    let json = r#"{"id":1,"error":{"code":-32601,"message":"'Foo.bar' wasn't found"}}"#;

    let rust = JsonCdpOutgoing::Result {
        id: 1,
        result: Err(JsonCdpError::method_not_found("Foo.bar")),
    };

    helper::do_test_json_de(json, &rust);

    let error = JsonCdpError::method_not_found("Foo.bar");

    {
        let mut serialized = String::new();
        JsonCdpOutgoing::serialize_error_to_string(&mut serialized, Some(1), &error)
            .expect("serialize error");
        assert_eq!(json, &serialized);
    }

    {
        let mut serialized = String::new();
        JsonCdpOutgoing::serialize_result_to_string(
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
    JsonCdpOutgoing::serialize_event_to_string(&mut serialized, &event).expect("serialize error");
    assert_eq!(json, &serialized);

    let event_value = serde_json::to_value(&event).expect("to_value error");
    assert_eq!(event, serde_json::from_value(event_value.clone()).expect("from_value error"));

    let rust = JsonCdpOutgoing::Event {
        name: "Page.domContentEventFired".into(),
        params: match event_value {
            Value::Object(params) => params,
            _ => unreachable!("to_value produced a non-object"),
        },
    };
    helper::do_test_json(json, &rust);
}

#[test]
fn test_outgoing_error() {
    let json = r#"{"error":{"code":-32700,"message":"Message must be a valid JSON"}}"#;

    let rust = JsonCdpOutgoing::Error(JsonCdpError::invalid_json());

    helper::do_test_json_de(json, &rust);

    let error = JsonCdpError::invalid_json();

    let mut serialized = String::new();
    JsonCdpOutgoing::serialize_error_to_string(&mut serialized, None, &error)
        .expect("serialize error");
    assert_eq!(json, &serialized);
}
