// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.

use serde::Serialize;
use std::error::Error;

use traits::{DeserializeCdpCommand, SerializeCdpEvent};

/// The encoded command type for the server side of this transport.
///
/// Can be deserialized into a specific command type via `deserialize_command`.
/// This could be an enum of command types supported by the server
/// implementation. In that case, an implementation of `DeserializeCdpCommand`
/// can be derived with the `cdp-derive` crate.
pub trait CdpServerCommand: Sized {
    /// The type of a deserialization error.
    type Error: Error;

    /// Get the name of the encoded command.
    fn command_name(&self) -> &str;

    /// Attempt to deserialize the encoded command into a specific command type.
    ///
    /// If the command type recognizes the name of this command and attempts to
    /// deserialize it, returns the result of that deserialization wrapped in an
    /// Ok. Otherwise, gives back the encoded command wrapped in an Err.
    fn deserialize_command<C>(self) -> Result<Result<C, Self::Error>, Self>
    where
        C: for<'de> DeserializeCdpCommand<'de>;
}

/// A handle which grants the ability to respond to a particular command sent by
/// the client.
///
/// Calling any of the methods except `event_sender` on this trait will consume
/// the responder and submit a response to the client.
pub trait CdpServerResponder: Sized {
    /// The type of a respond error.
    type Error: Error;

    /// A cloneable handle which grants the ability to push events to the same
    /// client which would receive a response submitted through this
    /// `CdpServerResponder`.
    type EventSender: CdpServerEventSender<Error = Self::Error>;

    /// Generate a cloneable handle which grants the ability to push events to
    /// the same client which would receive a response submitted through this
    /// `CdpServerResponder`.
    fn event_sender(&self) -> Self::EventSender;

    /// Send back a successful response.
    ///
    /// If an error is encountered serializing the response, implementations of
    /// this trait should attempt to send an `internal_error` back to the
    /// client, in addition to returning `Err` from this method.
    fn respond<R>(self, response: &R) -> Result<(), Self::Error>
    where
        R: Serialize;

    /// Send back a failure response, indicating to the client that the command
    /// was not recognized or is not supported by this particular server
    /// implementation.
    fn command_not_found(self) -> Result<(), Self::Error>;

    /// Send back a failure response, indicating to the client that while the
    /// command was
    /// recognized, the parameters specified were invalid.
    ///
    /// ```rust,ignore
    /// responder.invalid_params("url: string value expected".into())
    /// ```
    fn invalid_params(self, message: String) -> Result<(), Self::Error>;

    /// Send back a failure response, indicating to the client that while the
    /// command was understood and the server implementation is functioning
    /// properly, the command could not be carried out for some reason.
    ///
    /// ```rust,ignore
    /// responder.server_error("The URL specified is invalid.".into())
    /// responder.server_error("The TLS handshake failed.".into())
    /// ```
    fn server_error(self, message: String) -> Result<(), Self::Error>;

    /// Send back a failure response, indicating to the client that an internal,
    /// server implementation-specific error occurred (for example, an IPC
    /// channel disconnected unexpectedly).
    fn internal_error(self) -> Result<(), Self::Error>;
}

/// A cloneable handle which grants the ability to push events from the server
/// to the client.
pub trait CdpServerEventSender: Sized + Clone {
    /// The type of a send error.
    type Error: Error;

    /// Push an event to the client.
    fn send_event<E>(&self, event: &E) -> Result<(), Self::Error>
    where
        E: SerializeCdpEvent;
}
