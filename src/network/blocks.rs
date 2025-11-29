use alloy::{
    providers::{Provider, ProviderBuilder, WsConnect},
    primitives::U256,
};
use eyre::Result;
use futures_util::StreamExt;
use tokio::sync::mpsc;
use crate::config::Config;
use crate::types::MinedBlock;

pub struct BlockMonitor {
    config: Config,
}

impl BlockMonitor {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn spawn_monitor(
        self,
        tx_sender: mpsc::Sender<MinedBlock>,
    ) -> Result<()> {
        println!("= Connecting to WebSocket for block monitoring: {}", self.config.rpc_url);

        let ws = WsConnect::new(self.config.rpc_url);
        let provider = ProviderBuilder::new().on_ws(ws).await?;

        let sub = provider.subscribe_blocks().await?;
        let mut stream = sub.into_stream();

        println!("=@ Block Monitor Active. Waiting for new blocks...");

        while let Some(block) = stream.next().await {
            let block_number = block.header.number.unwrap_or_default();

            let mined_block = MinedBlock {
                number: block_number,
                timestamp: block.header.timestamp,
                base_fee: U256::from(block.header.base_fee_per_gas.unwrap_or_default()),
                tx_hashes: block.transactions.hashes().map(|h| h.to_string()).collect(),
                gas_used: block.header.gas_used,
                gas_limit: block.header.gas_limit,
            };

            println!("[NEW BLOCK] #{} | {} txs | Base Fee: {} gwei",
                mined_block.number,
                mined_block.tx_hashes.len(),
                mined_block.base_fee / U256::from(1_000_000_000u64)
            );

            if let Err(e) = tx_sender.send(mined_block).await {
                eprintln!("Failed to send block to channel: {:?}", e);
                break;
            }
        }

        Ok(())
    }
}
