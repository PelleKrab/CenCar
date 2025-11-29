use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use alloy::primitives::U256;
use crate::types::{PendingTx, TrackedTx, TxStatus, MempoolSnapshot, FeePercentiles};

pub struct MempoolState {
    tracked_txs: Arc<RwLock<HashMap<String, TrackedTx>>>,
    fee_distribution: Arc<RwLock<Vec<U256>>>,
}

impl MempoolState {
    pub fn new() -> Self {
        Self {
            tracked_txs: Arc::new(RwLock::new(HashMap::new())),
            fee_distribution: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn add_tx(&self, tx: PendingTx) {
        let mut tracked = self.tracked_txs.write().await;
        let mut fees = self.fee_distribution.write().await;

        let tracked_tx = TrackedTx {
            tx: tx.clone(),
            status: TxStatus::Pending,
            last_checked: current_timestamp(),
        };

        tracked.insert(tx.hash.clone(), tracked_tx);
        fees.push(tx.max_priority_fee);
    }

    pub async fn mark_included_txs(&self, tx_hashes: &[String]) {
        let mut tracked = self.tracked_txs.write().await;

        for hash in tx_hashes {
            if let Some(tracked_tx) = tracked.get_mut(hash) {
                if let TxStatus::Pending = tracked_tx.status {
                    tracked_tx.status = TxStatus::Included {
                        block_number: 0, // Will be updated by detector
                    };
                    tracked_tx.last_checked = current_timestamp();
                }
            }
        }
    }

    pub async fn calculate_snapshot(&self) -> MempoolSnapshot {
        let fees = self.fee_distribution.read().await;
        let tracked = self.tracked_txs.read().await;

        let mut sorted_fees: Vec<U256> = fees.clone();
        sorted_fees.sort();

        let percentiles = if sorted_fees.is_empty() {
            FeePercentiles {
                p25: U256::ZERO,
                p50: U256::ZERO,
                p75: U256::ZERO,
                p90: U256::ZERO,
            }
        } else {
            let len = sorted_fees.len();
            FeePercentiles {
                p25: sorted_fees[len * 25 / 100],
                p50: sorted_fees[len * 50 / 100],
                p75: sorted_fees[len * 75 / 100],
                p90: sorted_fees[len * 90 / 100],
            }
        };

        MempoolSnapshot {
            timestamp: current_timestamp(),
            fee_percentiles: percentiles,
            tx_count: tracked.len(),
        }
    }

    pub async fn get_pending_txs(&self) -> Vec<TrackedTx> {
        let tracked = self.tracked_txs.read().await;
        tracked
            .values()
            .filter(|tx| matches!(tx.status, TxStatus::Pending))
            .cloned()
            .collect()
    }

    pub async fn cleanup_old_txs(&self, max_age_secs: i64) {
        let mut tracked = self.tracked_txs.write().await;
        let mut fees = self.fee_distribution.write().await;

        let cutoff = current_timestamp() - max_age_secs;

        tracked.retain(|_, tx| {
            matches!(tx.status, TxStatus::Pending | TxStatus::PotentiallyCensored)
                || tx.last_checked > cutoff
        });

        fees.clear();
        for tx in tracked.values() {
            if matches!(tx.status, TxStatus::Pending) {
                fees.push(tx.tx.max_priority_fee);
            }
        }
    }

    pub async fn get_tx_count(&self) -> usize {
        let tracked = self.tracked_txs.read().await;
        tracked.len()
    }
}

fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
