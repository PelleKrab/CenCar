use alloy::{
    providers::{Provider, ProviderBuilder, WsConnect},
    primitives::U256,
    rpc::types::BlockTransactionsKind,
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

        while let Some(block_header) = stream.next().await {
            let block_number = block_header.inner.number;

            // Fetch full block to get transaction hashes
            let tx_hashes = if let Ok(Some(full_block)) = provider.get_block_by_number(block_number.into(), BlockTransactionsKind::Hashes).await {
                full_block.transactions.hashes().map(|h| h.to_string()).collect()
            } else {
                Vec::new()
            };

            let mined_block = MinedBlock {
                number: block_number,
                timestamp: block_header.inner.timestamp,
                base_fee: U256::from(block_header.inner.base_fee_per_gas.unwrap_or_default()),
                tx_hashes,
                gas_used: block_header.inner.gas_used as u128,
                gas_limit: block_header.inner.gas_limit as u128,
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
