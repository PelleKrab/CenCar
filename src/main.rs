pub mod config;
pub mod types;
pub mod network;
pub mod db;
pub mod analysis;

use config::Config;
use eyre::Result;
use tokio::sync::mpsc;
use types::PendingTx;

#[tokio::main]
async fn main() -> Result<()>{
    let config = Config::from_env()?;

    // TODO: Implement full channel-based architecture
    // See the plan for details on how to wire everything together

    // Minimal stub to make it compile
    let (tx_sender, mut tx_receiver) = mpsc::channel::<PendingTx>(1000);

    let mempool_handle = tokio::spawn({
        let config = config.clone();
        async move {
            println!("   ↳ Spawning Mempool Watcher...");
            if let Err(e) = network::mempool::spawn_monitor(config, tx_sender).await {
                eprintln!("❌ Mempool Monitor Failed: {:?}", e);
            }
        }
    });

    // Receive and print transactions (temporary)
    let receiver_handle = tokio::spawn(async move {
        while let Some(tx) = tx_receiver.recv().await {
            println!("[NEW TX] Hash: {} | Priority Fee: {} wei", tx.hash, tx.max_priority_fee);
        }
    });

    tokio::try_join!(mempool_handle, receiver_handle)?;

    Ok(())

}
