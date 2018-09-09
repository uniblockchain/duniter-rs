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

/// WS2P Features
pub mod api_features;
/// WS2P v2 CONNECT Message
pub mod connect;
/// WS2P v2 OK Message
pub mod ok;
/// Message Payload container
pub mod payload_container;
/// WS2Pv2 requests responses messages
pub mod req_responses;
/// WS2Pv2 requests messages
pub mod requests;
/// WS2P v2 SECRET_FLAGS Message
pub mod secret_flags;

use duniter_crypto::hashs::Hash;
use duniter_crypto::keys::*;
use duniter_documents::CurrencyName;
use duniter_network::NodeId;
use dup_binarizer::*;
use v2::payload_container::*;

/// WS2P v2 message metadata size
pub static WS2P_V2_MESSAGE_METADATA_SIZE: &'static usize = &144;

/// WS2Pv0Message
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct WS2Pv0Message {
    /// Currency name
    pub currency_code: CurrencyName,
    /// Issuer NodeId
    pub issuer_node_id: NodeId,
    /// Issuer plublic key
    pub issuer_pubkey: PubKey,
    /// Message payload
    pub payload: WS2Pv0MessagePayload,
    /// Message hash
    pub message_hash: Option<Hash>,
    /// Signature
    pub signature: Option<Sig>,
}

impl WS2Pv0Message {
    /// WS2P Version number
    pub const WS2P_VERSION: u16 = 0;
}

impl<'de> BinMessageSignable<'de> for WS2Pv0Message {
    fn issuer_pubkey(&self) -> PubKey {
        self.issuer_pubkey
    }
    fn store_hash(&self) -> bool {
        true
    }
    fn hash(&self) -> Option<Hash> {
        self.message_hash
    }
    fn set_hash(&mut self, hash: Hash) {
        self.message_hash = Some(hash)
    }
    fn signature(&self) -> Option<Sig> {
        self.signature
    }
    fn set_signature(&mut self, signature: Sig) {
        self.signature = Some(signature)
    }
}

/*impl BinMessage for WS2Pv0Message {
    type ReadBytesError = WS2Pv0MessageReadBytesError;
    fn from_bytes(binary_msg: &[u8]) -> Result<WS2Pv0Message, WS2Pv0MessageReadBytesError> {
        let mut index = 0;
        // read currency_code
        let mut currency_code_bytes = Cursor::new(binary_msg[index..index + 2].to_vec());
        index += 2;
        let currency_code = CurrencyName::from_u16(currency_code_bytes.read_u16::<BigEndian>()?)?;
        // read ws2p_version
        let mut ws2p_version_bytes = Cursor::new(binary_msg[index..index + 2].to_vec());
        index += 2;
        let ws2p_version = ws2p_version_bytes.read_u16::<BigEndian>()?;
        match ws2p_version {
            2u16 => {
                // read issuer_node_id
                let mut node_id_bytes = Cursor::new(binary_msg[index..index + 4].to_vec());
                index += 4;
                let issuer_node_id = NodeId(node_id_bytes.read_u32::<BigEndian>()?);
                // read issuer_size
                let issuer_size = u16::read_u16_be(&binary_msg[index..index + 2])? as usize;
                index += 2;
                // read issuer_pubkey
                let (issuer_pubkey, key_algo) = if binary_msg.len() > index + issuer_size {
                    index += issuer_size;
                    pubkey_box::read_pubkey_box(&binary_msg[index - issuer_size..index])?
                } else {
                    return Err(WS2Pv0MessageReadBytesError::TooShort(String::from(
                        "issuer",
                    )));
                };
                // read payload_size
                let payload_size = if binary_msg.len() > index + 8 {
                    let mut payload_size_bytes =
                        Cursor::new(binary_msg[index + 4..index + 8].to_vec());
                    payload_size_bytes.read_u32::<BigEndian>()? as usize
                } else {
                    return Err(WS2Pv0MessageReadBytesError::TooShort(String::from(
                        "payload_size",
                    )));
                };
                // read payload
                let payload = if binary_msg.len() > index + payload_size + 8 {
                    index += payload_size + 8;
                    WS2Pv0MessagePayload::from_bytes(
                        &binary_msg[index - (payload_size + 8)..index],
                    )?
                } else {
                    return Err(WS2Pv0MessageReadBytesError::TooShort(String::from(
                        "payload",
                    )));
                };
                // read message_hash
                let message_hash = if binary_msg.len() >= index + 32 {
                    let mut hash_datas: [u8; 32] = [0u8; 32];
                    index += 32;
                    hash_datas.copy_from_slice(&binary_msg[index - 32..index]);
                    Some(Hash(hash_datas))
                } else if binary_msg.len() == index {
                    None
                } else {
                    return Err(WS2Pv0MessageReadBytesError::TooShort(String::from(
                        "message_hash",
                    )));
                };
                // read signature_size
                let signature_size = if binary_msg.len() > index + 2 {
                    index += 2;
                    u16::read_u16_be(&binary_msg[index - 2..index])? as usize
                } else {
                    return Err(WS2Pv0MessageReadBytesError::TooShort(String::from(
                        "signature_size",
                    )));
                };
                // read signature
                let signature = if binary_msg.len() > index + signature_size {
                    return Err(WS2Pv0MessageReadBytesError::TooLong());
                } else if binary_msg.len() == index + signature_size {
                    index += signature_size;
                    Some(sig_box::read_sig_box(
                        &binary_msg[index - signature_size..index],
                        key_algo,
                    )?)
                } else if binary_msg.len() > index {
                    return Err(WS2Pv0MessageReadBytesError::TooLong());
                } else if binary_msg.len() == index {
                    None
                } else {
                    return Err(WS2Pv0MessageReadBytesError::TooShort(String::from("end")));
                };
                Ok(WS2Pv0Message {
                    currency_code,
                    issuer_node_id,
                    issuer_pubkey,
                    payload,
                    message_hash,
                    signature,
                })
            }
            0u16 | 1u16 => Err(WS2Pv0MessageReadBytesError::TooEarlyVersion()),
            _ => Err(WS2Pv0MessageReadBytesError::VersionNotYetSupported()),
        }
    }
    fn to_bytes_vector(&self) -> Vec<u8> {
        // Binarize payload (message_type + elements_count + payload_size + payload)
        let bin_payload = self.payload.to_bytes_vector();
        let payload_size = bin_payload.len() - *WS2P_V2_MESSAGE_PAYLOAD_METADATA_SIZE;
        let msg_size = *WS2P_V2_MESSAGE_METADATA_SIZE + payload_size as usize;
        let mut bytes_vector = Vec::with_capacity(msg_size);
        // currency_code
        bytes_vector.extend_from_slice(
            &self
                .currency_code
                .to_bytes()
                .expect("Fatal Error : Try to binarize WS2Pv2 message with UnknowCurrencyName !"),
        );
        // ws2p_version
        let mut buffer = [0u8; mem::size_of::<u16>()];
        buffer
            .as_mut()
            .write_u16::<BigEndian>(WS2Pv0Message::WS2P_VERSION)
            .expect("Unable to write");
        bytes_vector.extend_from_slice(&buffer);
        // Write issuer_node_id
        let mut buffer = [0u8; mem::size_of::<u32>()];
        buffer
            .as_mut()
            .write_u32::<BigEndian>(self.issuer_node_id.0)
            .expect("Unable to write");
        bytes_vector.extend_from_slice(&buffer);
        // Write issuer_pubey
        pubkey_box::write_pubkey_box(&mut bytes_vector, self.issuer_pubkey)
            .expect("Fail to binarize peer.issuer !");
        // Write payload : message_type + elements_count + payload_size + payload_content
        bytes_vector.extend(bin_payload);
        // Write message_hash
        if let Some(message_hash) = self.message_hash {
            bytes_vector.extend(message_hash.to_bytes_vector());
        }
        // Write signature
        if let Some(signature) = self.signature {
            sig_box::write_sig_box(&mut bytes_vector, signature)
                .expect("Fail to binarize msg.sig !");
        }

        bytes_vector
    }
}*/

#[cfg(test)]
mod tests {
    use super::*;
    use tests::*;

    #[test]
    fn test_ws2p_message_ack() {
        test_ws2p_message(WS2Pv0MessagePayload::Ack(Hash::random()));
    }

    #[test]
    fn test_ws2p_message_peers() {
        let keypair1 = keypair1();
        let mut peer = create_peer_card_v11();
        peer.sign(PrivKey::Ed25519(keypair1.private_key()))
            .expect("Fail to sign peer card !");
        test_ws2p_message(WS2Pv0MessagePayload::Peers(vec![peer]));
    }
}
