use std::{
    fs::File,
    io::{BufRead, BufReader, Seek, SeekFrom},
    path::PathBuf,
};

use ethers::{
    contract::{Eip712, EthAbiType},
    types::{transaction::eip712::Eip712, Address, Signature, H256},
};
use hyperliquid_rust_sdk::Actions;
use notify::{event::ModifyKind, Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Deserialize;

#[derive(Debug, Eip712, Clone, EthAbiType)]
#[eip712(
    name = "Exchange",
    version = "1",
    chain_id = 1337,
    verifying_contract = "0x0000000000000000000000000000000000000000"
)]
pub struct Agent {
    pub source: String,
    pub connection_id: H256,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SignedAction {
    pub signature: Signature,
    pub vault_address: Option<Address>,
    pub action: Actions,
    pub nonce: u64,
}

#[derive(Debug, Deserialize)]
pub struct SignedActions(pub H256, pub Vec<SignedAction>);

#[derive(Debug, Deserialize)]
pub struct InnerBlock {
    pub time: String,
    pub raw_height: u64,
    pub signed_actions: Vec<SignedActions>,
}

#[derive(Debug, Deserialize)]
pub struct Block {
    pub block: InnerBlock,
    pub app_hash: Vec<u8>,
    pub height: u64,
}

impl Block {
    pub fn txs(&self) -> Vec<SignedAction> {
        self.block
            .signed_actions
            .iter()
            .flat_map(|x| x.1.clone())
            .collect()
    }
}

impl SignedAction {
    pub fn action_hash(&self) -> eyre::Result<H256> {
        let mut bytes = rmp_serde::to_vec_named(&self.action)?;
        bytes.extend(self.nonce.to_be_bytes());
        if let Some(vault_address) = self.vault_address {
            bytes.push(1);
            bytes.extend(vault_address.to_fixed_bytes());
        } else {
            bytes.push(0);
        }
        Ok(H256(ethers::utils::keccak256(bytes)))
    }

    pub fn hash(&self) -> eyre::Result<H256> {
        let connection_id = self.action_hash()?;
        let agent = Agent {
            source: "b".to_string(),
            connection_id,
        };
        let hash = agent.encode_eip712()?;
        Ok(H256::from(hash))
    }

    pub fn sender(&self) -> Address {
        let from = if let Some(vault_address) = self.vault_address {
            vault_address.into()
        } else {
            let hash = self.hash().unwrap();
            self.signature.recover(hash).unwrap()
        };
        from
    }
}

pub async fn subscribe_hl_blocks(
    path: PathBuf,
    block_tx: tokio::sync::mpsc::Sender<eyre::Result<Block>>,
) -> eyre::Result<()> {
    let mut path = PathBuf::from(path);
    let mut pos = std::fs::metadata(&path)?.len();

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
    watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

    for res in rx {
        match res {
            Ok(event) => {
                if let EventKind::Modify(ModifyKind::Data(_)) = event.kind {
                    let paths = event.paths;
                    let modified_path = paths.first().unwrap().clone();
                    if path != modified_path {
                        log::info!("Current file modified to: {modified_path:?}");
                        pos = 0;
                        path = modified_path.clone();
                    }
                    let mut f = File::open(&path).unwrap();
                    f.seek(SeekFrom::Start(pos)).unwrap();

                    pos = f.metadata().unwrap().len();

                    let reader = BufReader::new(&f);
                    for line in reader.lines() {
                        match line {
                            Ok(line) => {
                                if line.starts_with('{') {
                                    let block: eyre::Result<Block> = serde_json::from_str(&line)
                                        .map_err(|err| {
                                            eyre::eyre!(
                                                "Failed to parse block: {line:?}.\nError: {err:?}",
                                            )
                                        });
                                    if let Err(err) = block_tx.send(block).await {
                                        log::error!("failed to send block: {err:?}");
                                    }
                                }
                            }
                            Err(error) => {
                                log::error!("{error:?}");
                            }
                        }
                    }
                }
            }
            Err(error) => log::error!("{error:?}"),
        }
    }

    Ok(())
}
