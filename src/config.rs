use std::env;
use eyre::{Result, eyre};
use dotenv::dotenv;

#[derive(Clone)]
pub struct Config {
    pub rpc_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // Load .env file
        dotenv().ok();

        let rpc_url = env::var("RPC_URL")
            .map_err(|_| eyre!("RPC_URL must be set in .env file"))?;

        Ok(Config { rpc_url })
    }
}
