use std::path::Path;

use hyperliquid_node_watcher::subscribe_hl_blocks;
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    env_logger::init();
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    let error_file = Path::new("error.log");
    tokio::spawn(subscribe_hl_blocks(tx));
    while let Some(block) = rx.recv().await {
        if let Ok(block) = block {
            log::info!("Parsed block: {:?}", block.height);
        } else {
            log::error!("{:?}", block);
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(error_file)
                .await?;
            file.write_all(format!("{:?}", block).as_bytes()).await?;
            file.flush().await?;
        }
    }
    Ok(())
}
