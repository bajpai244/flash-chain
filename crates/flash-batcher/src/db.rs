use std::fmt::Display;

use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use serde_json;

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

impl Display for BatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BatchStatus::Pending => write!(f, "Pending"),
            BatchStatus::Submitting => write!(f, "Submitting"),
            BatchStatus::Submitted => write!(f, "Submitted"),
            BatchStatus::Failed => write!(f, "Failed"),
        }
    }
}

pub struct DB {
    conn: Connection,
}

impl DB {
    pub fn new(file_name: &str) -> Self {
        let conn = Connection::open(file_name).unwrap();
        Self { conn }
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn initialize_database(&self) -> Result<()> {
        self.conn.execute(
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

        Ok(())
    }

    pub fn get_pending_batches(&self) -> Result<Vec<BatchInfo>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM batches WHERE status = 'Pending'")?;

        let batches = stmt.query_map([], |row| {
            let block_numbers_str: String = row.get(1)?;
            let status_str: String = row.get(7)?;
            let data: String = row.get(2)?;

            Ok(BatchInfo {
                id: row.get(0)?,
                block_numbers: serde_json::from_str(&block_numbers_str).unwrap_or_default(),
                data: serde_json::from_str(&data).unwrap_or_default(),
                created_at: row.get(3)?,
                submitted_at: row.get(4)?,
                celestia_height: row.get(5)?,
                retry_count: row.get(6)?,
                status: match status_str.as_str() {
                    "Pending" => BatchStatus::Pending,
                    "Submitting" => BatchStatus::Submitting,
                    "Submitted" => BatchStatus::Submitted,
                    "Failed" => BatchStatus::Failed,
                    _ => BatchStatus::Pending,
                },
            })
        })?;

        println!("batch extracted correctly");

        Ok(batches.collect::<Result<Vec<BatchInfo>>>()?)
    }

    pub fn update_batch_status(&self, batch_id: &str, status: BatchStatus) -> Result<()> {
        self.conn.execute(
            "UPDATE batches SET status = ? WHERE id = ?",
            (status.to_string(), batch_id),
        )?;
        Ok(())
    }
}
