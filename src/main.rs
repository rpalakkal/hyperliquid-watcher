use std::path::PathBuf;
mod parse;

fn main() -> eyre::Result<()> {
    env_logger::init();
    let path = PathBuf::from(std::env::args().nth(1).unwrap());
    let (tx, rx) = std::sync::mpsc::channel();
    let _data_feed = std::thread::spawn(|| parse::subscribe_hl_blocks(path, tx));
    while let Ok(i) = rx.recv() {
        log::info!("received: {:?}", i);
    }
    Ok(())
}
