use std::path::PathBuf;

use hyperliquid_node_watcher::subscribe_hl_blocks;

fn main() -> eyre::Result<()> {
    env_logger::init();
    let path = PathBuf::from(std::env::args().nth(1).unwrap());
    let (tx, rx) = std::sync::mpsc::channel();
    let _data_feed = std::thread::spawn(|| subscribe_hl_blocks(path, tx));
    while let Ok(i) = rx.recv() {
        log::info!("Block: {:?}", i.height);
        for tx in i.txs() {
            log::info!("tx from: {:?}", tx.sender());
        }
        log::info!("");
    }
    Ok(())
}
