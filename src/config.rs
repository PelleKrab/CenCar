use std::env;
use eyre::{Result, eyre};
use dotenv::dotenv;

#[derive(Clone)]
pub struct Config {
    pub rpc_url: String,
    pub db_path: String,
    pub fee_percentile_threshold: f64,
    pub min_pending_blocks: u64,
    pub min_pending_seconds: i64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // Load .env file
        dotenv().ok();

        let rpc_url = env::var("RPC_URL")
            .map_err(|_| eyre!("RPC_URL must be set in .env file"))?;

        let db_path = env::var("DB_PATH")
            .unwrap_or_else(|_| "censorship.db".to_string());

        let fee_percentile_threshold = env::var("FEE_PERCENTILE_THRESHOLD")
            .unwrap_or_else(|_| "0.25".to_string())
            .parse()
            .map_err(|_| eyre!("FEE_PERCENTILE_THRESHOLD must be a valid f64"))?;

        let min_pending_blocks = env::var("MIN_PENDING_BLOCKS")
            .unwrap_or_else(|_| "3".to_string())
            .parse()
            .map_err(|_| eyre!("MIN_PENDING_BLOCKS must be a valid u64"))?;

        let min_pending_seconds = env::var("MIN_PENDING_SECONDS")
            .unwrap_or_else(|_| "60".to_string())
            .parse()
            .map_err(|_| eyre!("MIN_PENDING_SECONDS must be a valid i64"))?;

        Ok(Config {
            rpc_url,
            db_path,
            fee_percentile_threshold,
            min_pending_blocks,
            min_pending_seconds,
        })
    }
}
