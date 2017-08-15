extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate cdp;

use std::fmt::Debug;
use serde::{Deserialize, Serialize};

use cdp::http::{DevToolsUrls, HttpCommand, HttpResponse, OwnedHttpCommand, Page, PageType,
                VersionInfo};

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

    do_test_json(json, &rust);
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

    do_test_json(json, &rust);
}

fn do_test_json<T>(json: &str, rust: &T)
    where for<'de> T: Deserialize<'de> + Serialize + Eq + Debug
{
    do_test_json_ser(json, rust);
    do_test_json_de(json, rust);
}

fn do_test_json_ser<T>(json: &str, rust: &T)
    where T: Serialize + Eq + Debug
{
    let ser = serde_json::to_string_pretty(rust).expect("serialize error");
    assert_eq!(json,
               ser,
               "json serialize mismatch\n\nexpected:\n{}\n\nactual:\n{}",
               json,
               ser);
}

fn do_test_json_de<T>(json: &str, rust: &T)
    where for<'de> T: Deserialize<'de> + Eq + Debug
{
    let de: T = serde_json::from_str(json).expect("deserialize error");
    assert_eq!(rust,
               &de,
               "json deserialize mismatch\n\nexpected:\n{:#?}\n\nactual:\n{:#?}",
               rust,
               &de);
}

#[test]
fn test_frontend_url_format() {
    let expected_frontend_url = "chrome-devtools://devtools/bundled/inspector.html?ws=localhost:\
                                 9222/devtools/page/0";
    let actual_frontend_url = format!(cdp_frontend_url_format!(),
                                      server_addr = "localhost:9222",
                                      page_id = "0");
    assert_eq!(expected_frontend_url, actual_frontend_url);
}

#[test]
fn test_version_info_url_format() {
    let expected_version_info_url = "http://localhost:9222/json/version";
    let actual_version_info_url = format!(cdp_http_version_info_url_format!(),
                                          server_addr = "localhost:9222");
    assert_eq!(expected_version_info_url, actual_version_info_url);
}

#[test]
fn test_page_list_url_format() {
    let expected_page_list_url = "http://localhost:9222/json/list";
    let actual_page_list_url = format!(cdp_http_page_list_url_format!(),
                                       server_addr = "localhost:9222");
    assert_eq!(expected_page_list_url, actual_page_list_url);
}

#[test]
fn test_new_page_url_format() {
    let expected_new_page_url = "http://localhost:9222/json/new";
    let actual_new_page_url = format!(cdp_http_new_page_url_format!(),
                                      server_addr = "localhost:9222");
    assert_eq!(expected_new_page_url, actual_new_page_url);
}

#[test]
fn test_new_page_and_nav_url_format() {
    let expected_new_page_and_nav_url = "http://localhost:9222/json/new?https://www.mozilla.org";
    let actual_new_page_and_nav_url = format!(cdp_http_new_page_and_navigate_url_format!(),
                                              server_addr = "localhost:9222",
                                              url = "https://www.mozilla.org");
    assert_eq!(expected_new_page_and_nav_url, actual_new_page_and_nav_url);
}

#[test]
fn test_activate_page_url_format() {
    let expected_activate_page_url = "http://localhost:9222/json/activate/0";
    let actual_activate_page_url = format!(cdp_http_activate_page_url_format!(),
                                           server_addr = "localhost:9222",
                                           page_id = "0");
    assert_eq!(expected_activate_page_url, actual_activate_page_url);
}

#[test]
fn test_http_command_unknown() {
    assert!(HttpCommand::parse("foo", None).is_none());
    assert!(HttpCommand::parse("json/", None).is_none());
    assert!(HttpCommand::parse("json/foo", None).is_none());

    assert!(HttpCommand::parse_with_slash("/foo", None).is_none());
    assert!(HttpCommand::parse_with_slash("/json/", None).is_none());
    assert!(HttpCommand::parse_with_slash("/json/foo", None).is_none());

    assert!(OwnedHttpCommand::parse("foo", None).is_none());
    assert!(OwnedHttpCommand::parse("json/", None).is_none());
    assert!(OwnedHttpCommand::parse("json/foo", None).is_none());

    assert!(OwnedHttpCommand::parse_with_slash("/foo", None).is_none());
    assert!(OwnedHttpCommand::parse_with_slash("/json/", None).is_none());
    assert!(OwnedHttpCommand::parse_with_slash("/json/foo", None).is_none());
}

#[test]
fn test_http_command_missing_slash() {
    assert!(HttpCommand::parse_with_slash("", None).is_none());
    assert!(HttpCommand::parse_with_slash("foo", None).is_none());

    assert!(OwnedHttpCommand::parse_with_slash("", None).is_none());
    assert!(OwnedHttpCommand::parse_with_slash("foo", None).is_none());
}

#[test]
fn test_http_command_version_info() {
    do_test_http_command("json/version", None, OwnedHttpCommand::VersionInfo);
}

#[test]
fn test_http_command_page_list() {
    do_test_http_command("json/list", None, OwnedHttpCommand::PageList);
    do_test_http_command("json", None, OwnedHttpCommand::PageList);
}

#[test]
fn test_http_command_new_page() {
    do_test_http_command("json/new", None, OwnedHttpCommand::NewPage(None));
    do_test_http_command("json/new",
                         Some("https://www.mozilla.org"),
                         OwnedHttpCommand::NewPage(Some("https://www.mozilla.org".into())));
}

#[test]
fn test_http_command_activate_page() {
    do_test_http_command("json/activate/0",
                         None,
                         OwnedHttpCommand::ActivatePage("0".into()));
    do_test_http_command("json/activate/0/",
                         None,
                         OwnedHttpCommand::ActivatePage("0/".into()));
    do_test_http_command("json/activate/0/foo",
                         None,
                         OwnedHttpCommand::ActivatePage("0/foo".into()));
}

fn do_test_http_command(path: &str, query: Option<&str>, owned_command: OwnedHttpCommand) {
    let borrowed_command = HttpCommand::from(&owned_command);
    assert_eq!(OwnedHttpCommand::from(&borrowed_command), owned_command);

    let borrowed_parsed_command = HttpCommand::parse(path, query)
        .expect("borrowed command parse error");

    let path_with_slash = format!("/{}", path);
    let borrowed_parsed_command_with_slash =
        HttpCommand::parse_with_slash(path_with_slash.as_str(), query)
            .expect("borrowed command parse with slash error");

    assert_eq!(borrowed_command, borrowed_parsed_command);
    assert_eq!(borrowed_command, borrowed_parsed_command_with_slash);
    assert_eq!(borrowed_parsed_command, borrowed_parsed_command_with_slash);

    let owned_parsed_command = OwnedHttpCommand::parse(path, query)
        .expect("owned command parse error");
    let owned_parsed_command_with_slash =
        OwnedHttpCommand::parse_with_slash(format!("/{}", path).as_str(), query)
            .expect("owned command parse with slash error");

    assert_eq!(owned_command, owned_parsed_command);
    assert_eq!(owned_command, owned_parsed_command_with_slash);
    assert_eq!(owned_parsed_command, owned_parsed_command_with_slash);
}

#[test]
fn test_http_response_version_info() {
    let json = r#"{
  "Browser": "Mozzarella Foxfire",
  "Protocol-Version": "0.0",
  "User-Agent": "Mozzarella/5.0 Gouda/20100101 Foxfire/57.0",
  "V8-Version": "0",
  "WebKit-Version": "0"
}"#;

    let rust = HttpResponse::VersionInfo(VersionInfo {
        browser: "Mozzarella Foxfire".into(),
        protocol_version: "0.0".into(),
        user_agent: "Mozzarella/5.0 Gouda/20100101 Foxfire/57.0".into(),
        v8_version: None,
        webkit_version: None,
    });

    assert_eq!(rust.status(), 200);
    do_test_json_ser(json, &rust);
}

#[test]
fn test_http_response_page_list() {
    let json = r#"[
  {
    "description": "",
    "devtoolsFrontendUrl": "chrome-devtools://devtools/bundled/inspector.html?ws=127.0.0.1:9222/devtools/page/0",
    "faviconUrl": "https://www.mozilla.org/favicon.ico",
    "id": "0",
    "title": "Mozilla",
    "type": "page",
    "url": "https://www.mozilla.org",
    "webSocketDebuggerUrl": "ws://127.0.0.1:9222/devtools/page/0"
  }
]"#;

    let rust =
        HttpResponse::PageList(vec![Page {
                                        id: "0".into(),
                                        ty: PageType::Tab,
                                        url: "https://www.mozilla.org".into(),
                                        title: "Mozilla".into(),
                                        description: None,
                                        favicon_url: Some("https://www.mozilla.org/favicon.ico"
                                            .into()),
                                        devtools_urls: Some(DevToolsUrls::new(&"127.0.0.1:9222"
                                                                                  .parse()
                                                                                  .unwrap(),
                                                                              "0")),
                                    }]);

    assert_eq!(rust.status(), 200);
    do_test_json_ser(json, &rust);
}

#[test]
fn test_http_response_new_page() {
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

    let rust = HttpResponse::NewPage(Page {
        id: "0".into(),
        ty: PageType::Tab,
        url: "https://www.mozilla.org".into(),
        title: "Mozilla".into(),
        description: None,
        favicon_url: Some("https://www.mozilla.org/favicon.ico".into()),
        devtools_urls: Some(DevToolsUrls::new(&"127.0.0.1:9222".parse().unwrap(), "0")),
    });

    assert_eq!(rust.status(), 200);
    do_test_json_ser(json, &rust);
}

#[test]
fn test_http_response_activate_page() {
    assert_eq!(HttpResponse::ActivatePage(true).status(), 200);
    assert_eq!(HttpResponse::ActivatePage(false).status(), 404);
}
