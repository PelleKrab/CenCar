  use rusqlite::{Connection, params};
  use std::sync::Arc;
  use tokio::sync::Mutex;
  use eyre::Result;
  use crate::types::{PendingTx, CensorshipEvent, MinedBlock, MempoolSnapshot};

  pub struct Repository {
      conn: Arc<Mutex<Connection>>,
  }

  impl Repository {
      pub async fn new(db_path: &str) -> Result<Self> {
          let conn = Connection::open(db_path)?;
          let repo = Self {
              conn: Arc::new(Mutex::new(conn)),
          };
          repo.init_schema().await?;
          Ok(repo)
      }

      async fn init_schema(&self) -> Result<()> {
          let conn = self.conn.lock().await;

          // Create transactions table
          conn.execute(
              "CREATE TABLE IF NOT EXISTS transactions (
                  id INTEGER PRIMARY KEY AUTOINCREMENT,
                  tx_hash TEXT NOT NULL UNIQUE,
                  from_address TEXT NOT NULL,
                  to_address TEXT,
                  max_priority_fee TEXT NOT NULL,
                  max_fee TEXT NOT NULL,
                  nonce INTEGER NOT NULL,
                  gas_limit INTEGER NOT NULL,
                  value TEXT NOT NULL,
                  input_data_size INTEGER NOT NULL,
                  first_seen INTEGER NOT NULL,
                  status TEXT NOT NULL,
                  included_in_block INTEGER,
                  last_updated INTEGER NOT NULL,
                  CHECK(status IN ('pending', 'included', 'dropped', 'censored'))
              )",
              [],
          )?;

          // Create censorship events table
          conn.execute(
              "CREATE TABLE IF NOT EXISTS censorship_events (
                  id INTEGER PRIMARY KEY AUTOINCREMENT,
                  tx_hash TEXT NOT NULL,
                  from_address TEXT NOT NULL,
                  to_address TEXT,
                  priority_fee TEXT NOT NULL,
                  threshold_fee TEXT NOT NULL,
                  fee_percentile REAL NOT NULL,
                  blocks_pending INTEGER NOT NULL,
                  seconds_pending INTEGER NOT NULL,
                  confidence_score REAL NOT NULL,
                  detected_at_block INTEGER NOT NULL,
                  detected_at INTEGER NOT NULL,
                  FOREIGN KEY(tx_hash) REFERENCES transactions(tx_hash)
              )",
              [],
          )?;

          // Create blocks table
          conn.execute(
              "CREATE TABLE IF NOT EXISTS blocks (
                  block_number INTEGER PRIMARY KEY,
                  timestamp INTEGER NOT NULL,
                  base_fee TEXT NOT NULL,
                  gas_used INTEGER NOT NULL,
                  gas_limit INTEGER NOT NULL,
                  tx_count INTEGER NOT NULL,
                  created_at INTEGER NOT NULL
              )",
              [],
          )?;

          // Create mempool snapshots table
          conn.execute(
              "CREATE TABLE IF NOT EXISTS mempool_snapshots (
                  id INTEGER PRIMARY KEY AUTOINCREMENT,
                  timestamp INTEGER NOT NULL,
                  block_number INTEGER,
                  p25_fee TEXT NOT NULL,
                  p50_fee TEXT NOT NULL,
                  p75_fee TEXT NOT NULL,
                  p90_fee TEXT NOT NULL,
                  tx_count INTEGER NOT NULL
              )",
              [],
          )?;

          // Create indexes
          conn.execute(
              "CREATE INDEX IF NOT EXISTS idx_tx_status ON transactions(status)",
              [],
          )?;
          conn.execute(
              "CREATE INDEX IF NOT EXISTS idx_tx_first_seen ON transactions(first_seen)",
              [],
          )?;
          conn.execute(
              "CREATE INDEX IF NOT EXISTS idx_censorship_detected_at ON censorship_events(detected_at)",
              [],
          )?;

          Ok(())
      }

      pub async fn insert_transaction(&self, tx: &PendingTx) -> Result<()> {
          let conn = self.conn.lock().await;

          conn.execute(
              "INSERT OR IGNORE INTO transactions (
                  tx_hash, from_address, to_address, max_priority_fee, max_fee,
                  nonce, gas_limit, value, input_data_size, first_seen,
                  status, last_updated
              ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
              params![
                  tx.hash,
                  tx.from.to_string(),
                  tx.to.map(|a| a.to_string()),
                  tx.max_priority_fee.to_string(),
                  tx.max_fee.to_string(),
                  tx.nonce,
                  tx.gas_limit,
                  tx.value.to_string(),
                  tx.input_data_size,
                  tx.first_seen,
                  "pending",
                  tx.first_seen,
              ],
          )?;

          Ok(())
      }

      pub async fn update_tx_status(
          &self,
          hash: &str,
          status: &str,
          block: Option<u64>,
      ) -> Result<()> {
          let conn = self.conn.lock().await;
          let now = std::time::SystemTime::now()
              .duration_since(std::time::UNIX_EPOCH)?
              .as_secs() as i64;

          conn.execute(
              "UPDATE transactions
               SET status = ?1, included_in_block = ?2, last_updated = ?3
               WHERE tx_hash = ?4",
              params![status, block, now, hash],
          )?;

          Ok(())
      }

      pub async fn insert_censorship_event(&self, event: &CensorshipEvent) -> Result<()> {
          let conn = self.conn.lock().await;

          conn.execute(
              "INSERT INTO censorship_events (
                  tx_hash, from_address, to_address, priority_fee, threshold_fee,
                  fee_percentile, blocks_pending, seconds_pending, confidence_score,
                  detected_at_block, detected_at
              ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
              params![
                  event.tx_hash,
                  event.from.to_string(),
                  event.to.map(|a| a.to_string()),
                  event.priority_fee.to_string(),
                  event.threshold_fee.to_string(),
                  event.fee_percentile,
                  event.blocks_pending,
                  event.seconds_pending,
                  event.confidence_score,
                  event.detected_at_block,
                  event.detected_at,
              ],
          )?;

          // Also update the transaction status to 'censored'
          self.update_tx_status(&event.tx_hash, "censored", None).await?;

          Ok(())
      }

      pub async fn insert_block(&self, block: &MinedBlock) -> Result<()> {
          let conn = self.conn.lock().await;
          let now = std::time::SystemTime::now()
              .duration_since(std::time::UNIX_EPOCH)?
              .as_secs() as i64;

          conn.execute(
              "INSERT OR REPLACE INTO blocks (
                  block_number, timestamp, base_fee, gas_used, gas_limit,
                  tx_count, created_at
              ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
              params![
                  block.number,
                  block.timestamp,
                  block.base_fee.to_string(),
                  block.gas_used,
                  block.gas_limit,
                  block.tx_hashes.len(),
                  now,
              ],
          )?;

          Ok(())
      }

      pub async fn insert_snapshot(
          &self,
          snapshot: &MempoolSnapshot,
          block: u64,
      ) -> Result<()> {
          let conn = self.conn.lock().await;

          conn.execute(
              "INSERT INTO mempool_snapshots (
                  timestamp, block_number, p25_fee, p50_fee, p75_fee, p90_fee, tx_count
              ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
              params![
                  snapshot.timestamp,
                  block,
                  snapshot.fee_percentiles.p25.to_string(),
                  snapshot.fee_percentiles.p50.to_string(),
                  snapshot.fee_percentiles.p75.to_string(),
                  snapshot.fee_percentiles.p90.to_string(),
                  snapshot.tx_count,
              ],
          )?;

          Ok(())
      }

      pub async fn cleanup_old_data(&self, retention_days: i64) -> Result<()> {
          let conn = self.conn.lock().await;
          let cutoff = std::time::SystemTime::now()
              .duration_since(std::time::UNIX_EPOCH)?
              .as_secs() as i64
              - (retention_days * 86400);

          // Delete old non-censored transactions
          conn.execute(
              "DELETE FROM transactions WHERE status != 'censored' AND last_updated < ?1",
              params![cutoff],
          )?;

          // Delete old blocks
          conn.execute(
              "DELETE FROM blocks WHERE created_at < ?1",
              params![cutoff],
          )?;

          Ok(())
      }
  }

