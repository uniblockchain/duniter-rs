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

//! Sub-module managing the inter-modules responses received.

use crate::*;
use std::ops::Deref;

pub fn receive_response(
    bc: &mut BlockchainModule,
    req_id: ModuleReqId,
    res_content: &DursResContent,
) {
    if let DursResContent::NetworkResponse(ref network_response) = *res_content {
        debug!("BlockchainModule : receive NetworkResponse() !");
        if let Some(request) = bc.pending_network_requests.remove(&req_id) {
            match request {
                OldNetworkRequest::GetConsensus(_) => {
                    if let NetworkResponse::Consensus(_, response) = *network_response.deref() {
                        if let Ok(blockstamp) = response {
                            bc.consensus = blockstamp;
                        }
                    }
                }
                OldNetworkRequest::GetBlocks(_, _, _, _) => {
                    if let NetworkResponse::Chunk(_, _, ref blocks) = *network_response.deref() {
                        let blocks: Vec<Block> = blocks
                            .iter()
                            .map(|b| Block::NetworkBlock(b.deref().clone()))
                            .collect();
                        dunp::receiver::receive_blocks(bc, blocks);
                    }
                }
                _ => {}
            }
        }
    }
}
