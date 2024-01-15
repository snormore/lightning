mod config;
#[cfg(test)]
mod tests;

use std::time::Duration;

use fast_sri::IntegrityMetadata;
use lightning_interfaces::infu_collection::Collection;
use lightning_interfaces::types::{Blake3Hash, CompressionAlgorithm};
use lightning_interfaces::{BlockStoreInterface, IncrementalPutInterface};
use reqwest::{Client, Url};

pub use crate::config::Config;

pub struct HttpOriginFetcher<C: Collection> {
    client: Client,
    blockstore: C::BlockStoreInterface,
}

impl<C: Collection> Clone for HttpOriginFetcher<C> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            blockstore: self.blockstore.clone(),
        }
    }
}

impl<C: Collection> HttpOriginFetcher<C> {
    pub fn new(_: Config, blockstore: C::BlockStoreInterface) -> anyhow::Result<Self> {
        let client = Client::new();
        Ok(Self { client, blockstore })
    }

    pub async fn fetch(&self, uri: &[u8]) -> anyhow::Result<Blake3Hash> {
        let (url, sri) = get_url_and_sri(uri)?;
        let resp = self
            .client
            .get(url)
            .timeout(Duration::from_millis(500))
            .send()
            .await?;
        let data = resp.bytes().await?;

        // We verify before inserting any blocks
        if let Some(integrity_metadata) = sri {
            let is_valid = match integrity_metadata {
                IntegrityMetadata::Sha256(integrity) => {
                    let mut verifier = integrity.verifier();
                    verifier.update(data.clone());
                    verifier.verify()
                },
                IntegrityMetadata::Sha512(integrity) => {
                    let mut verifier = integrity.verifier();
                    verifier.update(data.clone());
                    verifier.verify()
                },
                IntegrityMetadata::Blake3(integrity) => {
                    // Todo: in this case we could incrementally verify
                    // a stream. `fastcrypto` does not currently expose an API
                    // to do this with Blake3.
                    // Another option is using the putter to verify the input
                    // but it might insert blocks for invalid files.
                    let mut verifier = integrity.verifier();
                    verifier.update(data.clone());
                    verifier.verify()
                },
                _ => anyhow::bail!("sri failed: unsupported algorithm"),
            };

            if !is_valid {
                anyhow::bail!("sri failed: invalid digest");
            }
        }

        let mut putter = self.blockstore.put(None);
        putter.write(data.as_ref(), CompressionAlgorithm::Uncompressed)?;
        putter.finalize().await.map_err(Into::into)
    }
}

pub(crate) fn get_url_and_sri(uri: &[u8]) -> anyhow::Result<(Url, Option<IntegrityMetadata>)> {
    let uri_str = String::from_utf8(uri.to_vec())?;
    let (url, sri) = uri_str
        .split_once("#integrity=")
        .map(|(url, hash)| (Url::parse(url), Some(hash)))
        .unwrap_or_else(|| (Url::parse(uri_str.as_str()), None));

    let integrity: Option<IntegrityMetadata> = if let Some(sri) = sri {
        Some(sri.parse()?)
    } else {
        None
    };

    Ok((url?, integrity))
}
