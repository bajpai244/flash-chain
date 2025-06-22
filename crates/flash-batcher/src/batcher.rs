use rusqlite::Connection;
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::Mutex;

use crate::db::{BlockData, initialize_database};

pub struct Batcher {
    db: Arc<Mutex<Connection>>,
    pending_blocks: VecDeque<BlockData>,
    batch_size: u64,
}

impl Batcher {
    pub async fn new(db: Arc<Mutex<Connection>>, batch_size: u64) -> anyhow::Result<Self> {
        Ok(Self {
            db,
            pending_blocks: VecDeque::new(),
            batch_size,
        })
    }
}
