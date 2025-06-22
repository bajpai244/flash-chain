use crate::db::BlockData;
use rusqlite::Connection;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
use tracing::info;
use uuid::Uuid;

pub struct Batcher {
    db: Arc<Mutex<Connection>>,
    pending_blocks: VecDeque<BlockData>,
    batch_size: u64,
}

impl Batcher {
    pub fn new(db: Arc<Mutex<Connection>>, batch_size: u64) -> Self {
        Self {
            db,
            pending_blocks: VecDeque::new(),
            batch_size,
        }
    }

    pub fn batch_size(&self) -> u64 {
        self.batch_size
    }

    pub fn pending_blocks(&self) -> &VecDeque<BlockData> {
        &self.pending_blocks
    }

    pub fn add_block(&mut self, block: BlockData) {
        self.pending_blocks.push_back(block);
    }

    pub fn clear_queue(&mut self) {
        self.pending_blocks.clear();
    }

    // creates a batch from the pending blocks and inserts it into the database
    pub fn insert_batch(&mut self) -> anyhow::Result<()> {
        let conn = self.db.lock().unwrap();

        // batch data is a concat of all the block data
        let batch_data: Vec<Vec<u8>> = self
            .pending_blocks
            .iter()
            .map(|b| b.block_data.clone())
            .collect();
        let batch_data = serde_json::to_string(&batch_data.concat())?;

        let batch_id = Uuid::new_v4().to_string();
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        let block_numbers = self
            .pending_blocks
            .iter()
            .map(|b| b.block_number)
            .collect::<Vec<u64>>();
        let block_numbers = serde_json::to_string(&block_numbers)?;

        // Create batch record
        conn.execute(
            "INSERT INTO batches (id, block_numbers, data, created_at, status) 
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                &batch_id,
                block_numbers,
                batch_data,
                current_time,
                "Pending",
            ),
        )?;

        info!(
            "Inserted batch {} with block numbers {:?}",
            batch_id,
            self.pending_blocks
                .iter()
                .map(|b| b.block_number)
                .collect::<Vec<u64>>()
        );

        Ok(())
    }
}
