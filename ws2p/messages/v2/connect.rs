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

//use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use super::api_features::WS2PFeatures;
use duniter_crypto::hashs::Hash;
use duniter_documents::{Blockstamp, ReadBytesBlockstampError};
use duniter_network::network_peer::{PeerCardReadBytesError, PeerCardV11};
use dup_binarizer::u16;
use dup_binarizer::*;
//use std::io::Cursor;
//use std::mem;

/// WS2P v2 connect message min size
pub static CONNECT_MSG_MIN_SIZE: &'static usize = &36;

#[derive(Clone, Debug, Eq, PartialEq)]
/// WS2PConnectFlags
pub struct WS2PConnectFlags(Vec<u8>);

impl WS2PConnectFlags {
    pub fn is_empty(&self) -> bool {
        for byte in &self.0 {
            if *byte > 0u8 {
                return false;
            }
        }
        true
    }
    pub fn _sync(&self) -> bool {
        self.0[0] & 0b0000_0001 == 1u8
    }
    pub fn _ask_sync_chunk(&self) -> bool {
        self.0[0] & 0b0000_0010 == 2u8
    }
    pub fn _res_sync_chunk(&self) -> bool {
        self.0[0] & 0b0000_0100 == 4u8
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// WS2Pv2OkWS2Pv2ConnectMsgMsg
pub struct WS2Pv2ConnectMsg {
    /// random hash generated by the sending node of the CONNECT message,
    /// the receiving node will then have to sign this challenge and then
    /// send this signature in its ACK message to prove that it has the corresponding private key to the public key it indicates.
    pub challenge: Hash,
    /// This is exactly the same type as the field of the same name in the endpoints. But private WS2P nodes do not declare endpoints,
    /// so they must be able to indicate in the CONNECT message which features they support. Public WS2P nodes also fill this field,
    /// so any changes in the configuration of a public node will be applied on the 1st new connection. (If this was not the case,
    /// we would have to wait for the update of the peer record).
    pub api_features: WS2PFeatures,
    /// WS2PConnectFlags
    pub flags_queries: WS2PConnectFlags,
    /// Issuer PeerCard
    pub peer_card: Option<PeerCardV11>,
    /// Blockstamp of the last block of the chunk
    pub chunkstamp: Option<Blockstamp>,
}

impl Default for WS2Pv2ConnectMsg {
    fn default() -> Self {
        WS2Pv2ConnectMsg {
            challenge: Hash::random(),
            api_features: WS2PFeatures(vec![]),
            flags_queries: WS2PConnectFlags(vec![]),
            peer_card: None,
            chunkstamp: None,
        }
    }
}

/// ReadWS2Pv2ConnectMsgError
#[derive(Debug)]
pub enum ReadWS2Pv2ConnectMsgError {
    /// TooShort
    TooShort(&'static str),
    /// WrongSize
    WrongSize(&'static str),
    /// IoError
    IoError(::std::io::Error),
    /// PeerCardReadBytesError
    PeerCardReadBytesError(PeerCardReadBytesError),
    /// ReadBytesBlockstampError
    ReadBytesBlockstampError(ReadBytesBlockstampError),
}

impl From<::std::io::Error> for ReadWS2Pv2ConnectMsgError {
    fn from(e: ::std::io::Error) -> Self {
        ReadWS2Pv2ConnectMsgError::IoError(e)
    }
}

impl From<ReadBytesBlockstampError> for ReadWS2Pv2ConnectMsgError {
    fn from(e: ReadBytesBlockstampError) -> Self {
        ReadWS2Pv2ConnectMsgError::ReadBytesBlockstampError(e)
    }
}

impl From<PeerCardReadBytesError> for ReadWS2Pv2ConnectMsgError {
    fn from(e: PeerCardReadBytesError) -> Self {
        ReadWS2Pv2ConnectMsgError::PeerCardReadBytesError(e)
    }
}

impl BinMessage for WS2Pv2ConnectMsg {
    type ReadBytesError = ReadWS2Pv2ConnectMsgError;
    fn from_bytes(datas: &[u8]) -> Result<Self, Self::ReadBytesError> {
        if datas.len() < *CONNECT_MSG_MIN_SIZE {
            return Err(ReadWS2Pv2ConnectMsgError::TooShort(
                "Insufficient size (<CONNECT_MSG_MIN_SIZE).",
            ));
        }
        let mut index = 0;
        // read challenge
        let mut challenge_datas: [u8; 32] = [0u8; 32];
        challenge_datas.copy_from_slice(&datas[0..32]);
        let challenge = Hash(challenge_datas);
        index += 32;
        // read af_size
        let af_size = datas[index] as usize;
        index += 1;
        // read api_features
        let api_features = WS2PFeatures(datas[index..index + af_size].to_vec());
        index += af_size;
        // read flags_size
        let flags_size = datas[index] as usize;
        index += 1;
        // read flags_queries
        let flags_queries = WS2PConnectFlags(datas[index..index + flags_size].to_vec());
        index += flags_size;
        // read peer_card
        let peer_card = if datas.len() > index + 2 {
            let peer_card_size = u16::read_u16_be(&datas[index..index + 2])? as usize;
            index += 2 + peer_card_size;
            if peer_card_size > 0 {
                if datas.len() < index {
                    return Err(ReadWS2Pv2ConnectMsgError::TooShort("peer_card"));
                }
                Some(PeerCardV11::from_bytes(
                    &datas[index - peer_card_size..index],
                )?)
            } else {
                None
            }
        } else if datas.len() == index {
            None
        } else {
            return Err(ReadWS2Pv2ConnectMsgError::TooShort("peer_card_size"));
        };
        // read chunkstamp
        let chunkstamp = if datas.len() == index + Blockstamp::SIZE_IN_BYTES {
            Some(Blockstamp::from_bytes(&datas[index..])?)
        } else if datas.len() == index {
            None
        } else {
            return Err(ReadWS2Pv2ConnectMsgError::WrongSize("chunkstamp"));
        };
        Ok(WS2Pv2ConnectMsg {
            challenge,
            api_features,
            flags_queries,
            peer_card,
            chunkstamp,
        })
    }
    fn to_bytes_vector(&self) -> Vec<u8> {
        // Binarize peer_card
        let bin_peer_card = if let Some(ref peer_card) = self.peer_card {
            peer_card.to_bytes_vector()
        } else {
            vec![0u8, 0u8] // peer_card_size
        };
        // Binarize bin_chunkstamp
        let bin_chunkstamp = if let Some(ref chunkstamp) = self.chunkstamp {
            chunkstamp.to_bytes_vector()
        } else {
            vec![]
        };
        // Compute msg_bin_size and allocate buffer
        let af_size = if self.api_features.is_empty() {
            0
        } else {
            self.api_features.0.len()
        };
        let flags_size = if self.flags_queries.is_empty() {
            0
        } else {
            self.flags_queries.0.len()
        };
        let msg_bin_size = CONNECT_MSG_MIN_SIZE
            + af_size
            + flags_size
            + bin_peer_card.len()
            + bin_chunkstamp.len();
        let mut buffer = Vec::with_capacity(msg_bin_size);
        // Write challenge
        buffer.extend_from_slice(&self.challenge.0);
        // Write af_size
        buffer.push(af_size as u8);
        // Write api_features
        if !self.api_features.is_empty() {
            buffer.extend(&self.api_features.0);
        }
        // Write flags_size
        buffer.push(flags_size as u8);
        // Write flags_queries
        if !self.flags_queries.is_empty() {
            buffer.extend(&self.flags_queries.0);
        }
        // Write peer_card
        if !bin_peer_card.is_empty() {
            buffer.extend(&bin_peer_card);
        }
        // Write chunkstamp
        if !bin_chunkstamp.is_empty() {
            buffer.extend(&bin_chunkstamp);
        }
        buffer
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;
    use duniter_documents::Blockstamp;

    fn keypair1() -> ed25519::KeyPair {
        ed25519::KeyPairFromSaltedPasswordGenerator::with_default_parameters().generate(
            "JhxtHB7UcsDbA9wMSyMKXUzBZUQvqVyB32KwzS9SWoLkjrUhHV".as_bytes(),
            "JhxtHB7UcsDbA9wMSyMKXUzBZUQvqVyB32KwzS9SWoLkjrUhHV_".as_bytes(),
        )
    }

    #[test]
    fn test_ws2p_message_connect() {
        let keypair1 = keypair1();

        let connect_msg = WS2Pv2ConnectMsg {
            challenge: Hash::from_hex(
                "000007722B243094269E548F600BD34D73449F7578C05BD370A6D301D20B5F10",
            ).unwrap(),
            api_features: WS2PFeatures(vec![]),
            flags_queries: WS2PConnectFlags(vec![]),
            peer_card: None,
            chunkstamp: Some(
                Blockstamp::from_string(
                    "499-000011BABEEE1020B1F6B2627E2BC1C35BCD24375E114349634404D2C266D84F",
                ).unwrap(),
            ),
        };
        let mut ws2p_message = WS2Pv2Message {
            currency_code: CurrencyName(String::from("g1")),
            ws2p_version: 2u16,
            issuer_node_id: NodeId(0),
            issuer_pubkey: PubKey::Ed25519(keypair1.public_key()),
            payload: WS2Pv2MessagePayload::Connect(Box::new(connect_msg)),
            message_hash: None,
            signature: None,
        };

        let sign_result = ws2p_message.sign(PrivKey::Ed25519(keypair1.private_key()));
        if let Ok(bin_msg) = sign_result {
            // Test binarization
            assert_eq!(ws2p_message.to_bytes_vector(), bin_msg);
            // Test sign
            assert_eq!(ws2p_message.verify(), Ok(()));
            // Test debinarization
            let debinarization_result = WS2Pv2Message::from_bytes(&bin_msg);
            if let Ok(ws2p_message2) = debinarization_result {
                assert_eq!(ws2p_message, ws2p_message2);
            } else {
                panic!(
                    "Fail to debinarize ws2p_message : {:?}",
                    debinarization_result.err().unwrap()
                );
            }
        } else {
            panic!(
                "Fail to sign ws2p_message : {:?}",
                sign_result.err().unwrap()
            );
        }
    }
}
