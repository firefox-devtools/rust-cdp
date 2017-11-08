// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

use futures::{Async, Future, Poll};
use hyper::{self, Body, Chunk, Method, Request, Response, StatusCode};
use hyper::server::{BindUpgradableConnection, Http, UpgradableResponse};
use hyper_websocket::WsHandshake;
use regex::Regex;
use serde::{Serialize, Serializer};
use serde_json;
use std::error::Error;
use std::fmt;
use std::net::SocketAddr;
use tokio_core::reactor::Handle;
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_service::Service;

use greeter::{ActivateResponse, BrowserInfo, CloseResponse, Target, WsEndpoint};
use private::start_tools_server;
use tools::server::ToolsServerStart;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GreeterError {
    pub message: Option<String>,
}

impl Default for GreeterError {
    fn default() -> Self {
        GreeterError { message: None }
    }
}

impl<T> From<T> for GreeterError
where
    T: Into<String>,
{
    fn from(src: T) -> Self {
        GreeterError {
            message: Some(src.into()),
        }
    }
}

impl fmt::Display for GreeterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.message {
            Some(ref message) => write!(f, "{}: {}", self.description(), message),
            None => write!(f, "{}", self.description()),
        }
    }
}

impl Error for GreeterError {
    fn description(&self) -> &str {
        "tokio-cdp greeter error"
    }
}

// https://chromium.googlesource.com/chromium/src/+/master/content/browser/devtools/devtools_http_handler.cc#502
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GreeterRequest {
    Browser,
    Protocol,
    List,
    Open { maybe_url: Option<String> },
    Activate { id: String },
    Close { id: String },
}

impl GreeterRequest {
    pub fn command_name(&self) -> &'static str {
        match *self {
            GreeterRequest::Browser => "version",
            GreeterRequest::Protocol => "protocol",
            GreeterRequest::List => "list",
            GreeterRequest::Open { .. } => "new",
            GreeterRequest::Activate { .. } => "activate",
            GreeterRequest::Close { .. } => "close",
        }
    }
}

#[derive(Debug)]
pub struct GreeterResponse {
    status: StatusCode,
    body: Option<Body>,
}

impl GreeterResponse {
    fn new(status: StatusCode) -> Self {
        GreeterResponse {
            status: status,
            body: None,
        }
    }

    fn with_body<T>(self, body: T) -> Self
    where
        T: Into<Body>,
    {
        GreeterResponse {
            status: self.status,
            body: Some(body.into()),
        }
    }

    fn from_result<T, E>(result: Result<T, E>) -> Self
    where
        T: Into<Body>,
        E: fmt::Display,
    {
        match result {
            Ok(body) => GreeterResponse::new(StatusCode::Ok).with_body(body),
            Err(err) => GreeterResponse::new(StatusCode::InternalServerError)
                .with_body(format!("Error serializing response: {}", err.to_string())),
        }
    }

    fn from_value<T>(value: &T) -> Self
    where
        T: Serialize,
    {
        GreeterResponse::from_result(serde_json::to_vec(value))
    }

    pub fn browser<'a>(browser_info: &BrowserInfo<'a>) -> Self {
        GreeterResponse::from_value(browser_info)
    }

    pub fn protocol<T>(json: T) -> Self
    where
        T: Into<Body>,
    {
        GreeterResponse::new(StatusCode::Ok).with_body(json)
    }

    pub fn list<'a, 'b, T>(targets: T) -> Self
    where
        T: IntoIterator<Item = &'b Target<'a>>,
        'a: 'b,
    {
        let mut serializer = serde_json::Serializer::new(Vec::new());
        let result = serializer.collect_seq(targets);
        GreeterResponse::from_result(result.map(|_| serializer.into_inner()))
    }

    pub fn open<'a>(target: &Target<'a>) -> Self {
        GreeterResponse::from_value(target)
    }

    pub fn activate(res: ActivateResponse) -> Self {
        GreeterResponse::new(match res {
            ActivateResponse::Activated => StatusCode::Ok,
            ActivateResponse::NotFound => StatusCode::NotFound,
        })
    }

    pub fn close(res: CloseResponse) -> Self {
        GreeterResponse::new(match res {
            CloseResponse::Closed => StatusCode::Ok,
            CloseResponse::NotFound => StatusCode::NotFound,
        })
    }
}

#[derive(Debug)]
pub struct GreeterServer<T> {
    conn: BindUpgradableConnection<T, Chunk, Option<(WsEndpoint, WsHandshake)>>,
}

impl<T> GreeterServer<T>
where
    T: 'static + AsyncRead + AsyncWrite,
{
    pub fn bind<S>(handle: &Handle, io: T, remote_addr: SocketAddr, service: S) -> Self
    where
        S: 'static
            + Service<Request = GreeterRequest, Response = GreeterResponse, Error = GreeterError>,
    {
        let http_service = GreeterService::new(service);
        let conn = Http::new().bind_upgradable_connection(handle, io, remote_addr, http_service);
        GreeterServer { conn: conn }
    }
}

impl<T> Future for GreeterServer<T>
where
    T: 'static + AsyncRead + AsyncWrite,
{
    type Item = Option<(WsEndpoint, ToolsServerStart<T>)>;
    type Error = hyper::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let maybe_switch_proto = try_ready!(self.conn.poll());
        Ok(Async::Ready(match maybe_switch_proto {
            Some((io, read_buf, Some((ws_endpoint, ws_handshake)))) => {
                Some((ws_endpoint, start_tools_server(ws_handshake.start(io, read_buf))))
            }
            _ => None,
        }))
    }
}

#[derive(Debug)]
struct GreeterService<T> {
    inner: T,
}

impl<T> GreeterService<T> {
    fn new(inner: T) -> Self {
        GreeterService { inner: inner }
    }
}

impl<T> Service for GreeterService<T>
where
    T: Service<Request = GreeterRequest, Response = GreeterResponse, Error = GreeterError>,
{
    type Request = Request;
    type Response = UpgradableResponse<Option<(WsEndpoint, WsHandshake)>>;
    type Error = hyper::Error;
    type Future = GreeterServiceFuture<T::Future>;

    fn call(&self, req: Self::Request) -> Self::Future {
        lazy_static! {
            static ref GREETER_PATH_RE: Regex =
                Regex::new(concat!(r"^/json(/([^/]*))?(/(.*))?$"))
                    .expect("tokio-cdp: GREETER_PATH_RE compilation failed");
            static ref WS_ENDPOINT_RE: Regex =
                Regex::new(concat!(r"^/devtools/(browser|page)(/(.*))?$"))
                    .expect("tokio-cdp: WS_ENDPOINT_RE compilation failed");
        }

        if let Some(handshake) = WsHandshake::detect(&req) {
            let maybe_endpoint = WS_ENDPOINT_RE.captures(req.path()).and_then(|captures| {
                captures
                    .get(1)
                    .and_then(|category| match category.as_str() {
                        "browser" => {
                            Some(WsEndpoint::Browser(captures.get(3).map(|id| id.as_str().into())))
                        }
                        "page" => captures
                            .get(3)
                            .map(|id| WsEndpoint::Page(id.as_str().into())),
                        _ => None,
                    })
            });
            return match maybe_endpoint {
                None => GreeterServiceFuture::WsBadRequest,
                Some(endpoint) => GreeterServiceFuture::WsHandshake(Some((endpoint, handshake))),
            };
        }

        // hyper enforces that responses to connect and head requests don't have
        // a body (we'll get an error if we try to send such a response).
        let can_send_body = *req.method() != Method::Connect && *req.method() != Method::Head;

        let query = req.query();
        let maybe_req = match GREETER_PATH_RE.captures(req.path()) {
            None => None,
            Some(captures) => match captures.get(2) {
                None => Some(GreeterRequest::List),
                Some(command) => match command.as_str() {
                    "version" => Some(GreeterRequest::Browser),
                    "protocol" => Some(GreeterRequest::Protocol),
                    "list" => Some(GreeterRequest::List),
                    "new" => Some(GreeterRequest::Open {
                        maybe_url: query.map(|url| url.to_owned()),
                    }),
                    "activate" => {
                        let id = match captures.get(4) {
                            None => "",
                            Some(id) => id.as_str(),
                        };
                        Some(GreeterRequest::Activate { id: id.to_owned() })
                    }
                    "close" => {
                        let id = match captures.get(4) {
                            None => "",
                            Some(id) => id.as_str(),
                        };
                        Some(GreeterRequest::Close { id: id.to_owned() })
                    }
                    command_str => {
                        return GreeterServiceFuture::HttpNotFound(Some(command_str.to_owned()))
                    }
                },
            },
        };

        match maybe_req {
            None => GreeterServiceFuture::HttpNotFound(None),
            Some(greeter_req) => {
                GreeterServiceFuture::HttpRequest(self.inner.call(greeter_req), can_send_body)
            }
        }
    }
}

#[derive(Debug)]
enum GreeterServiceFuture<T> {
    HttpRequest(T, bool),
    HttpNotFound(Option<String>),
    WsHandshake(Option<(WsEndpoint, WsHandshake)>),
    WsBadRequest,
}

impl<T> Future for GreeterServiceFuture<T>
where
    T: Future<Item = GreeterResponse, Error = GreeterError>,
{
    type Item = UpgradableResponse<Option<(WsEndpoint, WsHandshake)>>;
    type Error = hyper::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(match *self {
            GreeterServiceFuture::HttpRequest(ref mut future, can_send_body) => {
                let res = match future.poll() {
                    Ok(Async::NotReady) => return Ok(Async::NotReady),
                    Ok(Async::Ready(greeter_res)) => {
                        let mut res = Response::new();
                        res.set_status(greeter_res.status);
                        if can_send_body {
                            if let Some(body) = greeter_res.body {
                                res.set_body(body);
                            }
                        }
                        res
                    }
                    Err(err) => {
                        let mut res = Response::new();
                        res.set_status(StatusCode::InternalServerError);
                        if can_send_body {
                            if let Some(message) = err.message {
                                res.set_body(message);
                            }
                        }
                        res
                    }
                };
                UpgradableResponse::Response(res).into()
            }
            GreeterServiceFuture::HttpNotFound(ref mut maybe_command) => {
                let mut res = Response::new().with_status(StatusCode::NotFound);
                if let Some(command) = maybe_command.take() {
                    res.set_body(format!("Unknown command: {}", command));
                }
                UpgradableResponse::Response(res).into()
            }
            GreeterServiceFuture::WsHandshake(ref mut info) => {
                UpgradableResponse::Upgrade(info.take(), None).into()
            }
            GreeterServiceFuture::WsBadRequest => {
                let head = Response::new().with_status(StatusCode::BadRequest);
                UpgradableResponse::Upgrade(None, Some(head)).into()
            }
        })
    }
}
