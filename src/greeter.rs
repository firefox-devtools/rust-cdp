// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{self, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::ser::SerializeStruct;
use serde_json;
use std::fmt::{self, Display, Formatter};
use std::io::Write;
use std::marker::PhantomData;
use std::net::SocketAddr;

// Protocol

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum GreeterRequest<'a> {
    VersionInfo,
    PageList,
    NewPage(Option<&'a str>),
    ActivatePage(&'a str),
}

impl<'a> GreeterRequest<'a> {
    pub fn parse(path: &'a str, query: Option<&'a str>) -> Option<Self> {
        lazy_static! {
            static ref GREETER_PATH_RE: Regex =
                Regex::new(concat!(r"^", cdp_greeter_root_path!(), r"(/([^/]*))?(/(.*))?$"))
                    .expect("cdp: GREETER_PATH_RE compilation failed");
        }

        GREETER_PATH_RE.captures(path).and_then(move |captures| match captures.get(2) {
            None => Some(GreeterRequest::PageList),
            Some(req) => match req.as_str() {
                cdp_greeter_version_info_slug!() => Some(GreeterRequest::VersionInfo),
                cdp_greeter_page_list_slug!() => Some(GreeterRequest::PageList),
                cdp_greeter_new_page_slug!() => Some(GreeterRequest::NewPage(query)),
                cdp_greeter_activate_page_slug!() => {
                    Some(GreeterRequest::ActivatePage(match captures.get(4) {
                        None => "",
                        Some(url) => url.as_str(),
                    }))
                }
                _ => None,
            },
        })
    }

    pub fn parse_with_slash(path: &'a str, query: Option<&'a str>) -> Option<Self> {
        if let Some('/') = path.chars().next() {
            GreeterRequest::parse(&path[1..], query)
        } else {
            None
        }
    }
}

impl<'a> From<&'a OwnedGreeterRequest> for GreeterRequest<'a> {
    fn from(src: &'a OwnedGreeterRequest) -> Self {
        match *src {
            OwnedGreeterRequest::VersionInfo => GreeterRequest::VersionInfo,
            OwnedGreeterRequest::PageList => GreeterRequest::PageList,
            OwnedGreeterRequest::NewPage(ref maybe_url) => {
                GreeterRequest::NewPage(maybe_url.as_ref().map(|x| x.as_str()))
            }
            OwnedGreeterRequest::ActivatePage(ref page_id) => {
                GreeterRequest::ActivatePage(page_id)
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum OwnedGreeterRequest {
    VersionInfo,
    PageList,
    NewPage(Option<String>),
    ActivatePage(String),
}

impl<'a, 'b> From<&'b GreeterRequest<'a>> for OwnedGreeterRequest {
    fn from(src: &'b GreeterRequest<'a>) -> Self {
        match *src {
            GreeterRequest::VersionInfo => OwnedGreeterRequest::VersionInfo,
            GreeterRequest::PageList => OwnedGreeterRequest::PageList,
            GreeterRequest::NewPage(maybe_url) => {
                OwnedGreeterRequest::NewPage(maybe_url.map(Into::into))
            }
            GreeterRequest::ActivatePage(page_id) => {
                OwnedGreeterRequest::ActivatePage(page_id.into())
            }
        }
    }
}

impl<'a> From<GreeterRequest<'a>> for OwnedGreeterRequest {
    fn from(src: GreeterRequest<'a>) -> Self {
        match src {
            GreeterRequest::VersionInfo => OwnedGreeterRequest::VersionInfo,
            GreeterRequest::PageList => OwnedGreeterRequest::PageList,
            GreeterRequest::NewPage(maybe_url) => {
                OwnedGreeterRequest::NewPage(maybe_url.map(Into::into))
            }
            GreeterRequest::ActivatePage(page_id) => {
                OwnedGreeterRequest::ActivatePage(page_id.into())
            }
        }
    }
}

impl OwnedGreeterRequest {
    pub fn parse(path: &str, query: Option<&str>) -> Option<Self> {
        GreeterRequest::parse(path, query).map(|x| x.into())
    }

    pub fn parse_with_slash(path: &str, query: Option<&str>) -> Option<Self> {
        GreeterRequest::parse_with_slash(path, query).map(|x| x.into())
    }
}

pub trait SerializeGreeterResponse {
    fn status(&self) -> u16;

    fn serialize_body<W>(&self, writer: W) -> Result<(), serde_json::Error>
    where
        W: Write;

    fn serialize_body_to_string(&self, string: &mut String) -> Result<(), serde_json::Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GreeterErrorResponse;

impl SerializeGreeterResponse for GreeterErrorResponse {
    fn status(&self) -> u16 {
        500
    }

    fn serialize_body<W>(&self, _writer: W) -> Result<(), serde_json::Error>
    where
        W: Write,
    {
        Ok(())
    }

    fn serialize_body_to_string(&self, _string: &mut String) -> Result<(), serde_json::Error> {
        Ok(())
    }
}

#[derive(Serialize, Clone, Debug, Eq, PartialEq)]
pub struct GreeterVersionInfoResponse<'a>(pub &'a VersionInfo);

impl<'a> SerializeGreeterResponse for GreeterVersionInfoResponse<'a> {
    fn status(&self) -> u16 {
        200
    }

    fn serialize_body<W>(&self, writer: W) -> Result<(), serde_json::Error>
    where
        W: Write,
    {
        serde_json::to_writer(writer, self.0)
    }

    fn serialize_body_to_string(&self, string: &mut String) -> Result<(), serde_json::Error> {
        serialize_json_to_string(string, self.0)
    }
}

#[derive(Serialize, Clone, Debug, Eq, PartialEq)]
pub struct GreeterPageListResponse<'a>(pub &'a [Page]);

impl<'a> SerializeGreeterResponse for GreeterPageListResponse<'a> {
    fn status(&self) -> u16 {
        200
    }

    fn serialize_body<W>(&self, writer: W) -> Result<(), serde_json::Error>
    where
        W: Write,
    {
        serde_json::to_writer(writer, self.0)
    }

    fn serialize_body_to_string(&self, string: &mut String) -> Result<(), serde_json::Error> {
        serialize_json_to_string(string, &self.0)
    }
}

#[derive(Serialize, Clone, Debug, Eq, PartialEq)]
pub struct GreeterNewPageResponse<'a>(pub &'a Page);

impl<'a> SerializeGreeterResponse for GreeterNewPageResponse<'a> {
    fn status(&self) -> u16 {
        200
    }

    fn serialize_body<W>(&self, writer: W) -> Result<(), serde_json::Error>
    where
        W: Write,
    {
        serde_json::to_writer(writer, self.0)
    }

    fn serialize_body_to_string(&self, string: &mut String) -> Result<(), serde_json::Error> {
        serialize_json_to_string(string, self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GreeterActivatePageResponse {
    Activated,
    PageNotFound,
}

impl SerializeGreeterResponse for GreeterActivatePageResponse {
    fn status(&self) -> u16 {
        match *self {
            GreeterActivatePageResponse::Activated => 200,
            GreeterActivatePageResponse::PageNotFound => 404,
        }
    }

    fn serialize_body<W>(&self, _writer: W) -> Result<(), serde_json::Error>
    where
        W: Write,
    {
        Ok(())
    }

    fn serialize_body_to_string(&self, _string: &mut String) -> Result<(), serde_json::Error> {
        Ok(())
    }
}

fn serialize_json_to_string<T>(string: &mut String, src: &T) -> Result<(), serde_json::Error>
where
    T: Serialize,
{
    // serde_json won't produce invalid UTF-8.
    let buf = unsafe { string.as_mut_vec() };
    serde_json::to_writer(buf, src)
}

// Types

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct VersionInfo {
    #[serde(rename = "Browser")]
    pub browser: String,
    #[serde(rename = "Protocol-Version")]
    pub protocol_version: String,
    #[serde(rename = "User-Agent")]
    pub user_agent: String,
    #[serde(rename = "V8-Version")]
    #[serde(serialize_with = "serialize_component_version")]
    #[serde(deserialize_with = "deserialize_component_version")]
    pub v8_version: Option<String>,
    #[serde(rename = "WebKit-Version")]
    #[serde(serialize_with = "serialize_component_version")]
    #[serde(deserialize_with = "deserialize_component_version")]
    pub webkit_version: Option<String>,
}

fn serialize_component_version<S>(value: &Option<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match *value {
        None => "0".serialize(serializer),
        Some(ref version) => version.serialize(serializer),
    }
}

fn deserialize_component_version<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    Ok(value.and_then(|version| if version == "0" { None } else { Some(version) }))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Page {
    pub id: String,
    pub ty: PageType,
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub favicon_url: Option<String>,
    pub devtools_urls: Option<DevToolsUrls>,
}

impl Serialize for Page {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Page", 8)?;

        state.serialize_field(
            "description",
            self.description.as_ref().map(String::as_str).unwrap_or(""),
        )?;

        if let Some(ref urls) = self.devtools_urls {
            state.serialize_field("devtoolsFrontendUrl", &urls.frontend_url)?;
        }

        if let Some(ref url) = self.favicon_url {
            state.serialize_field("faviconUrl", url)?;
        }

        state.serialize_field("id", &self.id)?;
        state.serialize_field("title", &self.title)?;
        state.serialize_field("type", &self.ty)?;
        state.serialize_field("url", &self.url)?;

        if let Some(ref urls) = self.devtools_urls {
            state.serialize_field("webSocketDebuggerUrl", &urls.ws_url)?;
        }

        state.end()
    }
}

impl<'de> Deserialize<'de> for Page {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            Description,
            FrontendUrl,
            FaviconUrl,
            Id,
            Title,
            Type,
            Url,
            WsUrl,
            Unknown,
        }

        struct FieldVisitor;

        impl<'de> Visitor<'de> for FieldVisitor {
            type Value = Field;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("field identifier")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    "description" => Ok(Field::Description),
                    "devtoolsFrontendUrl" => Ok(Field::FrontendUrl),
                    "faviconUrl" => Ok(Field::FaviconUrl),
                    "id" => Ok(Field::Id),
                    "title" => Ok(Field::Title),
                    "type" => Ok(Field::Type),
                    "url" => Ok(Field::Url),
                    "webSocketDebuggerUrl" => Ok(Field::WsUrl),
                    _ => Ok(Field::Unknown),
                }
            }

            fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    b"description" => Ok(Field::Description),
                    b"devtoolsFrontendUrl" => Ok(Field::FrontendUrl),
                    b"faviconUrl" => Ok(Field::FaviconUrl),
                    b"id" => Ok(Field::Id),
                    b"title" => Ok(Field::Title),
                    b"type" => Ok(Field::Type),
                    b"url" => Ok(Field::Url),
                    b"webSocketDebuggerUrl" => Ok(Field::WsUrl),
                    _ => Ok(Field::Unknown),
                }
            }
        }

        impl<'de> Deserialize<'de> for Field {
            #[inline]
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct PageVisitor<'de>(PhantomData<&'de ()>);

        impl<'de> Visitor<'de> for PageVisitor<'de> {
            type Value = Page;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("struct Page")
            }

            #[inline]
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let description = match seq.next_element::<Option<String>>()? {
                    Some(value) => {
                        value.and_then(|desc| if desc.is_empty() { None } else { Some(desc) })
                    }
                    None => return Err(de::Error::invalid_length(0, &"tuple of 8 elements")),
                };

                let frontend_url = match seq.next_element::<Option<String>>()? {
                    Some(value) => value,
                    None => return Err(de::Error::invalid_length(1, &"tuple of 8 elements")),
                };

                let favicon_url = match seq.next_element::<Option<String>>()? {
                    Some(value) => value,
                    None => return Err(de::Error::invalid_length(2, &"tuple of 8 elements")),
                };

                let id = match seq.next_element::<String>()? {
                    Some(value) => value,
                    None => return Err(de::Error::invalid_length(3, &"tuple of 8 elements")),
                };

                let title = match seq.next_element::<String>()? {
                    Some(value) => value,
                    None => return Err(de::Error::invalid_length(4, &"tuple of 8 elements")),
                };

                let ty = match seq.next_element::<PageType>()? {
                    Some(value) => value,
                    None => return Err(de::Error::invalid_length(5, &"tuple of 8 elements")),
                };

                let url = match seq.next_element::<String>()? {
                    Some(value) => value,
                    None => return Err(de::Error::invalid_length(6, &"tuple of 8 elements")),
                };

                let ws_url = match seq.next_element::<Option<String>>()? {
                    Some(value) => value,
                    None => return Err(de::Error::invalid_length(7, &"tuple of 8 elements")),
                };

                let devtools_urls = match (ws_url, frontend_url) {
                    (None, None) => None,
                    (Some(ws_url), Some(frontend_url)) => Some(DevToolsUrls {
                        ws_url: ws_url,
                        frontend_url: frontend_url,
                    }),
                    (None, Some(_)) => return Err(de::Error::missing_field("devtoolsFrontendUrl")),
                    (Some(_), None) => {
                        return Err(de::Error::missing_field("webSocketDebuggerUrl"))
                    }
                };

                Ok(Page {
                    description: description,
                    devtools_urls: devtools_urls,
                    favicon_url: favicon_url,
                    id: id,
                    title: title,
                    ty: ty,
                    url: url,
                })
            }

            #[inline]
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut description: Option<String> = None;
                let mut frontend_url: Option<String> = None;
                let mut favicon_url: Option<String> = None;
                let mut id: Option<String> = None;
                let mut title: Option<String> = None;
                let mut ty: Option<PageType> = None;
                let mut url: Option<String> = None;
                let mut ws_url: Option<String> = None;

                while let Some(key) = map.next_key::<Field>()? {
                    match key {
                        Field::Description => {
                            if description.is_some() {
                                return Err(de::Error::duplicate_field("description"));
                            }
                            description = map.next_value::<Option<String>>()?
                                .and_then(|desc| if desc.is_empty() { None } else { Some(desc) });
                        }
                        Field::FrontendUrl => {
                            if frontend_url.is_some() {
                                return Err(de::Error::duplicate_field("devtoolsFrontendUrl"));
                            }
                            frontend_url = map.next_value::<Option<String>>()?;
                        }
                        Field::FaviconUrl => {
                            if favicon_url.is_some() {
                                return Err(de::Error::duplicate_field("faviconUrl"));
                            }
                            favicon_url = map.next_value::<Option<String>>()?;
                        }
                        Field::Id => {
                            if id.is_some() {
                                return Err(de::Error::duplicate_field("id"));
                            }
                            id = Some(map.next_value::<String>()?);
                        }
                        Field::Title => {
                            if title.is_some() {
                                return Err(de::Error::duplicate_field("title"));
                            }
                            title = Some(map.next_value::<String>()?);
                        }
                        Field::Type => {
                            if ty.is_some() {
                                return Err(de::Error::duplicate_field("type"));
                            }
                            ty = Some(map.next_value::<PageType>()?);
                        }
                        Field::Url => {
                            if url.is_some() {
                                return Err(de::Error::duplicate_field("url"));
                            }
                            url = Some(map.next_value::<String>()?);
                        }
                        Field::WsUrl => {
                            if ws_url.is_some() {
                                return Err(de::Error::duplicate_field("webSocketDebuggerUrl"));
                            }
                            ws_url = map.next_value::<Option<String>>()?;
                        }
                        _ => {
                            map.next_value::<IgnoredAny>()?;
                        }
                    }
                }

                let devtools_urls = match (ws_url, frontend_url) {
                    (None, None) => None,
                    (Some(ws_url), Some(frontend_url)) => Some(DevToolsUrls {
                        ws_url: ws_url,
                        frontend_url: frontend_url,
                    }),
                    (None, Some(_)) => return Err(de::Error::missing_field("devtoolsFrontendUrl")),
                    (Some(_), None) => {
                        return Err(de::Error::missing_field("webSocketDebuggerUrl"))
                    }
                };

                Ok(Page {
                    description: description,
                    devtools_urls: devtools_urls,
                    favicon_url: favicon_url,
                    id: id.ok_or_else(|| de::Error::missing_field("id"))?,
                    title: title.ok_or_else(|| de::Error::missing_field("title"))?,
                    ty: ty.ok_or_else(|| de::Error::missing_field("type"))?,
                    url: url.ok_or_else(|| de::Error::missing_field("url"))?,
                })
            }
        }

        const FIELDS: &'static [&'static str] = &[
            "description",
            "devtoolsFrontendUrl",
            "faviconUrl",
            "id",
            "title",
            "type",
            "url",
            "webSocketDebuggerUrl",
        ];

        deserializer.deserialize_struct("Page", FIELDS, PageVisitor(PhantomData))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct DevToolsUrls {
    #[serde(rename = "webSocketDebuggerUrl")]
    pub ws_url: String,
    #[serde(rename = "devtoolsFrontendUrl")]
    pub frontend_url: String,
}

impl DevToolsUrls {
    pub fn new(addr: &SocketAddr, page_id: &str) -> Self {
        DevToolsUrls {
            ws_url: format!(cdp_ws_url_format!(), server_addr = addr, page_id = page_id),
            frontend_url: format!(
                cdp_frontend_url_format!(),
                server_addr = addr,
                page_id = page_id
            ),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PageType {
    #[serde(rename = "page")]
    Tab,
    #[serde(rename = "background_page")]
    Background,
    #[serde(rename = "service_worker")]
    ServiceWorker,
    #[serde(rename = "app")]
    App,
    #[serde(rename = "other")]
    Other,
}

impl Display for PageType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                PageType::Tab => "tab",
                PageType::Background => "background page",
                PageType::ServiceWorker => "service worker",
                PageType::App => "app",
                PageType::Other => "other page",
            }
        )
    }
}
