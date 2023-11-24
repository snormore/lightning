use arrayref::array_ref;
use cid::Cid;
use lightning_handshake::schema::{HandshakeRequestFrame, RequestFrame, ResponseFrame};
use tcp_client::TcpClient;

const ADDRESS: &str = "127.0.0.1:4221";
const CID: &str = "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi";
const SERVICE_ID: u32 = 0;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Connect and handshake with the node
    let mut client = TcpClient::connect(ADDRESS).await?;
    client
        .send_handshake(HandshakeRequestFrame::Handshake {
            retry: None,
            service: SERVICE_ID,
            pk: [0; 96].into(),
            pop: [0; 48].into(),
        })
        .await?;

    println!("send handshake");

    // Send a request for the CID.
    let cid = Cid::try_from(CID).expect("valid cid").to_bytes();
    client
        .send(RequestFrame::ServicePayload { bytes: cid.into() })
        .await?;
    println!("sent request for cid");

    let mut total = 0;

    // Read the number of blocks we should receive back.
    let Some(ResponseFrame::ServicePayload { bytes }) = client.recv().await else {
        panic!("invalid or no response received");
    };

    println!("first frame was {} bytes long", bytes.len());
    let num_blocks = u32::from_be_bytes(*array_ref![bytes, 0, 4]);
    println!("Expecting {num_blocks} block(s) of content");

    // Stream the remaining content
    for _ in 0..num_blocks {
        println!("receiving block");

        // Read the block
        let Some(ResponseFrame::ServicePayload { bytes }) = client.recv().await else {
            panic!("invalid or no response received");
        };

        total += bytes.len();
    }

    println!("Successfully streamed {total} bytes for {CID}");

    Ok(())
}

mod tcp_client {
    use anyhow::Result;
    use arrayref::array_ref;
    use bytes::{Bytes, BytesMut};
    use lightning_handshake::schema::{HandshakeRequestFrame, RequestFrame, ResponseFrame};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpStream, ToSocketAddrs};

    pub struct TcpClient {
        pub stream: TcpStream,
        pub buffer: BytesMut,
    }

    impl TcpClient {
        pub async fn connect<A: ToSocketAddrs>(addr: A) -> Result<Self> {
            let stream = TcpStream::connect(addr).await?;
            Ok(Self {
                stream,
                buffer: BytesMut::with_capacity(4),
            })
        }

        pub async fn recv(&mut self) -> Option<ResponseFrame> {
            loop {
                if self.buffer.len() < 4 {
                    // Read more bytes for the length delimiter
                    self.stream.read_buf(&mut self.buffer).await.ok()?;
                } else {
                    // parse the length delimiter
                    let len = u32::from_be_bytes(*array_ref!(self.buffer, 0, 4)) as usize + 4;
                    self.buffer.reserve(len);

                    // if we need more bytes, read until we have enough
                    while self.buffer.len() < len {
                        self.stream.read_buf(&mut self.buffer).await.ok()?;
                    }

                    // take the frame bytes from the buffer
                    let bytes = self.buffer.split_to(len);

                    // decode the frame
                    match ResponseFrame::decode(&bytes[4..]) {
                        Ok(frame) => return Some(frame),
                        Err(_) => {
                            eprintln!("invalid frame, dropping payload");
                            continue;
                        },
                    }
                }
            }
        }

        pub async fn send_handshake(&mut self, frame: HandshakeRequestFrame) -> Result<()> {
            self.send_inner(frame.encode()).await
        }

        pub async fn send(&mut self, frame: RequestFrame) -> Result<()> {
            self.send_inner(frame.encode()).await
        }

        async fn send_inner(&mut self, bytes: Bytes) -> Result<()> {
            self.stream.write_u32(bytes.len() as u32).await?;
            self.stream.write_all(&bytes).await?;
            Ok(())
        }
    }
}
