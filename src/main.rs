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

    // TODO: Implement full channel-based architecture
    // See the plan for details on how to wire everything together

    // Minimal stub to make it compile
    // let (tx_sender, mut tx_receiver) = mpsc::channel::<PendingTx>(1000);
    //
    // let mempool_handle = tokio::spawn({
    //     let config = config.clone();
    //     async move {
    //         println!("   ↳ Spawning Mempool Watcher...");
    //         if let Err(e) = network::mempool::spawn_monitor(config, tx_sender).await {
    //             eprintln!("❌ Mempool Monitor Failed: {:?}", e);
    //         }
    //     }
    // });
    //
    // // Receive and print transactions (temporary)
    // let receiver_handle = tokio::spawn(async move {
    //     while let Some(tx) = tx_receiver.recv().await {
    //         println!("[NEW TX] Hash: {} | Priority Fee: {} wei", tx.hash, tx.max_priority_fee);
    //     }
    // });
    
    let config = Config::from_env()?;

    // Init DB
    let db = Arc::new(Repository::new(&config.db_path).await?);
    let mempool_state = Arc::new(MempoolState::new());
    let detector = Arc::new(CensorshipDetector::new(
        mempool_state.clone(),
        db.clone(),
        config.clone(),
    ));

    // Spawn channels
    let (tx_sender, tx_receiver) = mpsc::channel::<PendingTx>(1000);
    let (block_sender, block_receiver) = mpsc::channel::<MinedBlock>(100);

    let mempool_handle = tokio::spawn({
        let config = config.clone();
        async move {
            network::mempool::spawn_monitor(config, tx_sender).await
        }
    });

    let block_haandle = tokio::spawn({
        let config = config.clone();
        async move {
            let mut block_monitor = network::BlockMonitor::new(config);
            block_monitor.spawn_monitor(tx_sender).await
        }
    });

    let tx_processor = tokio::spawn({
        let db = db.clone();
        let mem_state = mempool_state.clone();
        let rx = tx_receiver;
        async move {
            while let Some(tx) = rx.recv().await{
                mem_state.add_tx(tx.clone()).await;

                let _ = db.insert_transaction(tx).await;
            }
        }
    });

    let block_processor = tokio::spawn({
        let detector = detector.clone();
        let mem_state = mempool_state.clone();
        let db = db.clone();
        let mut rx = block_receiver;

        async move {
            while let Some(block) = rx.recv().await{
                // 1. New block arrived
                println("New block recieved: #{:?}", block.number);
                let _ = db.insert_block(&block).await;

                // 2. Update state
                mem_state.mark_included_txs(&block.tx_hashes).await;
            }

            // 3. RUN DETECTION
            let events = detector.scan_mempool(block.number).await;

            let snapshoot = self.mem_state.calculate_snapshoot().await;

            let pending_txs = self.mempool_state.get_pending_txs().await;

            
            
            // 4. Store results

        }

    });
     
    tokio::try_join!(
          mempool_handle,
          block_handle,
          tx_processor,
          block_processor,
          cleanup_handle,
      )?;

    Ok(())

}
