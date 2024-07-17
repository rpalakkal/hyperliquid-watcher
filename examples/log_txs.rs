use std::path::PathBuf;

use hyperliquid_node_watcher::subscribe_hl_blocks;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    env_logger::init();
    let path = PathBuf::from(std::env::args().nth(1).unwrap());
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    tokio::spawn(subscribe_hl_blocks(path, tx));
    while let Some(block) = rx.recv().await {
        if let Ok(block) = block {
            log::info!("Parsed block: {:?}", block.height);
        } else {
            log::error!("{:?}", block);
        }
    }
    Ok(())
}
