use crate::db::{BlockData, DB};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{debug, info, warn};
use uuid::Uuid;

pub struct ChannelBuilder {
    db: Arc<Mutex<DB>>,
    pending_blocks: VecDeque<BlockData>,
    batch_size: u64,
}

impl ChannelBuilder {
    pub fn new(db: Arc<Mutex<DB>>, batch_size: u64) -> Self {
        if batch_size == 0 {
            warn!("Batch size is 0, defaulting to 1");
        }

        debug!("Creating ChannelBuilder with batch size: {}", batch_size);

        Self {
            db,
            pending_blocks: VecDeque::new(),
            batch_size: batch_size.max(1), // Ensure minimum batch size of 1
        }
    }

    pub fn db(&self) -> Arc<Mutex<DB>> {
        self.db.clone()
    }

    pub fn batch_size(&self) -> u64 {
        self.batch_size
    }

    pub fn pending_blocks(&self) -> &VecDeque<BlockData> {
        &self.pending_blocks
    }

    pub fn add_block(&mut self, block: BlockData) {
        debug!("Adding block {} to pending queue", block.block_number);
        self.pending_blocks.push_back(block);
    }

    pub fn clear_queue(&mut self) {
        let count = self.pending_blocks.len();
        self.pending_blocks.clear();
        debug!("Cleared {} blocks from pending queue", count);
    }

    // Creates a batch from the pending blocks and inserts it into the database
    pub fn insert_batch(&mut self) -> anyhow::Result<()> {
        if self.pending_blocks.is_empty() {
            warn!("Attempted to create batch with no pending blocks");
            return Ok(());
        }

        let db = self
            .db
            .lock()
            .map_err(|_| anyhow::anyhow!("Database lock poisoned"))?;

        // Batch data is a concat of all the block data
        let batch_data: Vec<Vec<u8>> = self
            .pending_blocks
            .iter()
            .map(|b| b.block_data.clone())
            .collect();

        let concatenated_data = batch_data.concat();
        let batch_data_json = serde_json::to_string(&concatenated_data)
            .map_err(|e| anyhow::anyhow!("Failed to serialize batch data: {}", e))?;

        let batch_id = Uuid::new_v4().to_string();
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| anyhow::anyhow!("System time error: {}", e))?
            .as_secs() as i64;

        let block_numbers: Vec<u64> = self.pending_blocks.iter().map(|b| b.block_number).collect();

        let block_numbers_json = serde_json::to_string(&block_numbers)
            .map_err(|e| anyhow::anyhow!("Failed to serialize block numbers: {}", e))?;

        // Create batch record
        db.conn()
            .execute(
                "INSERT INTO batches (id, block_numbers, data, created_at, status) 
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                (
                    &batch_id,
                    &block_numbers_json,
                    &batch_data_json,
                    current_time,
                    "Pending",
                ),
            )
            .map_err(|e| anyhow::anyhow!("Failed to insert batch into database: {}", e))?;

        info!(
            "Successfully created batch {} containing {} blocks ({})",
            batch_id,
            block_numbers.len(),
            block_numbers
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );

        debug!(
            "Batch {} data size: {} bytes",
            batch_id,
            batch_data_json.len()
        );

        Ok(())
    }
}
