use async_trait::async_trait;
use axum::Router;
use enum_dispatch::enum_dispatch;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::schema;
use crate::shutdown::ShutdownWaiter;

pub mod mock;
pub mod tcp;
pub mod webrtc;
pub mod webtransport;

#[async_trait]
pub trait Transport: Sized + Send + Sync + 'static {
    type Config: Default + Serialize + DeserializeOwned;

    type Sender: TransportSender + Into<StaticSender>;
    type Receiver: TransportReceiver;

    /// Bind the transport with the provided config.
    async fn bind(
        shutdown: ShutdownWaiter,
        config: Self::Config,
    ) -> anyhow::Result<(Self, Option<Router>)>;

    /// Accept a new connection.
    async fn accept(
        &mut self,
    ) -> Option<(schema::HandshakeRequestFrame, Self::Sender, Self::Receiver)>;
}

#[enum_dispatch(StaticSender)]
pub trait TransportSender: Sized + Send + Sync + 'static {
    /// Send the initial handshake response to the client.
    fn send_handshake_response(&mut self, response: schema::HandshakeResponse);

    /// Send a frame to the client.
    fn send(&mut self, frame: schema::ResponseFrame);

    /// Terminate the connection
    fn terminate(mut self, reason: schema::TerminationReason) {
        self.send(schema::ResponseFrame::Termination { reason })
    }
}

#[async_trait]
pub trait TransportReceiver: Sized + Send + Sync + 'static {
    /// Receive a frame from the connection. Returns `None` when the connection
    /// is closed.
    async fn recv(&mut self) -> Option<schema::RequestFrame>;
}

/// This enum is supposed to help us with avoiding dynamic dispatch over different
/// sender objects.
#[enum_dispatch]
pub enum StaticSender {
    MockTransport(mock::MockTransportSender),
    TcpTransport(tcp::TcpSender),
    WebRtcTransport(webrtc::WebRtcSender),
    WebTransport(webtransport::WebTransportSender),
}
