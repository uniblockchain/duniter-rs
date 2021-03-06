//  Copyright (C) 2018  The Duniter Project Developers.
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

//! Sub-module checking if a block complies with all the rules of the (DUBP DUniter Blockchain Protocol).

pub mod hashs;

use crate::dubp::BlockError;
use dubp_documents::documents::block::BlockDocument;
use dubp_documents::*;
use dup_crypto::keys::PubKey;
use durs_blockchain_dal::*;
use durs_wot::*;
use std::collections::HashMap;

#[derive(Debug, Copy, Clone)]
pub enum InvalidBlockError {
    NoPreviousBlock,
    VersionDecrease,
}

pub fn verify_block_validity<W: WebOfTrust>(
    block: &BlockDocument,
    blockchain_db: &BinDB<LocalBlockchainV10Datas>,
    _certs_db: &BinDB<CertsExpirV10Datas>,
    _wot_index: &HashMap<PubKey, NodeId>,
    _wot_db: &BinDB<W>,
) -> Result<(), BlockError> {
    // Rules that do not concern genesis block
    if block.number.0 > 0 {
        // Get previous block
        let previous_block_opt = readers::block::get_block_in_local_blockchain(
            blockchain_db,
            BlockNumber(block.number.0 - 1),
        )?;

        // Previous block must exist
        if previous_block_opt.is_none() {
            return Err(BlockError::InvalidBlock(InvalidBlockError::NoPreviousBlock));
        }
        let previous_block = previous_block_opt.expect("safe unwrap");

        // Block version must not decrease
        if previous_block.version > block.version {
            return Err(BlockError::InvalidBlock(InvalidBlockError::VersionDecrease));
        }
    }

    Ok(())
}
