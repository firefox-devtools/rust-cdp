extern crate serde;
extern crate serde_json;

extern crate cdp;

use serde::Deserialize;

use cdp::websocket::{page, Command, OwnedClientMessage, OwnedServerMessage};

#[test]
fn test_serialize_client_message() {
    let message = OwnedClientMessage {
        id: 1,
        command: Command::PageNavigate(page::NavigateParams {
            url: "https://www.mozilla.org".into(),
            referrer: None,
            transition_type: None,
        }),
    };
    println!("{}", serde_json::to_string(&message).unwrap());
}

#[test]
fn test_deserialize_client_message() {
    let json = r#"
        {
            "id": 1,
            "method": "Page.navigate",
            "params": {
                "url": "https://www.mozilla.org"
            }
        }
    "#;
    let message: OwnedClientMessage = serde_json::from_str(json).unwrap();
    println!("{:?}", message);
}

#[test]
fn test_deserialize_server_message_command_success() {
    let json = r#"
        {
            "id": 1,
            "result": {
                "frameId": "HelloWorld"
            }
        }
    "#;
    let message: OwnedServerMessage<page::NavigateResponse> = serde_json::from_str(json).unwrap();
    println!("{:?}", message);
}
