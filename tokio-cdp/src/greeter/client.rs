// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

use futures::{Future, Poll, Stream};
use futures::stream::Concat2;
use hyper::{self, Body, Chunk, Response, StatusCode};
use hyper::client::{FutureResponse, HttpConnector};
use hyper::error::UriError;
use serde::Deserialize;
use serde_json;
use std::error::Error;
use std::marker::PhantomData;
use std::mem;
use tokio_core::reactor::Handle;

use greeter::{ActivateResponse, BrowserInfo, CloseResponse, Target};

quick_error! {
    #[derive(Debug)]
    pub enum GreeterClientError {
        UriParseError(inner: UriError) {
            from()
            description("URI parse error")
            display(me) -> ("{}: {}", me.description(), inner)
            cause(inner)
        }
        HttpError(inner: hyper::Error) {
            from()
            description("HTTP error")
            display(me) -> ("{}: {}", me.description(), inner)
            cause(inner)
        }
        JsonError(inner: serde_json::Error) {
            from()
            description("JSON parse error")
            display(me) -> ("{}: {}", me.description(), inner)
            cause(inner)
        }
        BadStatus(status: StatusCode) {
            description("received a bad status code")
            display(me) -> ("{} ({})", me.description(), status)
        }
    }
}

#[derive(Clone, Debug)]
pub struct GreeterClient {
    inner: hyper::Client<HttpConnector>,
    server_addr: String,
}

impl GreeterClient {
    pub fn new(handle: &Handle, server_addr: String) -> Self {
        GreeterClient {
            inner: hyper::Client::new(handle),
            server_addr: server_addr,
        }
    }

    pub fn browser(&self) -> GreeterClientBrowser {
        let url = format!("http://{server_addr}/json/version", server_addr = self.server_addr);
        GreeterClientBrowser {
            inner: self.request_json(&url),
        }
    }

    pub fn list(&self) -> GreeterClientList {
        let url = format!("http://{server_addr}/json/list", server_addr = self.server_addr);
        GreeterClientList {
            inner: self.request_json(&url),
        }
    }

    pub fn open(&self, maybe_url: Option<&str>) -> GreeterClientOpen {
        let url = match maybe_url {
            None => format!("http://{server_addr}/json/new", server_addr = self.server_addr),
            Some(url) => format!(
                "http://{server_addr}/json/new?{url}",
                server_addr = self.server_addr,
                url = url,
            ),
        };
        GreeterClientOpen {
            inner: self.request_json(&url),
        }
    }

    pub fn activate(&self, id: &str) -> GreeterClientActivate {
        let url = format!(
            "http://{server_addr}/json/activate/{id}",
            server_addr = self.server_addr,
            id = id,
        );
        GreeterClientActivate {
            inner: self.request(&url),
        }
    }

    pub fn close(&self, id: &str) -> GreeterClientClose {
        let url = format!(
            "http://{server_addr}/json/close/{id}",
            server_addr = self.server_addr,
            id = id,
        );
        GreeterClientClose {
            inner: self.request(&url),
        }
    }

    pub fn protocol(&self) -> GreeterClientProtocol {
        let url = format!("http://{server_addr}/json/protocol", server_addr = self.server_addr);
        GreeterClientProtocol {
            inner: self.request(&url),
        }
    }

    fn request(&self, request_url: &str) -> GreeterClientRequest {
        let request_uri = match request_url.parse() {
            Err(err) => return GreeterClientRequest::Err(Some(GreeterClientError::from(err))),
            Ok(request_uri) => request_uri,
        };
        GreeterClientRequest::MakeRequest(self.inner.get(request_uri))
    }

    fn request_body(&self, request_url: &str) -> GreeterClientRequestBody {
        GreeterClientRequestBody::MakeRequest(self.request(request_url))
    }

    fn request_json<T>(&self, request_url: &str) -> GreeterClientRequestJson<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        GreeterClientRequestJson {
            inner: self.request_body(request_url),
            phantom: PhantomData,
        }
    }
}

#[derive(Debug)]
enum GreeterClientRequest {
    Err(Option<GreeterClientError>),
    MakeRequest(FutureResponse),
}

impl Future for GreeterClientRequest {
    type Item = Response;
    type Error = GreeterClientError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match *self {
            GreeterClientRequest::Err(ref mut err) => Err(
                err.take()
                    .expect("tokio-cdp: GreeterClientRequest polled after error"),
            ),
            GreeterClientRequest::MakeRequest(ref mut future) => future.poll().map_err(From::from),
        }
    }
}

#[derive(Debug)]
enum GreeterClientRequestBody {
    MakeRequest(GreeterClientRequest),
    ReadBody(Concat2<Body>),
}

impl Future for GreeterClientRequestBody {
    type Item = Chunk;
    type Error = GreeterClientError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let new = match *self {
                GreeterClientRequestBody::MakeRequest(ref mut future) => {
                    let res = try_ready!(future.poll());
                    let status = res.status();
                    match status {
                        StatusCode::Ok => GreeterClientRequestBody::ReadBody(res.body().concat2()),
                        _ => return Err(GreeterClientError::BadStatus(status)),
                    }
                }
                GreeterClientRequestBody::ReadBody(ref mut future) => {
                    return future.poll().map_err(From::from)
                }
            };
            mem::replace(self, new);
        }
    }
}

#[derive(Debug)]
struct GreeterClientRequestJson<T> {
    inner: GreeterClientRequestBody,
    phantom: PhantomData<T>,
}

impl<T> Future for GreeterClientRequestJson<T>
where
    T: for<'de> Deserialize<'de>,
{
    type Item = T;
    type Error = GreeterClientError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let body = try_ready!(self.inner.poll());
        serde_json::from_slice::<T>(&body)
            .map(From::from)
            .map_err(From::from)
    }
}

macro_rules! request_json_future {
    ( $id:ident, $out:ty ) => {
        #[derive(Debug)]
        pub struct $id {
            inner: GreeterClientRequestJson<$out>,
        }

        impl Future for $id {
            type Item = $out;
            type Error = GreeterClientError;

            fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
                self.inner.poll()
            }
        }
    }
}
request_json_future!(GreeterClientBrowser, BrowserInfo<'static>);
request_json_future!(GreeterClientList, Vec<Target<'static>>);
request_json_future!(GreeterClientOpen, Target<'static>);

#[derive(Debug)]
pub struct GreeterClientActivate {
    inner: GreeterClientRequest,
}

impl Future for GreeterClientActivate {
    type Item = ActivateResponse;
    type Error = GreeterClientError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let res = try_ready!(self.inner.poll());
        match res.status() {
            StatusCode::Ok => Ok(ActivateResponse::Activated.into()),
            StatusCode::NotFound => Ok(ActivateResponse::NotFound.into()),
            status => Err(GreeterClientError::BadStatus(status)),
        }
    }
}

#[derive(Debug)]
pub struct GreeterClientClose {
    inner: GreeterClientRequest,
}

impl Future for GreeterClientClose {
    type Item = CloseResponse;
    type Error = GreeterClientError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let res = try_ready!(self.inner.poll());
        match res.status() {
            StatusCode::Ok => Ok(CloseResponse::Closed.into()),
            StatusCode::NotFound => Ok(CloseResponse::NotFound.into()),
            status => Err(GreeterClientError::BadStatus(status)),
        }
    }
}

#[derive(Debug)]
pub struct GreeterClientProtocol {
    inner: GreeterClientRequest,
}

impl Future for GreeterClientProtocol {
    type Item = Body;
    type Error = GreeterClientError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let res = try_ready!(self.inner.poll());
        match res.status() {
            StatusCode::Ok => Ok(res.body().into()),
            status => Err(GreeterClientError::BadStatus(status)),
        }
    }
}
