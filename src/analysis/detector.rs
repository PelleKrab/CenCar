use std::sync::Arc;
use std::collections::HashMap;
use alloy::primitives::U256;
use crate::config::Config;
use crate::network::state::MempoolState;
use crate::db::repo::Repository;
use crate::types::{CensorshipEvent, TrackedTx, MempoolSnapshot};

pub struct CensorshipDetector {
    mempool_state: Arc<MempoolState>,
    db: Arc<Repository>,
    config: Config,
    block_first_seen: Arc<tokio::sync::RwLock<HashMap<String, u64>>>,
}

impl CensorshipDetector {
    pub fn new(
        mempool_state: Arc<MempoolState>,
        db: Arc<Repository>,
        config: Config,
    ) -> Self {
        Self {
            mempool_state,
            db,
            config,
            block_first_seen: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    pub async fn scan_mempool(&self, current_block: u64) -> Vec<CensorshipEvent> {
        let snapshot = self.mempool_state.calculate_snapshot().await;
        let pending_txs = self.mempool_state.get_pending_txs().await;

        let mut events = Vec::new();

        for tracked_tx in pending_txs {
            if let Some(event) = self.analyze_transaction(&tracked_tx, current_block, &snapshot).await {
                events.push(event);
            }
        }

        events
    }

    async fn analyze_transaction(
        &self,
        tracked_tx: &TrackedTx,
        current_block: u64,
        snapshot: &MempoolSnapshot,
    ) -> Option<CensorshipEvent> {
        let current_time = current_timestamp();
        let time_in_mempool = current_time - tracked_tx.tx.first_seen;

        let first_seen_block = {
            let mut map = self.block_first_seen.write().await;
            *map.entry(tracked_tx.tx.hash.clone()).or_insert(current_block)
        };

        let blocks_waited = current_block.saturating_sub(first_seen_block);

        let tx_priority_fee = tracked_tx.tx.max_priority_fee;
        let threshold_fee = snapshot.fee_percentiles.p25;

        if threshold_fee == U256::ZERO {
            return None;
        }

        let has_competitive_fee = tx_priority_fee >= threshold_fee;
        let waited_long_enough = blocks_waited >= self.config.min_pending_blocks
            && time_in_mempool >= self.config.min_pending_seconds;

        if !has_competitive_fee || !waited_long_enough {
            return None;
        }

        let fee_ratio = if threshold_fee > U256::ZERO {
            tx_priority_fee.to::<u128>() as f64 / threshold_fee.to::<u128>() as f64
        } else {
            1.0
        };

        let min_fee_ratio = 1.0;
        if fee_ratio < min_fee_ratio {
            return None;
        }

        let time_score = (blocks_waited as f64 / 10.0).min(1.0);
        let confidence_score = (fee_ratio * time_score * 0.5).min(1.0);

        let fee_percentile = self.calculate_percentile(tx_priority_fee, snapshot);

        Some(CensorshipEvent {
            tx_hash: tracked_tx.tx.hash.clone(),
            from: tracked_tx.tx.from,
            to: tracked_tx.tx.to,
            priority_fee: tx_priority_fee,
            threshold_fee,
            fee_percentile,
            blocks_pending: blocks_waited,
            seconds_pending: time_in_mempool,
            confidence_score,
            detected_at_block: current_block,
            detected_at: current_time,
        })
    }

    fn calculate_percentile(&self, fee: U256, snapshot: &MempoolSnapshot) -> f64 {
        if fee >= snapshot.fee_percentiles.p90 {
            0.90
        } else if fee >= snapshot.fee_percentiles.p75 {
            0.75
        } else if fee >= snapshot.fee_percentiles.p50 {
            0.50
        } else if fee >= snapshot.fee_percentiles.p25 {
            0.25
        } else {
            0.10
        }
    }
}

fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
