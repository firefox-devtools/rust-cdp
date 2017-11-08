// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;
use std::fmt;

pub mod client;
pub mod server;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WsEndpoint {
    Browser(Option<String>),
    Page(String),
}

impl WsEndpoint {
    pub fn browser_endpoint_url(server_addr: &str, maybe_id: Option<&str>) -> String {
        match maybe_id {
            Some(id) => format!(
                "ws://{server_addr}/devtools/browser/{id}",
                server_addr = server_addr,
                id = id
            ),
            None => format!("ws://{server_addr}/devtools/browser", server_addr = server_addr),
        }
    }

    pub fn page_endpoint_url(server_addr: &str, id: &str) -> String {
        format!("ws://{server_addr}/devtools/page/{id}", server_addr = server_addr, id = id)
    }

    pub fn url(&self, server_addr: &str) -> String {
        match *self {
            WsEndpoint::Browser(ref maybe_id) => WsEndpoint::browser_endpoint_url(
                server_addr,
                maybe_id.as_ref().map(|x| x.as_str()),
            ),
            WsEndpoint::Page(ref id) => WsEndpoint::page_endpoint_url(server_addr, id),
        }
    }
}

// https://chromium.googlesource.com/chromium/src/+/lkcr/chrome/test/chromedriver/chrome/devtools_http_client_unittest.cc#93
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum TargetKind {
    #[serde(rename = "page")]
    Page,
    #[serde(rename = "background_page")]
    BackgroundPage,
    #[serde(rename = "service_worker")]
    ServiceWorker,
    #[serde(rename = "app")]
    App,
    #[serde(rename = "iframe")]
    IFrame,
    #[serde(rename = "webview")]
    WebView,
    #[serde(rename = "worker")]
    Worker,
    #[serde(rename = "other")]
    Other,
}

impl fmt::Display for TargetKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                TargetKind::Page => "page",
                TargetKind::BackgroundPage => "background page",
                TargetKind::ServiceWorker => "service worker",
                TargetKind::App => "app",
                TargetKind::IFrame => "iframe",
                TargetKind::WebView => "webview",
                TargetKind::Worker => "worker",
                TargetKind::Other => "other",
            }
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Target<'a> {
    pub id: Cow<'a, str>,
    #[serde(rename = "type")]
    pub kind: TargetKind,
    pub url: Cow<'a, str>,
    pub title: Cow<'a, str>,
    pub description: Option<Cow<'a, str>>,
    #[serde(rename = "faviconUrl")]
    pub favicon_url: Option<Cow<'a, str>>,
    #[serde(rename = "webSocketDebuggerUrl")]
    pub ws_url: String,
    #[serde(rename = "devtoolsFrontendUrl")]
    pub frontend_url: Option<String>,
}

// https://chromium.googlesource.com/chromium/src/+/master/content/browser/devtools/devtools_http_handler.cc#502
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct BrowserInfo<'a> {
    #[serde(rename = "Browser")]
    pub id: Cow<'a, str>,
    #[serde(rename = "User-Agent")]
    pub user_agent: Cow<'a, str>,
    #[serde(rename = "webSocketDebuggerUrl")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ws_url: Option<Cow<'a, str>>,
    #[serde(rename = "Protocol-Version")]
    pub protocol_version: Cow<'a, str>,
    #[serde(rename = "WebKit-Version")]
    #[serde(serialize_with = "serialize_component_version")]
    #[serde(deserialize_with = "deserialize_component_version")]
    pub webkit_version: Option<Cow<'a, str>>,
    #[serde(rename = "V8-Version")]
    #[serde(serialize_with = "serialize_component_version")]
    #[serde(deserialize_with = "deserialize_component_version")]
    pub v8_version: Option<Cow<'a, str>>,
    #[serde(rename = "Android-Package")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub android_package: Option<Cow<'a, str>>,
}

fn serialize_component_version<'a, S>(
    value: &Option<Cow<'a, str>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match *value {
        None => "0".serialize(serializer),
        Some(ref version) => version.serialize(serializer),
    }
}

fn deserialize_component_version<'de, 'a, D>(
    deserializer: D,
) -> Result<Option<Cow<'a, str>>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Cow<'a, str>>::deserialize(deserializer)?;
    Ok(value.and_then(|version| if version == "0" { None } else { Some(version) }))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ActivateResponse {
    Activated,
    NotFound,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum CloseResponse {
    Closed,
    NotFound,
}
