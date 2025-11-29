use alloy::primitives::{U256, Address};

// Status of a tracked t transaction
#[derive(Debug, Clone, PartialEq)]
pub enum TxStatus {
    Pending,
    Included { block_number: u64 },
    Dropped,
    PotentiallyCensored,
}

// Pending transaction data
#[derive(Debug, Clone)]
pub struct PendingTx {
    pub hash: String,
    pub from: Address,
    pub to: Option<Address>,
    pub max_priority_fee: U256,
    pub max_fee: U256,
    pub nonce: u64,
    pub first_seen: i64,
    pub gas_limit: u64,
    pub value: U256,
    pub input_data_size: usize,
}

// Transaction wrapper with tracking metadata
#[derive(Debug, Clone)]
pub struct TrackedTx {
    pub tx: PendingTx,
    pub status: TxStatus,
    pub last_checked: i64,
}

// Fee percentiles for mempool analysis
#[derive(Debug, Clone)]
pub struct FeePercentiles {
    pub p25: U256,  // 25th percentile
    pub p50: U256,  // Median (50th percentile)
    pub p75: U256,  // 75th percentile
    pub p90: U256,  // 90th percentile
}

// Snapshot of mempool state at a point in time
#[derive(Debug, Clone)]
pub struct MempoolSnapshot {
    pub timestamp: i64,
    pub fee_percentiles: FeePercentiles,
    pub tx_count: usize,
}

// Block data for correlation with pending transactions
#[derive(Debug, Clone)]
pub struct MinedBlock {
    pub number: u64,
    pub timestamp: u64,
    pub base_fee: U256,
    pub tx_hashes: Vec<String>,
    pub gas_used: u128,
    pub gas_limit: u128,
}

// Detected censorship event
#[derive(Debug, Clone)]
pub struct CensorshipEvent {
    pub tx_hash: String,
    pub from: Address,
    pub to: Option<Address>,
    pub priority_fee: U256,
    pub threshold_fee: U256,
    pub fee_percentile: f64,
    pub blocks_pending: u64,
    pub seconds_pending: i64,
    pub confidence_score: f64,
    pub detected_at_block: u64,
    pub detected_at: i64,
}
