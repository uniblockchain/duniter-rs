//  Copyright (C) 2017  The Duniter Project Developers.
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

//! Module defining the format of network heads v2 and how to handle them.

use duniter_crypto::keys::*;
use duniter_documents::Blockstamp;
use std::cmp::Ordering;
use std::ops::Deref;
use NodeId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Head Message V2
pub struct NetworkHeadMessageV2 {
    /// API details
    pub api: String,
    /// Head version
    pub version: usize,
    /// Head pubkey
    pub pubkey: PubKey,
    /// Head blockstamp
    pub blockstamp: Blockstamp,
    /// Head node id
    pub node_uuid: NodeId,
    /// Issuer node software
    pub software: String,
    /// Issuer node soft version
    pub soft_version: String,
    /// Issuer node prefix
    pub prefix: usize,
    /// Issuer node free member room
    pub free_member_room: Option<usize>,
    /// Issuer node free mirror room
    pub free_mirror_room: Option<usize>,
}

impl PartialOrd for NetworkHeadMessageV2 {
    fn partial_cmp(&self, other: &NetworkHeadMessageV2) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NetworkHeadMessageV2 {
    fn cmp(&self, other: &NetworkHeadMessageV2) -> Ordering {
        self.blockstamp.cmp(&other.blockstamp)
    }
}

impl NetworkHeadMessageV2 {
    /// To human readable string
    pub fn to_human_string(&self, max_len: usize, uid: Option<String>) -> String {
        let short_api = &self.api[4..];

        if max_len > 85 && uid.is_some() {
            format!(
                "{node_id:8}-{pubkey:.8} {blockstamp:.16} {soft:7}:{ver:14} {pre:3} [{api:5}]  {mer:02}:{mir:02} {uid}",
                node_id = self.node_uuid.to_string(),
                pubkey = self.pubkey.to_string(),
                blockstamp = self.blockstamp.to_string(),
                soft = self.software,
                ver = self.soft_version,
                pre = self.prefix,
                api = short_api,
                mer = self.free_member_room.unwrap_or(0),
                mir = self.free_mirror_room.unwrap_or(0),
                uid = uid.unwrap(),
            )
        } else if max_len > 75 {
            format!(
                "{node_id:8}-{pubkey:.8} {blockstamp:.16} {soft:7}:{ver:14} {pre:3} [{api:5}]  {mer:02}:{mir:02}",
                node_id = self.node_uuid.to_string(),
                pubkey = self.pubkey.to_string(),
                blockstamp = self.blockstamp.to_string(),
                soft = self.software,
                ver = self.soft_version,
                pre = self.prefix,
                api = short_api,
                mer = self.free_member_room.unwrap_or(0),
                mir = self.free_mirror_room.unwrap_or(0),
            )
        } else if max_len > 70 {
            format!(
                "{node_id:8}-{pubkey:.8} {blockstamp:.16} {soft:7}:{ver:14} [{api:5}]  {mer:02}:{mir:02}",
                node_id = self.node_uuid.to_string(),
                pubkey = self.pubkey.to_string(),
                blockstamp = self.blockstamp.to_string(),
                soft = self.software,
                ver = self.soft_version,
                api = short_api,
                mer = self.free_member_room.unwrap_or(0),
                mir = self.free_mirror_room.unwrap_or(0),
            )
        } else if max_len > 47 {
            format!(
                "{node_id:8}-{pubkey:.8} {blockstamp:.16} [{api:5}]  {mer:02}:{mir:02}",
                node_id = self.node_uuid.to_string(),
                pubkey = self.pubkey.to_string(),
                blockstamp = self.blockstamp.to_string(),
                api = short_api,
                mer = self.free_member_room.unwrap_or(0),
                mir = self.free_mirror_room.unwrap_or(0),
            )
        } else if max_len > 41 {
            format!(
                "{node_id:8}-{pubkey:.8} {blockstamp:.16} [{api:5}]",
                node_id = self.node_uuid.to_string(),
                pubkey = self.pubkey.to_string(),
                blockstamp = self.blockstamp.to_string(),
                api = short_api,
            )
        } else {
            String::from("Term width insufficient")
        }
    }
}

#[derive(Debug, Clone, Eq, Ord, PartialOrd, PartialEq, Hash, Serialize, Deserialize)]
/// Head Message
pub enum NetworkHeadMessage {
    /// Head Message V2
    V2(NetworkHeadMessageV2),
    /// Do not use
    Other(),
}

impl NetworkHeadMessage {
    /// To human readable string
    pub fn to_human_string(&self, max_len: usize, uid: Option<String>) -> String {
        match *self {
            NetworkHeadMessage::V2(ref mess_v2) => mess_v2.deref().to_human_string(max_len, uid),
            _ => panic!("NetworkHead version not supported !"),
        }
    }
    /// Parse head from string
    pub fn from_str(source: &str) -> Option<NetworkHeadMessage> {
        let source_array: Vec<&str> = source.split(':').collect();
        if let Ok(pubkey) = ed25519::PublicKey::from_base58(&source_array[3].to_string()) {
            Some(NetworkHeadMessage::V2(NetworkHeadMessageV2 {
                api: source_array[0].to_string(),
                version: source_array[2].parse().unwrap(),
                pubkey: PubKey::Ed25519(pubkey),
                blockstamp: Blockstamp::from_string(source_array[4]).unwrap(),
                node_uuid: NodeId(u32::from_str_radix(source_array[5], 16).unwrap()),
                software: source_array[6].to_string(),
                soft_version: source_array[7].to_string(),
                prefix: source_array[8].parse().unwrap(),
                free_member_room: if let Some(field) = source_array.get(9) {
                    Some(field.parse().unwrap())
                } else {
                    None
                },
                free_mirror_room: if let Some(field) = source_array.get(10) {
                    Some(field.parse().unwrap())
                } else {
                    None
                },
            }))
        } else {
            None
        }
    }
    /// Get head blockcstamp
    pub fn blockstamp(&self) -> Blockstamp {
        match *self {
            NetworkHeadMessage::V2(ref head_message_v2) => head_message_v2.blockstamp,
            _ => panic!("This HEAD version is not supported !"),
        }
    }
    /// Get head node id
    pub fn node_uuid(&self) -> NodeId {
        match *self {
            NetworkHeadMessage::V2(ref head_message_v2) => head_message_v2.node_uuid,
            _ => panic!("This HEAD version is not supported !"),
        }
    }
    /// Get head issuer public key
    fn _pubkey(&self) -> PubKey {
        match *self {
            NetworkHeadMessage::V2(ref head_message_v2) => head_message_v2.pubkey,
            _ => panic!("This HEAD version is not supported !"),
        }
    }
}

impl ToString for NetworkHeadMessageV2 {
    fn to_string(&self) -> String {
        match self.version {
            1 => format!(
                "{}:HEAD:1:{}:{}:{}:{}:{}:{}",
                self.api,
                self.pubkey,
                self.blockstamp,
                self.node_uuid,
                self.software,
                self.soft_version,
                self.prefix
            ),
            2 => format!(
                "{}:HEAD:2:{}:{}:{}:{}:{}:{}:{}:{}",
                self.api,
                self.pubkey,
                self.blockstamp,
                self.node_uuid,
                self.software,
                self.soft_version,
                self.prefix,
                self.free_member_room.unwrap(),
                self.free_mirror_room.unwrap()
            ),
            _ => panic!("NetworkHeadMessage is wrongly parsed !"),
        }
    }
}

impl ToString for NetworkHeadMessage {
    fn to_string(&self) -> String {
        match *self {
            NetworkHeadMessage::V2(ref head_message) => head_message.to_string(),
            _ => panic!("This HEADMessage version is not supported !"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Head V2
pub struct NetworkHeadV2 {
    /// Head V1 Message
    pub message: NetworkHeadMessage,
    /// signature of V1 Message
    pub sig: Sig,
    /// Head V2 Message
    pub message_v2: NetworkHeadMessage,
    /// signature of V2 Message
    pub sig_v2: Sig,
    /// Head step
    pub step: u32,
    /// Head issuer uid
    pub uid: Option<String>,
}

impl ToString for NetworkHeadV2 {
    fn to_string(&self) -> String {
        self.message_v2.to_string()
    }
}

impl PartialOrd for NetworkHeadV2 {
    fn partial_cmp(&self, other: &NetworkHeadV2) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NetworkHeadV2 {
    fn cmp(&self, other: &NetworkHeadV2) -> Ordering {
        self.message.cmp(&other.message)
    }
}

impl NetworkHeadV2 {
    /// To human readable string
    pub fn to_human_string(&self, max_len: usize) -> String {
        if max_len > 2 {
            format!(
                "{} {}",
                self.step,
                self.message_v2
                    .to_human_string(max_len - 2, self.uid.clone())
            )
        } else {
            String::from(".")
        }
    }
    /// Get uid of head issuer
    pub fn uid(&self) -> Option<String> {
        self.uid.clone()
    }
}
