pub mod clustering;
mod config;
pub mod divisive;
pub mod pairing;
#[cfg(test)]
mod tests;

use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
pub use config::Config;
use fleek_crypto::NodePublicKey;
use lightning_interfaces::{ConfigConsumer, SyncQueryRunnerInterface, TopologyInterface};
use ndarray::{Array, Array2};

pub struct Topology<Q: SyncQueryRunnerInterface> {
    #[allow(dead_code)]
    query: Q,
    our_public_key: NodePublicKey,
}

impl<Q: SyncQueryRunnerInterface> Topology<Q> {
    #[allow(dead_code)]
    fn build_latency_matrix(&self) -> (Array2<i32>, HashMap<usize, NodePublicKey>, Option<usize>) {
        let latencies = self.query.get_latencies();
        let valid_pubkeys: BTreeSet<NodePublicKey> = self
            .query
            .get_node_registry()
            .into_iter()
            .map(|node_info| node_info.public_key)
            .collect();

        let latency_count = latencies.len();
        let mut latency_map: HashMap<NodePublicKey, HashMap<NodePublicKey, Duration>> =
            HashMap::new();
        let mut latency_sum = Duration::ZERO;
        for ((pubkey_lhs, pubkey_rhs), latency) in latencies {
            if !valid_pubkeys.contains(&pubkey_lhs) || !valid_pubkeys.contains(&pubkey_rhs) {
                continue;
            }

            latency_sum += latency;
            let opposite_dir_latency = latency_map
                .get(&pubkey_rhs)
                .and_then(|latency_row| latency_row.get(&pubkey_lhs));

            let latency = if let Some(opp_latency) = opposite_dir_latency {
                // If a latency measurement for the opposite direction exists, we use the average
                // of both latency measurements.
                let avg_latency = (latency + *opp_latency) / 2;
                latency_map
                    .entry(pubkey_rhs)
                    .or_insert(HashMap::new())
                    .insert(pubkey_lhs, avg_latency);
                avg_latency
            } else {
                latency
            };
            latency_map
                .entry(pubkey_lhs)
                .or_insert(HashMap::new())
                .insert(pubkey_rhs, latency);
        }
        let mean_latency = latency_sum / latency_count as u32; // why do we have to cast to u32?
        let mean_latency: i32 = mean_latency.as_micros().try_into().unwrap_or(i32::MAX);

        let mut matrix = Array::zeros((valid_pubkeys.len(), valid_pubkeys.len()));
        for (index_lhs, pubkey_lhs) in valid_pubkeys.iter().enumerate() {
            for (index_rhs, pubkey_rhs) in valid_pubkeys.iter().enumerate() {
                if index_lhs != index_rhs {
                    matrix[[index_lhs, index_rhs]] = mean_latency;
                    matrix[[index_rhs, index_lhs]] = mean_latency;
                    if let Some(latency) = latency_map
                        .get(pubkey_lhs)
                        .and_then(|latency_row| latency_row.get(pubkey_rhs))
                    {
                        let latency: i32 = latency.as_micros().try_into().unwrap_or(i32::MAX);
                        matrix[[index_lhs, index_rhs]] = latency;
                        matrix[[index_rhs, index_lhs]] = latency;
                    }
                    if let Some(latency) = latency_map
                        .get(pubkey_rhs)
                        .and_then(|latency_row| latency_row.get(pubkey_lhs))
                    {
                        let latency: i32 = latency.as_micros().try_into().unwrap_or(i32::MAX);
                        matrix[[index_lhs, index_rhs]] = latency;
                        matrix[[index_rhs, index_lhs]] = latency;
                    }
                }
            }
        }
        let mut our_index = None;
        let index_to_pubkey: HashMap<usize, NodePublicKey> = valid_pubkeys
            .into_iter()
            .enumerate()
            .map(|(index, pubkey)| {
                if pubkey == self.our_public_key {
                    our_index = Some(index);
                }
                (index, pubkey)
            })
            .collect();
        (matrix, index_to_pubkey, our_index)
    }
}

#[async_trait]
impl<Q: SyncQueryRunnerInterface> TopologyInterface for Topology<Q> {
    type SyncQuery = Q;

    async fn init(
        _config: Self::Config,
        our_public_key: NodePublicKey,
        query_runner: Self::SyncQuery,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            query: query_runner,
            our_public_key,
        })
    }

    fn suggest_connections(&self) -> Arc<Vec<Vec<NodePublicKey>>> {
        todo!()
    }
}

impl<Q: SyncQueryRunnerInterface> ConfigConsumer for Topology<Q> {
    type Config = Config;

    const KEY: &'static str = "TOPOLOGY";
}
