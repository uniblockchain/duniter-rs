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

mod apply;
mod download;

use crate::*;
use dubp_documents::{BlockHash, BlockId};
use dup_crypto::keys::*;
use durs_blockchain_dal::currency_params::CurrencyParameters;
use durs_blockchain_dal::writers::requests::*;
use durs_blockchain_dal::ForkId;
use durs_common_tools::fatal_error;
use durs_wot::NodeId;
use pbr::ProgressBar;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::sync::mpsc;
use std::thread;
use std::time::SystemTime;
use threadpool::ThreadPool;

/// Number of sync jobs
pub static NB_SYNC_JOBS: &'static usize = &4;

/*#[derive(Debug)]
/// Sync source
enum SyncSource {
    Network(String),
    LocalJsonFiles(PathBuf),
}*/

#[derive(Debug, Clone, PartialEq, Eq)]
/// Block header
pub struct BlockHeader {
    pub number: BlockId,
    pub hash: BlockHash,
    pub issuer: PubKey,
}

#[derive(Debug)]
/// Message for main sync thread
pub enum MessForSyncThread {
    Target(CurrencyName, Blockstamp),
    BlockDocument(Box<BlockDocument>),
    DownloadFinish(),
    ApplyFinish(),
}

#[derive(Debug)]
/// Message for a job thread
pub enum SyncJobsMess {
    BlocksDBsWriteQuery(BlocksDBsWriteQuery),
    WotsDBsWriteQuery(WotsDBsWriteQuery, Box<CurrencyParameters>),
    CurrencyDBsWriteQuery(CurrencyDBsWriteQuery),
    End(),
}

/// Sync
pub fn sync<DC: DuniterConf>(
    profile: &str,
    conf: &DC,
    //source: SyncSource,
    json_files_path: PathBuf,
    end: Option<u32>,
    cautious: bool,
    verif_inner_hash: bool,
) {
    // Get verification level
    let _verif_level = if cautious {
        println!("Start cautious sync...");
        info!("Start cautious sync...");
        SyncVerificationLevel::Cautious()
    } else {
        println!("Start fast sync...");
        info!("Start fast sync...");
        SyncVerificationLevel::FastSync()
    };

    // Create sync_thread channels
    let (sender_sync_thread, recv_sync_thread) = mpsc::channel();

    // Create ThreadPool
    let nb_cpus = num_cpus::get();
    let nb_workers = if nb_cpus < *NB_SYNC_JOBS {
        nb_cpus
    } else {
        *NB_SYNC_JOBS
    };
    let pool = ThreadPool::new(nb_workers);

    //match source {
    //SyncSource::LocalJsonFiles(json_files_path) => {
    // json_files_path must be a directory
    if !json_files_path.is_dir() {
        error!("json_files_path must be a directory");
        panic!("json_files_path must be a directory");
    }

    // Lauch json reader thread
    download::json_reader_worker::json_reader_worker(
        &pool,
        profile,
        sender_sync_thread.clone(),
        json_files_path,
        end,
    );
    //}
    //SyncSource::Network(url) => unimplemented!(),
    //}

    // Get target blockstamp
    let (currency, target_blockstamp) =
        if let Ok(MessForSyncThread::Target(currency, target_blockstamp)) = recv_sync_thread.recv()
        {
            (currency, target_blockstamp)
        } else {
            fatal_error("Fatal error : no target blockstamp !");
            panic!(); // for compilator
        };

    // Update DuniterConf
    let mut conf = conf.clone();
    conf.set_currency(currency.clone());

    // Get databases path
    let db_path = duniter_conf::get_blockchain_db_path(profile, &currency);

    // Write new conf
    duniter_conf::write_conf_file(profile, &conf).expect("Fail to write new conf !");

    // Open wot db
    let wot_db = open_wot_db::<RustyWebOfTrust>(Some(&db_path)).expect("Fail to open WotDB !");

    // Open blocks databases
    let databases = BlocksV10DBs::open(Some(&db_path));

    // Open wot databases
    let wot_databases = WotsV10DBs::open(Some(&db_path));

    // Get local current blockstamp
    debug!("Get local current blockstamp...");
    let mut current_blockstamp: Blockstamp =
        durs_blockchain_dal::block::get_current_blockstamp(&databases)
            .expect("ForksV10DB : RustBreakError !")
            .unwrap_or_default();
    debug!("Success to get local current blockstamp.");

    // Node is already synchronized ?
    if target_blockstamp.id.0 < current_blockstamp.id.0 {
        println!("Your duniter-rs node is already synchronized.");
        return;
    }

    // Get wot index
    let mut wot_index: HashMap<PubKey, NodeId> =
        DALIdentity::get_wot_index(&wot_databases.identities_db)
            .expect("Fatal eror : get_wot_index : Fail to read blockchain databases");

    // Start sync
    let sync_start_time = SystemTime::now();

    // Createprogess bar
    let count_blocks = target_blockstamp.id.0 + 1 - current_blockstamp.id.0;
    let count_chunks = if count_blocks % 250 > 0 {
        (count_blocks / 250) + 1
    } else {
        count_blocks / 250
    };
    let mut apply_pb = ProgressBar::new(count_chunks.into());
    apply_pb.format("╢▌▌░╟");

    // Create workers threads channels
    let (sender_blocks_thread, recv_blocks_thread) = mpsc::channel();
    let (sender_wot_thread, recv_wot_thread) = mpsc::channel();
    let (sender_tx_thread, recv_tx_thread) = mpsc::channel();

    // Launch blocks_worker thread
    apply::blocks_worker::execute(
        &pool,
        sender_sync_thread.clone(),
        recv_blocks_thread,
        databases,
        apply_pb,
    );

    // / Launch wot_worker thread
    apply::wot_worker::execute(
        &pool,
        profile.to_owned(),
        currency.clone(),
        sender_sync_thread.clone(),
        recv_wot_thread,
    );

    // Launch tx_worker thread
    apply::txs_worker::execute(
        &pool,
        profile.to_owned(),
        currency.clone(),
        sender_sync_thread.clone(),
        recv_tx_thread,
    );

    let main_job_begin = SystemTime::now();

    // Open currency_params_db
    let dbs_path = duniter_conf::get_blockchain_db_path(profile, &conf.currency());
    let currency_params_db = open_file_db::<CurrencyParamsV10Datas>(&dbs_path, "params.db")
        .expect("Fail to open params db");

    // Apply blocks
    let mut blocks_not_expiring = VecDeque::with_capacity(200_000);
    let mut last_block_expiring: isize = -1;
    let certs_db =
        BinDB::Mem(open_memory_db::<CertsExpirV10Datas>().expect("Fail to create memory certs_db"));
    let mut currency_params = CurrencyParameters::default();
    let mut get_currency_params = false;
    let mut certs_count = 0;

    let mut all_wait_duration = Duration::from_millis(0);
    let mut wait_begin = SystemTime::now();
    let mut all_verif_block_hashs_duration = Duration::from_millis(0);
    let mut all_apply_valid_block_duration = Duration::from_millis(0);
    while let Ok(MessForSyncThread::BlockDocument(block_doc)) = recv_sync_thread.recv() {
        all_wait_duration += SystemTime::now().duration_since(wait_begin).unwrap();
        let block_doc = block_doc.deref();
        // Verify block hashs
        let verif_block_hashs_begin = SystemTime::now();
        if verif_inner_hash {
            verify_block_hashs(&block_doc)
                .expect("Receive wrong block, please reset data and resync !");
        }
        all_verif_block_hashs_duration += SystemTime::now()
            .duration_since(verif_block_hashs_begin)
            .unwrap();
        // Get currency params
        if !get_currency_params && block_doc.number.0 == 0 {
            if block_doc.parameters.is_some() {
                currency_params_db
                    .write(|db| {
                        db.0 = block_doc.currency.clone();
                        db.1 = block_doc.parameters.unwrap();
                    })
                    .expect("fail to write in params DB");
                currency_params = CurrencyParameters::from((
                    block_doc.currency.clone(),
                    block_doc.parameters.unwrap(),
                ));
                get_currency_params = true;
            } else {
                panic!("The genesis block are None parameters !");
            }
        }
        // Push block median_time in blocks_not_expiring
        blocks_not_expiring.push_back(block_doc.median_time);
        // Get blocks_expiring
        let mut blocks_expiring = Vec::new();
        while blocks_not_expiring.front().cloned()
            < Some(block_doc.median_time - currency_params.sig_validity)
        {
            last_block_expiring += 1;
            blocks_expiring.push(BlockId(last_block_expiring as u32));
            blocks_not_expiring.pop_front();
        }
        // Find expire_certs
        let expire_certs =
            durs_blockchain_dal::certs::find_expire_certs(&certs_db, blocks_expiring)
                .expect("find_expire_certs() : DALError");
        // Apply block
        let apply_valid_block_begin = SystemTime::now();
        if let Ok(ValidBlockApplyReqs(block_req, wot_db_reqs, currency_db_reqs)) =
            apply_valid_block::<RustyWebOfTrust>(
                &block_doc,
                &mut wot_index,
                &wot_db,
                &expire_certs,
                None,
            )
        {
            all_apply_valid_block_duration += SystemTime::now()
                .duration_since(apply_valid_block_begin)
                .unwrap();
            current_blockstamp = block_doc.blockstamp();
            debug!("Apply db requests...");
            // Send block request to blocks worker thread
            sender_blocks_thread
                .send(SyncJobsMess::BlocksDBsWriteQuery(block_req.clone()))
                .expect(
                    "Fail to communicate with blocks worker thread, please reset data & resync !",
                );
            // Send wot requests to wot worker thread
            for req in wot_db_reqs {
                if let WotsDBsWriteQuery::CreateCert(
                    ref _source_pubkey,
                    ref source,
                    ref target,
                    ref created_block_id,
                    ref _median_time,
                ) = req
                {
                    certs_count += 1;
                    // Add cert in certs_db
                    certs_db
                        .write(|db| {
                            let mut created_certs =
                                db.get(&created_block_id).cloned().unwrap_or_default();
                            created_certs.insert((*source, *target));
                            db.insert(*created_block_id, created_certs);
                        })
                        .expect("RustBreakError : please reset data and resync !");
                }
                sender_wot_thread
                    .send(SyncJobsMess::WotsDBsWriteQuery(
                        req.clone(),
                        Box::new(currency_params),
                    ))
                    .expect(
                        "Fail to communicate with tx worker thread, please reset data & resync !",
                    )
            }
            // Send blocks and wot requests to wot worker thread
            for req in currency_db_reqs {
                sender_tx_thread
                    .send(SyncJobsMess::CurrencyDBsWriteQuery(req.clone()))
                    .expect(
                        "Fail to communicate with tx worker thread, please reset data & resync !",
                    );
            }
            debug!("Success to apply block #{}", current_blockstamp.id.0);
            if current_blockstamp.id.0 >= target_blockstamp.id.0 {
                if current_blockstamp == target_blockstamp {
                    // Sync completed
                    break;
                } else {
                    panic!("Fatal Error : we get a fork, please reset data and sync again !");
                }
            }
        } else {
            panic!(
                "Fatal error : fail to stack up block #{}",
                current_blockstamp.id.0 + 1
            )
        }
        wait_begin = SystemTime::now();
    }
    // Send end signal to workers threads
    sender_blocks_thread
        .send(SyncJobsMess::End())
        .expect("Sync : Fail to send End signal to blocks worker !");
    info!("Sync : send End signal to blocks job.");
    sender_wot_thread
        .send(SyncJobsMess::End())
        .expect("Sync : Fail to send End signal to wot worker !");
    info!("Sync : send End signal to wot job.");
    sender_tx_thread
        .send(SyncJobsMess::End())
        .expect("Sync : Fail to send End signal to writer worker !");
    info!("Sync : send End signal to tx job.");

    // Save params db
    currency_params_db.save().expect("Fail to save params db");

    // Save wot file
    wot_db.save().expect("Fail to save wot db");

    let main_job_duration =
        SystemTime::now().duration_since(main_job_begin).unwrap() - all_wait_duration;
    info!(
        "main_job_duration={},{:03} seconds.",
        main_job_duration.as_secs(),
        main_job_duration.subsec_millis()
    );
    info!(
        "all_verif_block_hashs_duration={},{:03} seconds.",
        all_verif_block_hashs_duration.as_secs(),
        all_verif_block_hashs_duration.subsec_millis()
    );
    info!(
        "all_apply_valid_block_duration={},{:03} seconds.",
        all_apply_valid_block_duration.as_secs(),
        all_apply_valid_block_duration.subsec_millis()
    );

    // Wait recv two finish signals
    let mut wait_jobs = *NB_SYNC_JOBS - 1;
    while wait_jobs > 0 {
        match recv_sync_thread.recv() {
            Ok(MessForSyncThread::ApplyFinish()) => wait_jobs -= 1,
            Ok(_) => thread::sleep(Duration::from_millis(50)),
            Err(_) => wait_jobs -= 1,
        }
    }
    info!("All sync jobs finish.");

    // Log sync duration
    debug!("certs_count={}", certs_count);
    let sync_duration = SystemTime::now().duration_since(sync_start_time).unwrap();
    println!(
        "Sync {} blocks in {}.{:03} seconds.",
        current_blockstamp.id.0 + 1,
        sync_duration.as_secs(),
        sync_duration.subsec_millis(),
    );
    info!(
        "Sync {} blocks in {}.{:03} seconds.",
        current_blockstamp.id.0 + 1,
        sync_duration.as_secs(),
        sync_duration.subsec_millis(),
    );
}
