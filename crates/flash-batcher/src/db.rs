use std::fmt::Display;

use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use serde_json;
use tracing::{debug, error, info, warn};

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
    pub fn new(file_path: &str) -> Result<Self> {
        let conn = Connection::open(file_path).map_err(|e| {
            error!("Failed to open database at {}: {}", file_path, e);
            e
        })?;

        info!("Database connection established: {}", file_path);

        Ok(Self { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn initialize_database(&self) -> Result<()> {
        debug!("Initializing database schema...");

        self.conn
            .execute(
                "CREATE TABLE IF NOT EXISTS batches (
                id TEXT PRIMARY KEY,
                block_numbers TEXT NOT NULL,
                data TEXT NOT NULL,  
                created_at INTEGER NOT NULL,
                submitted_at INTEGER,
                celestia_height INTEGER,
                retry_count INTEGER DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'Pending'
            )",
                [],
            )
            .map_err(|e| {
                error!("Failed to create batches table: {}", e);
                e
            })?;

        info!("Database schema initialized successfully");
        Ok(())
    }

    pub fn get_pending_batches(&self) -> Result<Vec<BatchInfo>> {
        debug!("Fetching pending batches from database...");

        let mut stmt = self
            .conn
            .prepare("SELECT * FROM batches WHERE status = 'Pending' ORDER BY created_at ASC")
            .map_err(|e| {
                error!("Failed to prepare pending batches query: {}", e);
                e
            })?;

        let batches = stmt
            .query_map([], |row| {
                let block_numbers_str: String = row.get(1)?;
                let status_str: String = row.get(7)?;
                let data_str: String = row.get(2)?;

                let block_numbers = match serde_json::from_str(&block_numbers_str) {
                    Ok(nums) => nums,
                    Err(e) => {
                        error!(
                            "Failed to deserialize block numbers for batch {}: {}",
                            row.get::<_, String>(0).unwrap_or_default(),
                            e
                        );
                        Vec::new()
                    }
                };

                let data = match serde_json::from_str(&data_str) {
                    Ok(d) => d,
                    Err(e) => {
                        error!(
                            "Failed to deserialize batch data for batch {}: {}",
                            row.get::<_, String>(0).unwrap_or_default(),
                            e
                        );
                        Vec::new()
                    }
                };

                let status = match status_str.as_str() {
                    "Pending" => BatchStatus::Pending,
                    "Submitting" => BatchStatus::Submitting,
                    "Submitted" => BatchStatus::Submitted,
                    "Failed" => BatchStatus::Failed,
                    unknown => {
                        warn!("Unknown batch status '{}', defaulting to Pending", unknown);
                        BatchStatus::Pending
                    }
                };

                Ok(BatchInfo {
                    id: row.get(0)?,
                    block_numbers,
                    data,
                    created_at: row.get(3)?,
                    submitted_at: row.get(4)?,
                    celestia_height: row.get(5)?,
                    retry_count: row.get(6)?,
                    status,
                })
            })
            .map_err(|e| {
                error!("Failed to execute pending batches query: {}", e);
                e
            })?;

        let result: Result<Vec<BatchInfo>> = batches.collect();

        match &result {
            Ok(batches) => debug!("Successfully fetched {} pending batches", batches.len()),
            Err(e) => error!("Failed to collect pending batches: {}", e),
        }

        result
    }

    pub fn update_batch_status(&self, batch_id: &str, status: BatchStatus) -> Result<()> {
        debug!("Updating batch {} status to {}", batch_id, status);

        let rows_affected = self
            .conn
            .execute(
                "UPDATE batches SET status = ? WHERE id = ?",
                (status.to_string(), batch_id),
            )
            .map_err(|e| {
                error!("Failed to update batch status for {}: {}", batch_id, e);
                e
            })?;

        if rows_affected == 0 {
            warn!("No batch found with id: {}", batch_id);
        } else {
            debug!("Successfully updated status for batch: {}", batch_id);
        }

        Ok(())
    }

    pub fn get_batch_count_by_status(&self, status: BatchStatus) -> Result<u32> {
        let count: u32 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM batches WHERE status = ?",
                [status.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| {
                error!("Failed to count batches with status {}: {}", status, e);
                e
            })?;

        debug!("Found {} batches with status {}", count, status);
        Ok(count)
    }
}
