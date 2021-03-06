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

//use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use super::api_features::WS2PFeatures;
use dubp_documents::Blockstamp;
use dup_crypto::hashs::Hash;
use durs_network_documents::network_peer::PeerCardV11;

/// WS2P v2 connect message min size
pub static CONNECT_MSG_MIN_SIZE: &'static usize = &36;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
/// WS2PConnectFlags
pub struct WS2PConnectFlags(Vec<u8>);

impl WS2PConnectFlags {
    /// Return true if all flags are disabled (or if it's really empty).
    pub fn is_empty(&self) -> bool {
        for byte in &self.0 {
            if *byte > 0u8 {
                return false;
            }
        }
        true
    }
    /// Check flag SYNC
    pub fn sync(&self) -> bool {
        0b1111_1110 | self.0[0] == 255u8
    }
    /// Check flag ASK_SYNC_CHUNK
    pub fn ask_sync_chunk(&self) -> bool {
        0b1111_1101 | self.0[0] == 255u8
    }
    /// Check flag RES_SYNC_CHUNK
    pub fn res_sync_chunk(&self) -> bool {
        0b1111_1011 | self.0[0] == 255u8
    }
    /// Check flag CLIENT
    pub fn client(&self) -> bool {
        0b1111_0111 | self.0[0] == 255u8
    }
}

impl From<WS2Pv2ConnectType> for WS2PConnectFlags {
    fn from(connect_type: WS2Pv2ConnectType) -> Self {
        match connect_type {
            WS2Pv2ConnectType::Incoming | WS2Pv2ConnectType::OutgoingServer => {
                WS2PConnectFlags(vec![])
            }
            WS2Pv2ConnectType::OutgoingClient => WS2PConnectFlags(vec![8u8]),
            WS2Pv2ConnectType::Sync(_) => WS2PConnectFlags(vec![1u8]),
            WS2Pv2ConnectType::SyncAskChunk(_) => WS2PConnectFlags(vec![3u8]),
            WS2Pv2ConnectType::SyncSendChunks => WS2PConnectFlags(vec![5u8]),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
/// WS2Pv2ConnectType
pub enum WS2Pv2ConnectType {
    /// Incoming connection
    Incoming,
    /// Client outgoing connection
    OutgoingClient,
    /// Server outgoing connection
    OutgoingServer,
    /// Sync outgoing connection (from blockstamp, or from genesis block if blockstamp is none)
    Sync(Option<Blockstamp>),
    /// Sync outgoing connection to request chunk
    SyncAskChunk(Blockstamp),
    /// Sync outgoing connection to send chunk
    SyncSendChunks,
}

impl WS2Pv2ConnectType {
    /// Create WS2Pv2ConnectType from WS2PConnectFlags
    pub fn from_flags(
        flags: &WS2PConnectFlags,
        blockstamp: Option<Blockstamp>,
    ) -> WS2Pv2ConnectType {
        if !flags.is_empty() && flags.sync() {
            if flags.ask_sync_chunk() && blockstamp.is_some() {
                WS2Pv2ConnectType::SyncAskChunk(blockstamp.expect("safe unwrap"))
            } else if flags.res_sync_chunk() {
                WS2Pv2ConnectType::SyncSendChunks
            } else {
                WS2Pv2ConnectType::Sync(blockstamp)
            }
        } else {
            WS2Pv2ConnectType::OutgoingServer
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

/// Generate connect message
pub fn generate_connect_message(
    connect_type: WS2Pv2ConnectType,
    api_features: WS2PFeatures,
    challenge: Hash,
    peer_card: Option<PeerCardV11>,
) -> WS2Pv2ConnectMsg {
    let chunkstamp = if let WS2Pv2ConnectType::SyncAskChunk(chunkstamp) = connect_type {
        Some(chunkstamp)
    } else {
        None
    };
    WS2Pv2ConnectMsg {
        challenge,
        api_features,
        flags_queries: WS2PConnectFlags::from(connect_type),
        peer_card,
        chunkstamp,
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;
    use crate::tests::*;
    use dubp_documents::Blockstamp;
    use dup_crypto::keys::text_signable::TextSignable;

    #[test]
    fn test_ws2p_connect_flags() {
        // test sync()
        assert!(WS2PConnectFlags(vec![1u8]).sync());
        assert!(WS2PConnectFlags(vec![3u8]).sync());
        assert!(WS2PConnectFlags(vec![5u8]).sync());

        // test ask_sync_chunk()
        assert_eq!(WS2PConnectFlags(vec![1u8]).ask_sync_chunk(), false);
        assert!(WS2PConnectFlags(vec![3u8]).ask_sync_chunk());
        assert_eq!(WS2PConnectFlags(vec![5u8]).ask_sync_chunk(), false);

        // test res_sync_chunk()
        assert_eq!(WS2PConnectFlags(vec![1u8]).res_sync_chunk(), false);
        assert_eq!(WS2PConnectFlags(vec![3u8]).res_sync_chunk(), false);
        assert!(WS2PConnectFlags(vec![5u8]).res_sync_chunk());
    }

    #[test]
    fn test_ws2p_message_connect() {
        let keypair1 = keypair1();
        let mut peer = create_peer_card_v11();
        peer.sign(PrivKey::Ed25519(keypair1.private_key()))
            .expect("Fail to sign peer card !");
        let connect_msg = WS2Pv2ConnectMsg {
            challenge: Hash::from_hex(
                "000007722B243094269E548F600BD34D73449F7578C05BD370A6D301D20B5F10",
            )
            .unwrap(),
            api_features: WS2PFeatures(vec![7u8]),
            flags_queries: WS2PConnectFlags(vec![]),
            peer_card: Some(peer),
            chunkstamp: Some(
                Blockstamp::from_string(
                    "499-000011BABEEE1020B1F6B2627E2BC1C35BCD24375E114349634404D2C266D84F",
                )
                .unwrap(),
            ),
        };
        test_ws2p_message(WS2Pv2MessagePayload::Connect(Box::new(connect_msg)));
    }
}
