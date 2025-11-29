use alloy::{
    consensus::Transaction,
    primitives::U256,
    providers::{Provider, ProviderBuilder, WsConnect},
};
use eyre::Result;
use futures_util::StreamExt;
use tokio::sync::mpsc;
use crate::config::Config;
use crate::types::PendingTx;

pub async fn spawn_monitor(
    config: Config,
    tx_sender: mpsc::Sender<PendingTx>,
) -> Result<()> {
    println!("🔌 Connecting to WebSocket at: {}", config.rpc_url);

    // 1. Establish the WebSocket connection
    let ws = WsConnect::new(config.rpc_url);
    let provider = ProviderBuilder::new().on_ws(ws).await?;

    // 2. Subscribe to the 'newPendingTransactions' stream
    let sub = provider.subscribe_pending_transactions().await?;
    let mut stream = sub.into_stream();

    println!("👀 Mempool Monitor Active. Waiting for transactions...");

    while let Some(tx_hash) = stream.next().await {
        if let Ok(Some(tx)) = provider.get_transaction_by_hash(tx_hash).await {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            let simple_tx = PendingTx {
                hash: tx_hash.to_string(),
                from: tx.from,
                to: tx.inner.to(),
                max_priority_fee: U256::from(tx.inner.max_priority_fee_per_gas().unwrap_or_default()),
                max_fee: U256::from(tx.inner.max_fee_per_gas()),
                nonce: tx.inner.nonce(),
                gas_limit: tx.inner.gas_limit(),
                value: tx.inner.value(),
                input_data_size: tx.inner.input().len(),
                first_seen: now,
            };

            if let Err(e) = tx_sender.send(simple_tx).await {
                eprintln!("Failed to send transaction to channel: {:?}", e);
                break;
            }
        }
    }

    Ok(())
}
