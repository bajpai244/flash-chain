use rusqlite::Connection;
use std::{collections::VecDeque, sync::Arc};
use tokio::{sync::Mutex, task::JoinHandle};

use crate::db::{BlockData, initialize_database};

pub struct BatcherExEx {
    db: Arc<Mutex<Connection>>,
    pending_blocks: VecDeque<BlockData>,
    batch_submitter_handle: Option<JoinHandle<()>>,
}

impl BatcherExEx {
    pub async fn new() -> anyhow::Result<Self> {
        let db = Arc::new(Mutex::new(initialize_database()?));

        Ok(Self {
            db,
            pending_blocks: VecDeque::new(),
            batch_submitter_handle: None,
        })
    }
}
