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

//! Sub-module managing the inter-modules requests received.

use crate::*;
use dubp_documents::documents::identity::IdentityDocument;
use durs_message::requests::*;
use durs_module::*;

pub fn receive_req(
    bc: &BlockchainModule,
    req_from: ModuleStaticName,
    req_id: ModuleReqId,
    req_content: DursReqContent,
) {
    if let DursReqContent::BlockchainRequest(blockchain_req) = req_content {
        match blockchain_req {
            BlockchainRequest::CurrentBlockstamp() => responses::sent::send_req_response(
                bc,
                req_from,
                req_id,
                &BlockchainResponse::CurrentBlockstamp(bc.current_blockstamp),
            ),
            BlockchainRequest::CurrentBlock => {
                debug!("BlockchainModule : receive BlockchainRequest::CurrentBlock()");

                if let Ok(block_opt) = readers::block::get_block(
                    &bc.blocks_databases.blockchain_db,
                    None,
                    &bc.current_blockstamp,
                ) {
                    if let Some(dal_block) = block_opt {
                        debug!(
                            "BlockchainModule : send_req_response(CurrentBlock({}))",
                            bc.current_blockstamp
                        );
                        responses::sent::send_req_response(
                            bc,
                            req_from,
                            req_id,
                            &BlockchainResponse::CurrentBlock(
                                Box::new(dal_block.block),
                                bc.current_blockstamp,
                            ),
                        );
                    } else {
                        warn!("BlockchainModule : Req : fail to get current_block in bdd !");
                    }
                } else {
                    fatal_error!(
                        "BlockchainModule: get_block(): fail to read LocalBlockchainV10DB !"
                    )
                }
            }
            BlockchainRequest::BlockByNumber { block_number } => {
                debug!(
                    "BlockchainModule : receive BlockchainRequest::BlockByNumber(#{})",
                    block_number
                );

                if let Ok(block_opt) = readers::block::get_block_in_local_blockchain(
                    &bc.blocks_databases.blockchain_db,
                    block_number,
                ) {
                    if let Some(block) = block_opt {
                        debug!(
                            "BlockchainModule : send_req_response(BlockByNumber(#{}))",
                            block_number
                        );
                        responses::sent::send_req_response(
                            bc,
                            req_from,
                            req_id,
                            &BlockchainResponse::BlockByNumber(Box::new(block)),
                        );
                    } else {
                        debug!(
                            "BlockchainModule : Req : not found block #{} in bdd !",
                            block_number
                        );
                    }
                } else {
                    fatal_error!(
                        "BlockchainModule: get_block(): fail to read LocalBlockchainV10DB !"
                    )
                }
            }
            BlockchainRequest::Chunk {
                first_block_number,
                count,
            } => {
                debug!(
                    "BlockchainModule : receive BlockchainRequest::Chunk(#{}, {})",
                    first_block_number, count
                );

                if let Ok(blocks) = readers::block::get_blocks_in_local_blockchain(
                    &bc.blocks_databases.blockchain_db,
                    first_block_number,
                    count,
                ) {
                    if blocks.is_empty() {
                        debug!(
                            "BlockchainModule : Req : not found chunk (#{}, {}) in bdd !",
                            first_block_number, count,
                        );
                    } else {
                        debug!(
                            "BlockchainModule : send_req_response(Chunk(#{}, {}))",
                            first_block_number,
                            blocks.len(),
                        );
                        responses::sent::send_req_response(
                            bc,
                            req_from,
                            req_id,
                            &BlockchainResponse::Chunk(blocks),
                        );
                    }
                } else {
                    fatal_error!(
                        "BlockchainModule: get_block(): fail to read LocalBlockchainV10DB !"
                    )
                }
            }
            BlockchainRequest::UIDs(pubkeys) => {
                responses::sent::send_req_response(
                    bc,
                    req_from,
                    req_id,
                    &BlockchainResponse::UIDs(
                        pubkeys
                            .into_iter()
                            .map(|p| {
                                (
                                    p,
                                    durs_blockchain_dal::readers::identity::get_uid(
                                        &bc.wot_databases.identities_db,
                                        p,
                                    )
                                    .expect("Fatal error : get_uid : Fail to read WotV10DB !"),
                                )
                            })
                            .collect(),
                    ),
                );
            }
            BlockchainRequest::GetIdentities(filters) => {
                let identities = durs_blockchain_dal::readers::identity::get_identities(
                    &bc.wot_databases.identities_db,
                    filters,
                    bc.current_blockstamp.id,
                )
                .expect("Fatal error : get_identities: Fail to read IdentitiesDB !")
                .into_iter()
                .map(|dal_idty| dal_idty.idty_doc)
                .collect::<Vec<IdentityDocument>>();
                responses::sent::send_req_response(
                    bc,
                    req_from,
                    req_id,
                    &BlockchainResponse::Identities(identities),
                );
            }
        }
    }
}
