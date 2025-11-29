pub mod config;
pub mod types;
pub mod network;
pub mod db;
pub mod analysis;

use config::Config;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()>{
    let config = Config::from_env()?;
    let config_mempool = config.clone();

    
     let mempool_handle = tokio::spawn(async move {
        println!("   ↳ Spawning Mempool Watcher...");
        if let Err(e) = network::mempool::spawn_monitor(config_mempool).await {
            eprintln!("❌ Mempool Monitor Failed: {:?}", e);
        }
    });   


    mempool_handle.await?;

    Ok(())
    
}
