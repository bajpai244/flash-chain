use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockData {
    pub block_number: u64,
    pub block_hash: String,
    pub block_data: Vec<u8>,
    pub timestamp: u64,
    pub batch_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchInfo {
    pub id: String,
    pub block_numbers: Vec<u64>,
    pub data: Vec<u8>,
    pub created_at: i64,
    pub submitted_at: Option<i64>,
    pub celestia_height: Option<u64>,
    pub retry_count: u32,
    pub status: BatchStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum BatchStatus {
    Pending,
    Submitting,
    Submitted,
    Failed,
}

pub fn initialize_database() -> Result<Connection> {
    let conn = Connection::open("batcher.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS batches (
            id TEXT PRIMARY KEY,
            block_numbers TEXT NOT NULL,
            data BLOB NOT NULL,
            created_at INTEGER NOT NULL,
            submitted_at INTEGER,
            celestia_height INTEGER,
            retry_count INTEGER DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'Pending'
        )",
        [],
    )?;

    Ok(conn)
}
