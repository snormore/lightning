use std::net::SocketAddr;

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;
use wtransport::endpoint::endpoint_side::Client;
use wtransport::{ClientConfig, Connection, Endpoint, RecvStream, SendStream};

use crate::tls;
use crate::transport::{Transport, TransportReceiver, TransportSender};

pub struct WebTransport {
    target: String,
    endpoint: Endpoint<Client>,
    connection: Mutex<Option<Connection>>,
}

impl WebTransport {
    pub fn new(config: Config) -> Result<Self> {
        let client_config = ClientConfig::builder()
            .with_bind_address(config.bind_address)
            // Todo: We need to customize handshake to validate server certificate hashes.
            // Maybe we don't have to do this if we perform authentication using keys
            // in a custom TLS validator similarly to libp2p.
            .with_custom_tls(tls::tls_config(config.server_hashes))
            .build();
        let endpoint = Endpoint::client(client_config)?;
        Ok(Self {
            endpoint,
            target: config.target,
            connection: Mutex::new(None),
        })
    }
}

pub struct Config {
    pub target: String,
    pub server_hashes: Vec<Vec<u8>>,
    pub bind_address: SocketAddr,
}

#[async_trait]
impl Transport for WebTransport {
    type Sender = WebTransportSender;
    type Receiver = WebTransportReceiver;

    async fn connect(&self) -> Result<(Self::Sender, Self::Receiver)> {
        let mut guard = self.connection.lock().await;

        if guard.is_none() {
            let conn = self.endpoint.connect(&self.target).await?;
            debug_assert!(guard.replace(conn).is_none());
        }

        let (tx_stream, rx_stream) = guard.as_ref().unwrap().open_bi().await?.await?;
        Ok((
            WebTransportSender { inner: tx_stream },
            WebTransportReceiver { inner: rx_stream },
        ))
    }
}

pub struct WebTransportSender {
    inner: SendStream,
}

#[async_trait]
impl TransportSender for WebTransportSender {
    async fn send(&mut self, data: &[u8]) -> Result<()> {
        self.inner.write_all(data).await.map_err(Into::into)
    }
}

pub struct WebTransportReceiver {
    inner: RecvStream,
}

#[async_trait]
impl TransportReceiver for WebTransportReceiver {
    async fn recv(&mut self) -> Result<Bytes> {
        // Todo: verify that read_to_end is cancel safe.
        let mut buffer = Vec::new();
        self.inner.read_to_end(buffer.as_mut()).await?;
        Ok(Bytes::from(buffer))
    }
}
