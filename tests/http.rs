extern crate serde;
extern crate serde_json;

extern crate cdp;

use serde::Deserialize;

use cdp::http::{Page, PageType};

/*
#[test]
fn test_serialize_page() {
    let message = Page {
        id: "12345",
        ty: PageType::Tab,
        url: "https://www.mozilla.org".into(),
        title: "Internet for people, not profit - Mozilla",
        description: "",
        websocket_url: "ws://localhost:9222/devtools/page/914ef283-c2da-4e57-9d7c-7d5500a12ac4",
        frontend_url: "?ws=localhost:9222/devtools/page/914ef283-c2da-4e57-9d7c-7d5500a12ac4"
        id: 1,
        command: Command::PageNavigate {
            url: "https://www.mozilla.org".into(),
            referrer: None,
        },
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
    let message: ClientMessage = serde_json::from_str(json).unwrap();
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
    let message: ServerMessage = serde_json::from_str(json).unwrap();
    println!("{:?}", message);
    match message {
        ServerMessage::Response { result, .. } => {
            println!("{:?}",
                     page::NavigateResponse::deserialize(result.unwrap()).unwrap());
        }
        _ => panic!("message is not ServerMessage::Response"),
    }
}
*/
