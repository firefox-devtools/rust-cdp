// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate cdp;

mod helper;

use serde_json::{Map, Value};

use cdp::ws::{page, ClientMessage, Command, Empty, Event, OwnedClientMessage, OwnedServerMessage,
              ServerError, ServerMessage};

#[test]
fn test_ws_url_format() {
    let expected_ws_url = "ws://localhost:9222/devtools/page/0";
    let actual_ws_url = format!(cdp_ws_url_format!(),
                                server_addr = "localhost:9222",
                                page_id = "0");
    assert_eq!(expected_ws_url, actual_ws_url);
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
    assert_eq!(cdp::ws::parse_path_with_slash("/devtools/page/0"),
               Some("0"));
    assert_eq!(cdp::ws::parse_path_with_slash("/devtools/page/0/"),
               Some("0/"));
    assert_eq!(cdp::ws::parse_path_with_slash("/devtools/page/0/1"),
               Some("0/1"));

    assert_eq!(cdp::ws::parse_path_with_slash(""), None);
    assert_eq!(cdp::ws::parse_path_with_slash("foo"), None);
    assert_eq!(cdp::ws::parse_path_with_slash("devtools/page"), None);
    assert_eq!(cdp::ws::parse_path_with_slash("devtools/page/"), None);
    assert_eq!(cdp::ws::parse_path_with_slash("devtools/page/0"), None);
    assert_eq!(cdp::ws::parse_path_with_slash("devtools/page/0/"), None);
    assert_eq!(cdp::ws::parse_path_with_slash("devtools/page/0/1"), None);
}

#[test]
fn test_empty() {
    let json = "{}";
    let rust = Empty;
    helper::do_test_json(json, &rust);
}

#[test]
fn test_client_message_page_navigate() {
    let json = r#"{
  "id": 1,
  "method": "Page.navigate",
  "params": {
    "url": "https://www.mozilla.org"
  }
}"#;

    let rust = OwnedClientMessage {
        id: 1,
        command: Command::PageNavigate(page::NavigateParams {
            url: "https://www.mozilla.org".into(),
            referrer: None,
            transition_type: None,
        }),
    };

    helper::do_test_json(json, &rust);
}

#[test]
fn test_parse_incoming_client_message_invalid() {
    let json = "hello";
    assert_eq!(OwnedClientMessage::parse_incoming(&mut serde_json::Deserializer::from_str(json)),
               Err((ServerError::invalid_json(), None)));
}

#[test]
fn test_parse_incoming_client_message_not_object() {
    let json = "1";
    assert_eq!(OwnedClientMessage::parse_incoming(&mut serde_json::Deserializer::from_str(json)),
               Err((ServerError::must_be_object(), None)));
}

#[test]
fn test_parse_incoming_client_message_missing_id() {
    let json = "{}";
    assert_eq!(OwnedClientMessage::parse_incoming(&mut serde_json::Deserializer::from_str(json)),
               Err((ServerError::must_have_id(), None)));
}

#[test]
fn test_parse_incoming_client_message_non_integer_id() {
    let json = r#"{"id":"0"}"#;
    assert_eq!(OwnedClientMessage::parse_incoming(&mut serde_json::Deserializer::from_str(json)),
               Err((ServerError::must_have_id(), None)));
}

#[test]
fn test_parse_incoming_client_message_missing_method() {
    let json = r#"{"id":0}"#;
    assert_eq!(OwnedClientMessage::parse_incoming(&mut serde_json::Deserializer::from_str(json)),
               Err((ServerError::must_have_method(), Some(0))));
}

#[test]
fn test_parse_incoming_client_message_method_not_found() {
    let json = r#"{"id":0,"method":"Foo.bar"}"#;
    assert_eq!(OwnedClientMessage::parse_incoming(&mut serde_json::Deserializer::from_str(json)),
               Err((ServerError::method_not_found("Foo.bar"), Some(0))));
}

#[test]
fn test_parse_incoming_client_message_missing_params_field() {
    let json = r#"{"id":0,"method":"Page.navigate"}"#;
    assert_eq!(OwnedClientMessage::parse_incoming(&mut serde_json::Deserializer::from_str(json)),
               Err((ServerError::invalid_parameters("missing field `url`"), Some(0))));
}

#[test]
fn test_parse_incoming_client_message_non_object_params() {
    let json = r#"{"id":0,"method":"Page.navigate","params":7}"#;
    assert_eq!(OwnedClientMessage::parse_incoming(&mut serde_json::Deserializer::from_str(json)),
               Err((ServerError::invalid_parameters("invalid type: integer `7`, expected \
                                                       struct NavigateParams"),
                    Some(0))));
}

#[test]
fn test_parse_incoming_client_message_missing_params_content() {
    let json = r#"{"id":0,"method":"Page.navigate","params":{}}"#;
    assert_eq!(OwnedClientMessage::parse_incoming(&mut serde_json::Deserializer::from_str(json)),
               Err((ServerError::invalid_parameters("missing field `url`"), Some(0))));
}

#[test]
fn test_parse_incoming_client_message_page_navigate() {
    let json = r#"{"id":0,"method":"Page.navigate","params":{"url":"https://www.mozilla.org"}}"#;
    let rust = OwnedClientMessage {
        id: 0,
        command: Command::PageNavigate(page::NavigateParams {
            url: "https://www.mozilla.org".into(),
            referrer: None,
            transition_type: None,
        }),
    };
    assert_eq!(OwnedClientMessage::parse_incoming(&mut serde_json::Deserializer::from_str(json)),
               Ok(rust));
}

#[test]
fn test_parse_incoming_client_message_page_enable_no_params_field() {
    let json = r#"{"id":0,"method":"Page.enable"}"#;
    let rust = OwnedClientMessage {
        id: 0,
        command: Command::PageEnable,
    };
    assert_eq!(OwnedClientMessage::parse_incoming(&mut serde_json::Deserializer::from_str(json)),
               Ok(rust));
}

#[test]
fn test_parse_incoming_client_message_page_enable_empty_params_object() {
    let json = r#"{"id":0,"method":"Page.enable","params":{}}"#;
    let rust = OwnedClientMessage {
        id: 0,
        command: Command::PageEnable,
    };
    assert_eq!(OwnedClientMessage::parse_incoming(&mut serde_json::Deserializer::from_str(json)),
               Ok(rust));
}

#[test]
fn test_parse_incoming_client_message_page_enable_extra_field() {
    let json = r#"{"id":0,"method":"Page.enable","params":{},"foo":"bar"}"#;
    let rust = OwnedClientMessage {
        id: 0,
        command: Command::PageEnable,
    };
    assert_eq!(OwnedClientMessage::parse_incoming(&mut serde_json::Deserializer::from_str(json)),
               Ok(rust));
}

#[test]
fn test_parse_incoming_client_message_page_enable_extra_params_field() {
    let json = r#"{"id":0,"method":"Page.enable","params":{"foo":"bar"}}"#;
    let rust = OwnedClientMessage {
        id: 0,
        command: Command::PageEnable,
    };
    assert_eq!(OwnedClientMessage::parse_incoming(&mut serde_json::Deserializer::from_str(json)),
               Ok(rust));
}

#[test]
fn test_borrowed_client_message() {
    let owned = OwnedClientMessage {
        id: 0,
        command: Command::PageEnable,
    };
    let borrowed = ClientMessage::from(&owned);
    assert_eq!(owned, OwnedClientMessage::from(&borrowed));
    assert_eq!(owned, OwnedClientMessage::from(borrowed));
}

#[test]
fn test_server_message_value_response() {
    let json = r#"{
  "id": 1,
  "result": {
    "frameId": "0"
  }
}"#;

    let mut map = Map::with_capacity(1);
    map.insert("frameId".into(), Value::String("0".into()));

    let rust = OwnedServerMessage::Response {
        id: 1,
        result: Ok(Value::Object(map)),
    };

    helper::do_test_json(json, &rust);

    let borrowed = ServerMessage::from(&rust);
    assert_eq!(rust, OwnedServerMessage::from(&borrowed));
    assert_eq!(rust, OwnedServerMessage::from(borrowed));
}

#[test]
fn test_server_message_specific_response() {
    let json = r#"{
  "id": 1,
  "result": {
    "frameId": "0"
  }
}"#;

    let rust = OwnedServerMessage::Response {
        id: 1,
        result: Ok(page::NavigateResponse { frame_id: "0".into() }),
    };

    helper::do_test_json(json, &rust);

    let borrowed = ServerMessage::from(&rust);
    assert_eq!(rust, OwnedServerMessage::from(&borrowed));
    assert_eq!(rust, OwnedServerMessage::from(borrowed));
}

#[test]
fn test_server_message_error_response() {
    let json = r#"{
  "id": 1,
  "error": {
    "code": -32601,
    "message": "'Foo.bar' wasn't found"
  }
}"#;

    let rust: OwnedServerMessage = OwnedServerMessage::Response {
        id: 1,
        result: Err(ServerError::method_not_found("Foo.bar")),
    };

    helper::do_test_json(json, &rust);

    let borrowed = ServerMessage::from(&rust);
    assert_eq!(rust, OwnedServerMessage::from(&borrowed));
    assert_eq!(rust, OwnedServerMessage::from(borrowed));
}

#[test]
fn test_server_message_event() {
    let json = r#"{
  "method": "Page.domContentEventFired",
  "params": {
    "timestamp": 12.7
  }
}"#;

    let rust: OwnedServerMessage =
        OwnedServerMessage::Event(Event::PageDomContentEventFired { timestamp: 12.7 });

    helper::do_test_json(json, &rust);

    let borrowed = ServerMessage::from(&rust);
    assert_eq!(rust, OwnedServerMessage::from(&borrowed));
    assert_eq!(rust, OwnedServerMessage::from(borrowed));
}

#[test]
fn test_server_message_error() {
    let json = r#"{
  "error": {
    "code": -32700,
    "message": "Message must be a valid JSON"
  }
}"#;

    let rust: OwnedServerMessage = OwnedServerMessage::Error(ServerError::invalid_json());

    helper::do_test_json(json, &rust);

    let borrowed = ServerMessage::from(&rust);
    assert_eq!(rust, OwnedServerMessage::from(&borrowed));
    assert_eq!(rust, OwnedServerMessage::from(borrowed));
}
