//  Copyright (C) 2019  Éloïs SANCHEZ
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

//! Mocks for projects use dubp-documents

pub mod identity;

use dubp_documents::documents::block::BlockDocument;
use dubp_documents::*;
use dup_crypto::hashs::Hash;

/// Generate n mock blockstamps
pub fn generate_blockstamps(n: usize) -> Vec<Blockstamp> {
    (0..n)
        .map(|i| Blockstamp {
            id: BlockNumber(i as u32),
            hash: BlockHash(dup_crypto_tests_tools::mocks::hash_from_byte(
                (i % 255) as u8,
            )),
        })
        .collect()
}

/// Generate n empty timed block document
pub fn gen_empty_timed_blocks(n: usize, time_step: u64) -> Vec<BlockDocument> {
    (0..n)
        .map(|i| {
            gen_empty_timed_block(
                Blockstamp {
                    id: BlockNumber(i as u32),
                    hash: BlockHash(dup_crypto_tests_tools::mocks::hash_from_byte(
                        (i % 255) as u8,
                    )),
                },
                time_step * n as u64,
                if i == 0 {
                    Hash::default()
                } else {
                    dup_crypto_tests_tools::mocks::hash_from_byte(((i - 1) % 255) as u8)
                },
            )
        })
        .collect()
}

/// Generate empty timed block document
/// (usefull for tests that only need blockstamp and median_time fields)
pub fn gen_empty_timed_block(
    blockstamp: Blockstamp,
    time: u64,
    previous_hash: Hash,
) -> BlockDocument {
    BlockDocument {
        version: 10,
        nonce: 0,
        number: blockstamp.id,
        pow_min: 0,
        time: 0,
        median_time: time,
        members_count: 0,
        monetary_mass: 0,
        unit_base: 0,
        issuers_count: 0,
        issuers_frame: 0,
        issuers_frame_var: 0,
        currency: CurrencyName::default(),
        issuers: vec![],
        signatures: vec![],
        hash: Some(blockstamp.hash),
        parameters: None,
        previous_hash,
        previous_issuer: None,
        dividend: None,
        identities: vec![],
        joiners: vec![],
        actives: vec![],
        leavers: vec![],
        revoked: vec![],
        excluded: vec![],
        certifications: vec![],
        transactions: vec![],
        inner_hash: None,
        inner_hash_and_nonce_str: "".to_owned(),
    }
}
