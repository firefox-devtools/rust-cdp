// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

#![cfg_attr(feature = "strict", deny(warnings))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

extern crate serde;

extern crate cdp;

mod helper;

use cdp::greeter::{DevToolsUrls, GreeterActivatePageResponse, GreeterErrorResponse,
                   GreeterNewPageResponse, GreeterPageListResponse, GreeterRequest,
                   GreeterVersionInfoResponse, OwnedGreeterRequest, Page, PageType,
                   SerializeGreeterResponse, VersionInfo};

#[test]
fn test_json_version_info() {
    let json = r#"{
  "Browser": "Mozzarella Foxfire",
  "Protocol-Version": "0.0",
  "User-Agent": "Mozzarella/5.0 Gouda/20100101 Foxfire/57.0",
  "V8-Version": "0",
  "WebKit-Version": "0"
}"#;

    let rust = VersionInfo {
        browser: "Mozzarella Foxfire".into(),
        protocol_version: "0.0".into(),
        user_agent: "Mozzarella/5.0 Gouda/20100101 Foxfire/57.0".into(),
        v8_version: None,
        webkit_version: None,
    };

    helper::do_test_json_pretty(json, &rust);
}

#[test]
fn test_json_page() {
    let json = r#"{
  "description": "",
  "devtoolsFrontendUrl": "chrome-devtools://devtools/bundled/inspector.html?ws=127.0.0.1:9222/devtools/page/0",
  "faviconUrl": "https://www.mozilla.org/favicon.ico",
  "id": "0",
  "title": "Mozilla",
  "type": "page",
  "url": "https://www.mozilla.org",
  "webSocketDebuggerUrl": "ws://127.0.0.1:9222/devtools/page/0"
}"#;

    let rust = Page {
        id: "0".into(),
        ty: PageType::Tab,
        url: "https://www.mozilla.org".into(),
        title: "Mozilla".into(),
        description: None,
        favicon_url: Some("https://www.mozilla.org/favicon.ico".into()),
        devtools_urls: Some(DevToolsUrls::new(&"127.0.0.1:9222".parse().unwrap(), "0")),
    };

    helper::do_test_json_pretty(json, &rust);
}

#[test]
fn test_command_unknown() {
    assert!(GreeterRequest::parse("foo", None).is_none());
    assert!(GreeterRequest::parse("json/", None).is_none());
    assert!(GreeterRequest::parse("json/foo", None).is_none());

    assert!(GreeterRequest::parse_with_slash("/foo", None).is_none());
    assert!(GreeterRequest::parse_with_slash("/json/", None).is_none());
    assert!(GreeterRequest::parse_with_slash("/json/foo", None).is_none());

    assert!(OwnedGreeterRequest::parse("foo", None).is_none());
    assert!(OwnedGreeterRequest::parse("json/", None).is_none());
    assert!(OwnedGreeterRequest::parse("json/foo", None).is_none());

    assert!(OwnedGreeterRequest::parse_with_slash("/foo", None).is_none());
    assert!(OwnedGreeterRequest::parse_with_slash("/json/", None).is_none());
    assert!(OwnedGreeterRequest::parse_with_slash("/json/foo", None).is_none());
}

#[test]
fn test_command_missing_slash() {
    assert!(GreeterRequest::parse_with_slash("", None).is_none());
    assert!(GreeterRequest::parse_with_slash("foo", None).is_none());

    assert!(OwnedGreeterRequest::parse_with_slash("", None).is_none());
    assert!(OwnedGreeterRequest::parse_with_slash("foo", None).is_none());
}

#[test]
fn test_command_version_info() {
    do_test_command("json/version", None, &OwnedGreeterRequest::VersionInfo);
}

#[test]
fn test_command_page_list() {
    do_test_command("json/list", None, &OwnedGreeterRequest::PageList);
    do_test_command("json", None, &OwnedGreeterRequest::PageList);
}

#[test]
fn test_command_new_page() {
    do_test_command("json/new", None, &OwnedGreeterRequest::NewPage(None));
    do_test_command(
        "json/new",
        Some("https://www.mozilla.org"),
        &OwnedGreeterRequest::NewPage(Some("https://www.mozilla.org".into())),
    );
}

#[test]
fn test_command_activate_page() {
    do_test_command("json/activate/0", None, &OwnedGreeterRequest::ActivatePage("0".into()));
    do_test_command("json/activate/0/", None, &OwnedGreeterRequest::ActivatePage("0/".into()));
    do_test_command(
        "json/activate/0/foo",
        None,
        &OwnedGreeterRequest::ActivatePage("0/foo".into()),
    );
}

fn do_test_command(path: &str, query: Option<&str>, owned_command: &OwnedGreeterRequest) {
    let borrowed_command = GreeterRequest::from(owned_command);
    assert_eq!(&OwnedGreeterRequest::from(&borrowed_command), owned_command);

    let borrowed_parsed_command =
        GreeterRequest::parse(path, query).expect("borrowed command parse error");

    let path_with_slash = format!("/{}", path);
    let borrowed_parsed_command_with_slash =
        GreeterRequest::parse_with_slash(path_with_slash.as_str(), query)
            .expect("borrowed command parse with slash error");

    assert_eq!(borrowed_command, borrowed_parsed_command);
    assert_eq!(borrowed_command, borrowed_parsed_command_with_slash);
    assert_eq!(borrowed_parsed_command, borrowed_parsed_command_with_slash);

    let owned_parsed_command =
        OwnedGreeterRequest::parse(path, query).expect("owned command parse error");
    let owned_parsed_command_with_slash =
        OwnedGreeterRequest::parse_with_slash(format!("/{}", path).as_str(), query)
            .expect("owned command parse with slash error");

    assert_eq!(owned_command, &owned_parsed_command);
    assert_eq!(owned_command, &owned_parsed_command_with_slash);
    assert_eq!(owned_parsed_command, owned_parsed_command_with_slash);
}

#[test]
fn test_response_version_info() {
    let json = "{\"Browser\":\"Mozzarella \
                Foxfire\",\"Protocol-Version\":\"0.0\",\"User-Agent\":\"Mozzarella/5.0 \
                Gouda/20100101 Foxfire/57.0\",\"V8-Version\":\"0\",\"WebKit-Version\":\"0\"}";
    let version_info = VersionInfo {
        browser: "Mozzarella Foxfire".into(),
        protocol_version: "0.0".into(),
        user_agent: "Mozzarella/5.0 Gouda/20100101 Foxfire/57.0".into(),
        v8_version: None,
        webkit_version: None,
    };
    let rust = GreeterVersionInfoResponse(&version_info);

    assert_eq!(rust.status(), 200);

    let mut string = String::new();
    rust.serialize_body_to_string(&mut string).expect("serialize_body_to_string error");
    assert_eq!(json, &string);
}

#[test]
fn test_response_page_list() {
    let json = "[{\"description\":\"\",\"devtoolsFrontendUrl\":\"chrome-devtools:\
                //devtools/bundled/inspector.html?ws=127.0.0.1:9222/devtools/page/0\",\
                \"faviconUrl\":\"https://www.mozilla.org/favicon.ico\",\"id\":\"0\",\"title\":\
                \"Mozilla\",\"type\":\"page\",\"url\":\"https://www.mozilla.org\",\
                \"webSocketDebuggerUrl\":\"ws://127.0.0.1:9222/devtools/page/0\"}]";

    let pages = vec![
        Page {
            id: "0".into(),
            ty: PageType::Tab,
            url: "https://www.mozilla.org".into(),
            title: "Mozilla".into(),
            description: None,
            favicon_url: Some("https://www.mozilla.org/favicon.ico".into()),
            devtools_urls: Some(DevToolsUrls::new(&"127.0.0.1:9222".parse().unwrap(), "0")),
        },
    ];
    let rust = GreeterPageListResponse(&pages);

    assert_eq!(rust.status(), 200);

    let mut string = String::new();
    rust.serialize_body_to_string(&mut string).expect("serialize_body_to_string error");
    assert_eq!(json, &string);
}

#[test]
fn test_response_new_page() {
    let json = "{\"description\":\"\",\"devtoolsFrontendUrl\":\"chrome-devtools:\
                //devtools/bundled/inspector.html?ws=127.0.0.1:9222/devtools/page/0\",\
                \"faviconUrl\":\"https://www.mozilla.org/favicon.ico\",\"id\":\"0\",\"title\":\
                \"Mozilla\",\"type\":\"page\",\"url\":\"https://www.mozilla.org\",\
                \"webSocketDebuggerUrl\":\"ws://127.0.0.1:9222/devtools/page/0\"}";
    let page = Page {
        id: "0".into(),
        ty: PageType::Tab,
        url: "https://www.mozilla.org".into(),
        title: "Mozilla".into(),
        description: None,
        favicon_url: Some("https://www.mozilla.org/favicon.ico".into()),
        devtools_urls: Some(DevToolsUrls::new(&"127.0.0.1:9222".parse().unwrap(), "0")),
    };
    let rust = GreeterNewPageResponse(&page);

    assert_eq!(rust.status(), 200);

    let mut string = String::new();
    rust.serialize_body_to_string(&mut string).expect("serialize_body_to_string error");
    assert_eq!(json, &string);
}

#[test]
fn test_response_activate_page_activated() {
    let rust = GreeterActivatePageResponse::Activated;

    assert_eq!(rust.status(), 200);

    let mut string = String::new();
    rust.serialize_body_to_string(&mut string).expect("serialize_body_to_string error");
    assert_eq!("", &string);
}

#[test]
fn test_response_activate_page_not_found() {
    let rust = GreeterActivatePageResponse::PageNotFound;

    assert_eq!(rust.status(), 404);

    let mut string = String::new();
    rust.serialize_body_to_string(&mut string).expect("serialize_body_to_string error");
    assert_eq!("", &string);
}

#[test]
fn test_response_error() {
    let rust = GreeterErrorResponse;

    assert_eq!(rust.status(), 500);

    let mut string = String::new();
    rust.serialize_body_to_string(&mut string).expect("serialize_body_to_string error");
    assert_eq!("", &string);
}
