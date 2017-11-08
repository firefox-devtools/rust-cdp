// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

#![cfg_attr(feature = "strict", deny(warnings))]
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

extern crate cdp;
extern crate futures;
extern crate hyper;
#[macro_use]
extern crate quick_error;
extern crate tokio_core;
extern crate tokio_service;

extern crate tokio_cdp;

use futures::{Future, Stream};
use futures::future::{self, FutureResult};
use hyper::{Method, Request, StatusCode, Uri};
use std::cell::RefCell;
use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio_core::net::TcpListener;
use tokio_core::reactor::{Core, Handle};
use tokio_service::Service;

use tokio_cdp::greeter::{ActivateResponse, CloseResponse, Target, TargetKind, BrowserInfo,
                         WsEndpoint};
use tokio_cdp::greeter::client::{GreeterClient, GreeterClientError};
use tokio_cdp::greeter::server::{GreeterError, GreeterRequest, GreeterResponse, GreeterServer};

fn make_sample_browser_info() -> BrowserInfo<'static> {
    BrowserInfo {
        id: "Mozzarella Foxfire/57.0".into(),
        user_agent: "Mozzarella/5.0 Gouda/20100101 Foxfire/57.0".into(),
        ws_url: None,
        protocol_version: cdp::STABLE_PROTOCOL_VERSION.into(),
        v8_version: None,
        webkit_version: None,
        android_package: None,
    }
}

fn make_sample_target(server_addr: &str, url: Option<String>) -> Target<'static> {
    let resolved_url = url.unwrap_or_else(|| "about:blank".into());
    let ws_url = WsEndpoint::page_endpoint_url(server_addr, &resolved_url);
    Target {
        id: resolved_url.clone().into(),
        kind: TargetKind::Page,
        url: resolved_url.into(),
        title: "Zombocom".into(),
        description: None,
        favicon_url: None,
        ws_url: ws_url,
        frontend_url: None,
    }
}

quick_error! {
    #[derive(Debug)]
    enum SampleServerError {
        Io(inner: io::Error) {
            from()
            cause(inner)
        }
        Http(inner: hyper::Error) {
            from()
            cause(inner)
        }
    }
}

#[derive(Clone)]
struct SampleGreeterService {
    server_addr: String,
    targets: Arc<RefCell<Vec<Target<'static>>>>,
}

impl SampleGreeterService {
    fn new(server_addr: String, targets: Arc<RefCell<Vec<Target<'static>>>>) -> Self {
        SampleGreeterService {
            server_addr: server_addr,
            targets: targets,
        }
    }
}

impl Service for SampleGreeterService {
    type Request = GreeterRequest;
    type Response = GreeterResponse;
    type Error = GreeterError;
    type Future = FutureResult<Self::Response, Self::Error>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let res = match req {
            GreeterRequest::Browser => GreeterResponse::browser(&make_sample_browser_info()),
            GreeterRequest::Protocol => GreeterResponse::protocol("{}"),
            GreeterRequest::List => GreeterResponse::list(self.targets.borrow().iter()),
            GreeterRequest::Open { maybe_url } => {
                let target = make_sample_target(&self.server_addr, maybe_url);
                let res = GreeterResponse::open(&target);
                self.targets.borrow_mut().push(target);
                res
            }
            GreeterRequest::Activate { id } => if id == "error" {
                return future::err(GreeterError::default());
            } else if self.targets.borrow().iter().any(|target| target.id == id) {
                GreeterResponse::activate(ActivateResponse::Activated)
            } else {
                GreeterResponse::activate(ActivateResponse::NotFound)
            },
            GreeterRequest::Close { id } => {
                if self.targets.borrow().iter().any(|target| target.id == id) {
                    GreeterResponse::close(CloseResponse::Closed)
                } else {
                    GreeterResponse::close(CloseResponse::NotFound)
                }
            }
        };
        future::ok(res)
    }
}

fn start_server() -> (Core, Handle, String, Arc<RefCell<Vec<Target<'static>>>>) {
    let core = Core::new().expect("core creation error");
    let handle = core.handle();

    let server_addr = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 0);
    let listener = TcpListener::bind(&server_addr, &handle).expect("listener bind error");
    let server_addr = listener.local_addr().expect("server address retrieval error").to_string();

    let targets = Arc::new(RefCell::new(vec![]));

    let greeter_service = SampleGreeterService::new(server_addr.clone(), Arc::clone(&targets));
    let accept_clients = {
        let handle = handle.clone();
        listener
            .incoming()
            .for_each(move |(tcp, remote_addr)| {
                handle.spawn(
                    GreeterServer::bind(&handle, tcp, remote_addr, greeter_service.clone()).then(
                        |result| {
                            result.expect("server error");
                            Ok(())
                        },
                    ),
                );
                Ok(())
            })
            .then(|result| {
                result.expect("server accept error");
                Ok(())
            })
    };
    handle.spawn(accept_clients);

    (core, handle, server_addr, targets)
}

#[test]
fn test_greeter_browser() {
    let (mut core, handle, server_addr, _targets) = start_server();
    let client = GreeterClient::new(&handle, server_addr.clone());

    let test =
        client.browser().map(|browser_info| assert_eq!(browser_info, make_sample_browser_info()));
    core.run(test).expect("client error");
}

#[test]
fn test_greeter_protocol() {
    let (mut core, handle, server_addr, _targets) = start_server();
    let client = GreeterClient::new(&handle, server_addr.clone());

    let test = client
        .protocol()
        .and_then(|stream| stream.concat2().from_err())
        .map(|json| assert_eq!(b"{}", json.as_ref()));
    core.run(test).expect("client error");
}

#[test]
fn test_greeter_list_empty() {
    let (mut core, handle, server_addr, _targets) = start_server();
    let client = GreeterClient::new(&handle, server_addr.clone());

    let test = client.list().map(|targets| assert_eq!(targets, vec![]));
    core.run(test).expect("client error");
}

#[test]
fn test_greeter_open_no_url() {
    let (mut core, handle, server_addr, targets) = start_server();
    let client = GreeterClient::new(&handle, server_addr.clone());

    let test = client.open(None).map(move |target| {
        assert_eq!(target, make_sample_target(&server_addr, None));
        assert_eq!(&*targets.borrow(), &vec![make_sample_target(&server_addr, None)]);
    });
    core.run(test).expect("client error");
}

#[test]
fn test_greeter_open_with_url() {
    let (mut core, handle, server_addr, targets) = start_server();
    let client = GreeterClient::new(&handle, server_addr.clone());

    let test = client.open(Some("https://mozilla.org")).map(move |target| {
        assert_eq!(target, make_sample_target(&server_addr, Some("https://mozilla.org".into())));
        assert_eq!(
            &*targets.borrow(),
            &vec![make_sample_target(&server_addr, Some("https://mozilla.org".into()))]
        );
    });
    core.run(test).expect("client error");
}

#[test]
fn test_greeter_list_populated() {
    let (mut core, handle, server_addr, targets) = start_server();
    let client = GreeterClient::new(&handle, server_addr.clone());

    let test = client
        .open(None)
        .and_then(move |_| client.open(Some("https://mozilla.org")).map(|x| (x, client)))
        .and_then(|(_, client)| client.list())
        .map(move |received_targets| {
            assert_eq!(
                received_targets,
                vec![
                    make_sample_target(&server_addr, None),
                    make_sample_target(&server_addr, Some("https://mozilla.org".into())),
                ]
            );
            assert_eq!(&*targets.borrow(), &received_targets);
        });
    core.run(test).expect("client error");
}

#[test]
fn test_greeter_activate_nonexistent() {
    let (mut core, handle, server_addr, _targets) = start_server();
    let client = GreeterClient::new(&handle, server_addr.clone());

    let test = client.activate("https://mozilla.org").and_then(move |response| {
        assert_eq!(ActivateResponse::NotFound, response);
        Ok(())
    });
    core.run(test).expect("client error");
}

#[test]
fn test_greeter_activate_existing() {
    let (mut core, handle, server_addr, _targets) = start_server();
    let client = GreeterClient::new(&handle, server_addr.clone());

    let test = client
        .open(Some("https://mozilla.org"))
        .and_then(move |_| client.activate("https://mozilla.org"))
        .and_then(move |response| {
            assert_eq!(ActivateResponse::Activated, response);
            Ok(())
        });
    core.run(test).expect("client error");
}

#[test]
fn test_greeter_close_nonexistent() {
    let (mut core, handle, server_addr, _targets) = start_server();
    let client = GreeterClient::new(&handle, server_addr.clone());

    let test = client.close("https://mozilla.org").and_then(move |response| {
        assert_eq!(CloseResponse::NotFound, response);
        Ok(())
    });
    core.run(test).expect("client error");
}

#[test]
fn test_greeter_close_existing() {
    let (mut core, handle, server_addr, _targets) = start_server();
    let client = GreeterClient::new(&handle, server_addr.clone());

    let test = client
        .open(Some("https://mozilla.org"))
        .and_then(move |_| client.close("https://mozilla.org"))
        .and_then(move |response| {
            assert_eq!(CloseResponse::Closed, response);
            Ok(())
        });
    core.run(test).expect("client error");
}

#[test]
fn test_greeter_methods() {
    let (mut core, handle, server_addr, _targets) = start_server();
    let http = hyper::Client::new(&handle);

    let uri: Uri =
        format!("http://{server_addr}/json", server_addr = server_addr).parse().unwrap();

    // CDP servers pretty much ignore the method attached to the HTTP request.
    let methods = [
        Method::Options,
        Method::Get,
        Method::Post,
        Method::Put,
        Method::Delete,
        Method::Head,
        Method::Trace,
        Method::Connect,
        Method::Patch,
        Method::Extension("FOO".into()),
    ];

    for method in &methods {
        let req = Request::new(method.clone(), uri.clone());
        let test = http.request(req).map(|res| {
            assert_eq!(
                res.status(),
                StatusCode::Ok,
                "expected StatusCode::Ok for method {} but got {}",
                method,
                res.status()
            )
        });
        core.run(test).expect("http error");
    }
}

#[test]
fn test_greeter_unknown_command() {
    let (mut core, handle, server_addr, _targets) = start_server();
    let http = hyper::Client::new(&handle);

    let uri = format!("http://{server_addr}/foo", server_addr = server_addr).parse().unwrap();
    let test = http.get(uri).map(|res| assert_eq!(res.status(), StatusCode::NotFound));
    core.run(test).expect("http error");
}

#[test]
fn test_greeter_server_error() {
    let (mut core, handle, server_addr, _targets) = start_server();
    let client = GreeterClient::new(&handle, server_addr.clone());

    let test = client.activate("error");
    let err = core.run(test).expect_err("expected client error");
    match err {
        GreeterClientError::BadStatus(status) => {
            assert_eq!(StatusCode::InternalServerError, status);
        }
        _ => panic!("expected GreeterClientError::BadStatus: {:#?}", err),
    }
}
