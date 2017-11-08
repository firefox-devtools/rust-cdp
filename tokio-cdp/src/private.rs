// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

use futures::{Async, Future, Poll, Sink, StartSend, Stream};
use hyper_websocket::{AcceptWsHandshake, RejectWsHandshake, WsStart};
use std::fmt;
use tokio_io::{AsyncRead, AsyncWrite};
use websocket::async::Client as WsClient;
use websocket::result::WebSocketError;
use websocket_transport::WsTransport;

use common::Omitted;

#[derive(Debug)]
pub struct ToolsServerStart<T> {
    ws_start: WsStart<T>,
}

pub fn start_tools_server<T>(ws_start: WsStart<T>) -> ToolsServerStart<T> {
    ToolsServerStart { ws_start: ws_start }
}

impl<T> ToolsServerStart<T>
where
    T: 'static + AsyncRead + AsyncWrite,
{
    pub fn accept(self) -> AcceptToolsHandshake<T> {
        AcceptToolsHandshake {
            future: self.ws_start.accept(),
        }
    }

    pub fn reject(self) -> RejectToolsHandshake<T> {
        RejectToolsHandshake {
            future: self.ws_start.reject(),
        }
    }
}

#[derive(Debug)]
pub struct AcceptToolsHandshake<T> {
    future: AcceptWsHandshake<T>,
}

impl<T> Future for AcceptToolsHandshake<T> {
    type Item = ToolsServer<T>;
    type Error = WebSocketError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let ws = try_ready!(self.future.poll());
        let ws_transport = WsTransport::new(ws);
        let server = ToolsServer {
            ws_transport: ws_transport,
        };
        Ok(Async::Ready(server))
    }
}

#[derive(Debug)]
pub struct RejectToolsHandshake<T: AsyncWrite> {
    future: RejectWsHandshake<T>,
}

impl<T> Future for RejectToolsHandshake<T>
where
    T: AsyncWrite,
{
    type Item = ();
    type Error = WebSocketError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        try_ready!(self.future.poll());
        Ok(Async::Ready(()))
    }
}

pub struct ToolsServer<T> {
    ws_transport: WsTransport<WsClient<T>>,
}

impl<T> fmt::Debug for ToolsServer<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ToolsServer")
            .field("ws_transport", &Omitted)
            .finish()
    }
}

impl<T> Stream for ToolsServer<T>
where
    T: AsyncRead + AsyncWrite,
{
    type Item = String;
    type Error = WebSocketError;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.ws_transport.poll()
    }
}

impl<T> Sink for ToolsServer<T>
where
    T: AsyncWrite,
{
    type SinkItem = String;
    type SinkError = WebSocketError;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        self.ws_transport.start_send(item)
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.ws_transport.poll_complete()
    }

    fn close(&mut self) -> Poll<(), Self::SinkError> {
        self.ws_transport.close()
    }
}
