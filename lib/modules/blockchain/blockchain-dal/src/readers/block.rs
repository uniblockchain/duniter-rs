//  Copyright (C) 2018  The Durs Project Developers.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::constants::MAX_FORKS;
use crate::*;
use dubp_documents::documents::block::BlockDocument;
use dubp_documents::Document;
use dubp_documents::{BlockHash, BlockId, Blockstamp};
use dup_crypto::keys::*;
use std::collections::HashMap;

///Get forks status
pub fn get_forks(
    forks_db: &BinDB<ForksV10Datas>,
    current_blockstamp: Blockstamp,
) -> Result<Vec<ForkStatus>, DALError> {
    Ok(forks_db.read(|forks_db| {
        let blockchain_meta_datas = forks_db
            .get(&ForkId(0))
            .expect("Fatal error : ForksV10DB not contain local chain !");
        let mut forks = Vec::new();
        for fork_id in 1..*MAX_FORKS {
            if let Some(fork_meta_datas) = forks_db.get(&ForkId(fork_id)) {
                if fork_meta_datas.is_empty() {
                    forks.push(ForkStatus::Free());
                } else if fork_meta_datas.contains_key(&current_blockstamp) {
                    forks.push(ForkStatus::Stackable(ForkAlreadyCheck(false)));
                } else {
                    let roll_back_max = if current_blockstamp.id.0 > 101 {
                        current_blockstamp.id.0 - 101
                    } else {
                        0
                    };
                    let mut max_common_block_id = None;
                    let mut too_old = false;
                    for previous_blockstamp in fork_meta_datas.keys() {
                        if blockchain_meta_datas.contains_key(&previous_blockstamp) {
                            if previous_blockstamp.id.0 >= roll_back_max {
                                if previous_blockstamp.id.0
                                    >= max_common_block_id.unwrap_or(BlockId(0)).0
                                {
                                    max_common_block_id = Some(previous_blockstamp.id);
                                    too_old = false;
                                }
                            } else {
                                too_old = true;
                            }
                        }
                    }
                    if too_old {
                        forks.push(ForkStatus::TooOld(ForkAlreadyCheck(false)));
                    } else if let Some(max_common_block_id) = max_common_block_id {
                        forks.push(ForkStatus::RollBack(
                            ForkAlreadyCheck(false),
                            max_common_block_id,
                        ));
                    } else {
                        forks.push(ForkStatus::Isolate());
                    }
                }
            } else {
                forks.push(ForkStatus::Free());
            }
        }
        forks
    })?)
}
/// get current blockstamp
pub fn get_current_blockstamp(blocks_db: &BlocksV10DBs) -> Result<Option<Blockstamp>, DALError> {
    let current_previous_blockstamp = blocks_db.blockchain_db.read(|db| {
        let blockchain_len = db.len() as u32;
        if blockchain_len == 0 {
            None
        } else if let Some(dal_block) = db.get(&BlockId(blockchain_len - 1)) {
            if blockchain_len > 1 {
                Some(Blockstamp {
                    id: BlockId(blockchain_len - 2),
                    hash: BlockHash(dal_block.block.previous_hash),
                })
            } else {
                Some(Blockstamp::default())
            }
        } else {
            None
        }
    })?;
    if current_previous_blockstamp.is_none() {
        return Ok(None);
    }
    let current_previous_blockstamp = current_previous_blockstamp.expect("safe unwrap");
    if let Some(current_block_hash) = blocks_db.forks_db.read(|db| {
        let blockchain_meta_datas = db
            .get(&ForkId(0))
            .expect("Fatal error : ForksDB is incoherent, please reset data and resync !");
        blockchain_meta_datas
            .get(&current_previous_blockstamp)
            .cloned()
    })? {
        Ok(Some(Blockstamp {
            id: BlockId(current_previous_blockstamp.id.0 + 1),
            hash: current_block_hash,
        }))
    } else {
        Ok(None)
    }
}

/// Get block fork id
pub fn get_fork_id_of_blockstamp(
    forks_blocks_db: &BinDB<ForksBlocksV10Datas>,
    blockstamp: &Blockstamp,
) -> Result<Option<ForkId>, DALError> {
    Ok(forks_blocks_db.read(|db| {
        if let Some(dal_block) = db.get(blockstamp) {
            Some(dal_block.fork_id)
        } else {
            None
        }
    })?)
}

/// Get block hash
pub fn get_block_hash(
    db: &BinDB<LocalBlockchainV10Datas>,
    block_number: BlockId,
) -> Result<Option<BlockHash>, DALError> {
    Ok(db.read(|db| {
        if let Some(dal_block) = db.get(&block_number) {
            dal_block.block.hash
        } else {
            None
        }
    })?)
}
/// Return true if the node already knows this block
pub fn already_have_block(
    blockchain_db: &BinDB<LocalBlockchainV10Datas>,
    forks_blocks_db: &BinDB<ForksBlocksV10Datas>,
    blockstamp: Blockstamp,
) -> Result<bool, DALError> {
    let already_have_block = forks_blocks_db.read(|db| db.contains_key(&blockstamp))?;
    if !already_have_block {
        Ok(blockchain_db.read(|db| {
            if let Some(dal_block) = db.get(&blockstamp.id) {
                if dal_block.block.hash.unwrap_or_default() == blockstamp.hash {
                    return true;
                }
            }
            false
        })?)
    } else {
        Ok(true)
    }
}

/// Get block
pub fn get_block(
    blockchain_db: &BinDB<LocalBlockchainV10Datas>,
    forks_blocks_db: Option<&BinDB<ForksBlocksV10Datas>>,
    blockstamp: &Blockstamp,
) -> Result<Option<DALBlock>, DALError> {
    let dal_block = blockchain_db.read(|db| db.get(&blockstamp.id).cloned())?;
    if dal_block.is_none() && forks_blocks_db.is_some() {
        Ok(forks_blocks_db
            .expect("safe unwrap")
            .read(|db| db.get(&blockstamp).cloned())?)
    } else {
        Ok(dal_block)
    }
}
/// Get block in local blockchain
pub fn get_block_in_local_blockchain(
    db: &BinDB<LocalBlockchainV10Datas>,
    block_id: BlockId,
) -> Result<Option<BlockDocument>, DALError> {
    Ok(db.read(|db| {
        if let Some(dal_block) = db.get(&block_id) {
            Some(dal_block.block.clone())
        } else {
            None
        }
    })?)
}

/// Get current frame of calculating members
pub fn get_current_frame(
    current_block: &DALBlock,
    db: &BinDB<LocalBlockchainV10Datas>,
) -> Result<HashMap<PubKey, usize>, DALError> {
    let frame_begin = current_block.block.number.0 - current_block.block.issuers_frame as u32;
    Ok(db.read(|db| {
        let mut current_frame: HashMap<PubKey, usize> = HashMap::new();
        for block_number in frame_begin..current_block.block.number.0 {
            let issuer = db
                .get(&BlockId(block_number))
                .unwrap_or_else(|| panic!("Fail to get block #{} !", block_number))
                .block
                .issuers()[0];
            let issuer_count_blocks = if let Some(issuer_count_blocks) = current_frame.get(&issuer)
            {
                issuer_count_blocks + 1
            } else {
                1
            };
            current_frame.insert(issuer, issuer_count_blocks);
        }
        current_frame
    })?)
}
