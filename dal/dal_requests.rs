extern crate duniter_crypto;
extern crate duniter_documents;
extern crate duniter_module;
extern crate serde;

use self::duniter_crypto::keys::ed25519;
use self::duniter_documents::blockchain::v10::documents::{
    BlockDocument, CertificationDocument, IdentityDocument, MembershipDocument, RevocationDocument,
};
use self::duniter_documents::Hash;
use self::duniter_module::ModuleReqFullId;
use std::collections::HashMap;

#[derive(Debug, Copy, Clone)]
pub enum DALReqPendings {
    AllPendingIdentyties(ModuleReqFullId, usize),
    AllPendingIdentytiesWithoutCerts(ModuleReqFullId, usize),
    PendingWotDatasForPubkey(ModuleReqFullId, ed25519::PublicKey),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DALReqBlockchain {
    CurrentBlock(ModuleReqFullId),
    BlockByNumber(ModuleReqFullId, u64),
    Chunk(ModuleReqFullId, u64, usize),
    UIDs(Vec<ed25519::PublicKey>),
}

#[derive(Debug, Clone)]
pub enum DALRequest {
    BlockchainRequest(DALReqBlockchain),
    PendingsRequest(DALReqPendings),
}

#[derive(Debug, Clone)]
pub struct PendingIdtyDatas {
    pub idty: IdentityDocument,
    pub memberships: Vec<MembershipDocument>,
    pub certs_count: usize,
    pub certs: Vec<CertificationDocument>,
    pub revocation: Option<RevocationDocument>,
}

#[derive(Debug, Clone)]
pub enum DALResPendings {
    AllPendingIdentyties(HashMap<Hash, PendingIdtyDatas>),
    AllPendingIdentytiesWithoutCerts(HashMap<Hash, PendingIdtyDatas>),
    PendingWotDatasForPubkey(Vec<PendingIdtyDatas>),
}

#[derive(Debug, Clone)]
pub enum DALResBlockchain {
    CurrentBlock(ModuleReqFullId, BlockDocument),
    BlockByNumber(ModuleReqFullId, BlockDocument),
    Chunk(ModuleReqFullId, Vec<BlockDocument>),
    UIDs(HashMap<ed25519::PublicKey, Option<String>>),
}

#[derive(Debug, Clone)]
pub enum DALResponse {
    Blockchain(DALResBlockchain),
    Pendings(ModuleReqFullId, DALResPendings),
}
