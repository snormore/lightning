use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use atomo::{Atomo, AtomoBuilder, DefaultSerdeBackend, UpdatePerm};
use atomo_rocks::{Options, RocksBackend, RocksBackendBuilder};
use lightning_interfaces::types::Epoch;

use crate::config::CheckpointerDatabaseConfig;
use crate::database::CheckpointerDatabase;
use crate::headers::{AggregateCheckpointHeader, CheckpointHeader};

const CHECKPOINT_HEADERS_TABLE: &str = "checkpoint_headers";
const AGGREGATE_CHECKPOINT_HEADERS_TABLE: &str = "aggregate_checkpoint_headers";

/// A checkpointer database that uses RocksDB as the underlying datastore.
///
/// It is thread-safe and can be shared between multiple threads.
#[derive(Clone)]
pub struct RocksCheckpointerDatabase {
    atomo: Arc<Mutex<Atomo<UpdatePerm, RocksBackend, DefaultSerdeBackend>>>,
}

impl RocksCheckpointerDatabase {
    pub fn new(atomo: Arc<Mutex<Atomo<UpdatePerm, RocksBackend, DefaultSerdeBackend>>>) -> Self {
        Self { atomo }
    }
}

impl CheckpointerDatabase for RocksCheckpointerDatabase {
    fn build(config: CheckpointerDatabaseConfig) -> Self {
        let mut options = Options::default();
        options.create_if_missing(true);
        options.create_missing_column_families(true);

        let builder = RocksBackendBuilder::new(config.path.to_path_buf()).with_options(options);
        let builder = AtomoBuilder::new(builder)
            .with_table::<Epoch, HashSet<CheckpointHeader>>(CHECKPOINT_HEADERS_TABLE)
            .with_table::<Epoch, AggregateCheckpointHeader>(AGGREGATE_CHECKPOINT_HEADERS_TABLE);

        let db = builder.build().unwrap();
        let db = Arc::new(Mutex::new(db));

        Self::new(db)
    }

    fn get_checkpoint_headers(&self, epoch: Epoch) -> Result<HashSet<CheckpointHeader>> {
        let headers = self.atomo.lock().unwrap().query().run(|ctx| {
            let table = ctx.get_table::<Epoch, HashSet<CheckpointHeader>>(CHECKPOINT_HEADERS_TABLE);

            table.get(epoch).unwrap_or_default()
        });

        Ok(headers)
    }

    fn add_checkpoint_header(&self, epoch: Epoch, header: CheckpointHeader) -> Result<()> {
        self.atomo.lock().unwrap().run(|ctx| {
            let mut table =
                ctx.get_table::<Epoch, HashSet<CheckpointHeader>>(CHECKPOINT_HEADERS_TABLE);

            let mut headers = table.get(epoch).unwrap_or_default();
            headers.insert(header);
            table.insert(epoch, headers);
        });

        Ok(())
    }

    fn get_aggregate_checkpoint_header(
        &self,
        epoch: Epoch,
    ) -> Result<Option<AggregateCheckpointHeader>> {
        let header = self.atomo.lock().unwrap().query().run(|ctx| {
            let table = ctx
                .get_table::<Epoch, AggregateCheckpointHeader>(AGGREGATE_CHECKPOINT_HEADERS_TABLE);

            table.get(epoch)
        });

        Ok(header)
    }

    fn set_aggregate_checkpoint_header(
        &self,
        epoch: Epoch,
        header: AggregateCheckpointHeader,
    ) -> Result<()> {
        self.atomo.lock().unwrap().run(|ctx| {
            let mut table = ctx
                .get_table::<Epoch, AggregateCheckpointHeader>(AGGREGATE_CHECKPOINT_HEADERS_TABLE);

            table.insert(epoch, header);
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bit_set::BitSet;
    use fleek_crypto::{ConsensusAggregateSignature, ConsensusSignature};
    use lightning_interfaces::types::NodeIndex;
    use rand::Rng;
    use tempfile::tempdir;

    use super::*;

    fn random_checkpoint_header(epoch: Epoch) -> CheckpointHeader {
        let mut rng = rand::thread_rng();

        CheckpointHeader {
            epoch,
            node_id: NodeIndex::from(rng.gen::<u32>()),
            previous_state_root: rng.gen::<[u8; 32]>(),
            next_state_root: rng.gen::<[u8; 32]>(),
            serialized_state_digest: rng.gen::<[u8; 32]>(),
            signature: ConsensusSignature({
                let mut sig = [0u8; 48];
                for item in &mut sig {
                    *item = rng.gen();
                }
                sig
            }),
        }
    }

    fn random_aggregate_checkpoint_header(epoch: Epoch) -> AggregateCheckpointHeader {
        let mut rng = rand::thread_rng();

        AggregateCheckpointHeader {
            epoch,
            previous_state_root: rng.gen::<[u8; 32]>(),
            next_state_root: rng.gen::<[u8; 32]>(),
            signature: ConsensusAggregateSignature({
                let mut sig = [0u8; 48];
                for item in &mut sig {
                    *item = rng.gen();
                }
                sig
            }),
            nodes: (0..32).fold(BitSet::with_capacity(32), |mut bs, i| {
                if rng.gen_bool(0.5) {
                    bs.insert(i);
                }
                bs
            }),
        }
    }

    #[test]
    fn test_add_and_get_checkpoint_headers() {
        let tempdir = tempdir().unwrap();
        let db = RocksCheckpointerDatabase::build(CheckpointerDatabaseConfig {
            path: tempdir.path().to_path_buf().try_into().unwrap(),
        });

        // Check that the database is empty.
        let headers = db.get_checkpoint_headers(0).unwrap();
        assert_eq!(headers, HashSet::new());

        // Add some headers and check that they're retrievable.
        let epoch0_headers = (0..10)
            .map(|_| random_checkpoint_header(0))
            .collect::<HashSet<_>>();
        for header in epoch0_headers.clone() {
            db.add_checkpoint_header(0, header.clone()).unwrap();
        }
        assert_eq!(db.get_checkpoint_headers(0).unwrap(), epoch0_headers);

        // Add the same headers and check that it doesn't duplicate.
        for header in epoch0_headers.clone() {
            db.add_checkpoint_header(0, header.clone()).unwrap();
        }
        assert_eq!(db.get_checkpoint_headers(0).unwrap(), epoch0_headers);

        // Add headers for a different epoch and check that it doesn't affect the previous epoch.
        assert_eq!(db.get_checkpoint_headers(1).unwrap(), HashSet::new());
        let epoch1_headers = (0..10)
            .map(|_| random_checkpoint_header(0))
            .collect::<HashSet<_>>();
        for header in epoch1_headers.clone() {
            db.add_checkpoint_header(1, header.clone()).unwrap();
        }
        assert_eq!(db.get_checkpoint_headers(0).unwrap(), epoch0_headers);
        assert_eq!(db.get_checkpoint_headers(1).unwrap(), epoch1_headers);
    }

    #[test]
    fn test_set_and_get_aggregate_checkpoint_header() {
        let tempdir = tempdir().unwrap();
        let db = RocksCheckpointerDatabase::build(CheckpointerDatabaseConfig {
            path: tempdir.path().to_path_buf().try_into().unwrap(),
        });

        // Check that the database is empty.
        assert_eq!(db.get_aggregate_checkpoint_header(0).unwrap(), None);

        // Set an aggregate checkpoint header and check that it's retrievable.
        let header = random_aggregate_checkpoint_header(0);
        db.set_aggregate_checkpoint_header(0, header.clone())
            .unwrap();
        assert_eq!(
            db.get_aggregate_checkpoint_header(0).unwrap(),
            Some(header.clone())
        );

        // Set the same header again and check that it remains the same.
        db.set_aggregate_checkpoint_header(0, header.clone())
            .unwrap();
        assert_eq!(
            db.get_aggregate_checkpoint_header(0).unwrap(),
            Some(header.clone())
        );

        // Set the same epoch with a different header and check that it overwrites.
        let new_header = random_aggregate_checkpoint_header(0);
        db.set_aggregate_checkpoint_header(0, new_header.clone())
            .unwrap();
        assert_eq!(
            db.get_aggregate_checkpoint_header(0).unwrap(),
            Some(new_header.clone())
        );
        assert_ne!(new_header, header);

        // Set the header for a different epoch and check that it doesn't affect the previous
        // epoch.
        assert_eq!(db.get_aggregate_checkpoint_header(1).unwrap(), None);
        let header = random_aggregate_checkpoint_header(1);
        db.set_aggregate_checkpoint_header(1, header.clone())
            .unwrap();
        assert_eq!(
            db.get_aggregate_checkpoint_header(0).unwrap(),
            Some(new_header.clone())
        );
        assert_eq!(db.get_aggregate_checkpoint_header(1).unwrap(), Some(header));
    }
}
