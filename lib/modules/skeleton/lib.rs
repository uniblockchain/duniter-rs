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

//! Module template to copy to create a new Durs module.

#![cfg_attr(feature = "strict", deny(warnings))]
#![deny(
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate structopt;

use duniter_conf::DuRsConf;
use duniter_module::*;
use duniter_network::events::NetworkEvent;
use durs_message::events::*;
use durs_message::*;
use std::ops::Deref;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime};

/// Name of your module
pub static MODULE_NAME: &'static str = "skeleton";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Skeleton Module Configuration
pub struct SkeletonConf {
    test_fake_conf_field: String,
}

impl Default for SkeletonConf {
    fn default() -> Self {
        SkeletonConf {
            test_fake_conf_field: String::from("default value"),
        }
    }
}

#[derive(Debug, Copy, Clone)]
/// Message from others thread of skeleton module
pub enum SkeletonThreadMsg {}

#[derive(Debug, Clone)]
/// Format of messages received by the skeleton module
pub enum SkeletonMsg {
    /// Message from another module
    DursMsg(Box<DursMsg>),
    /// Message from others thread of skeleton module
    SkeletonThreadMsg(SkeletonThreadMsg),
}

#[derive(StructOpt, Debug, Clone)]
#[structopt(
    name = "skeleton",
    raw(setting = "structopt::clap::AppSettings::ColoredHelp")
)]
/// Skeleton subcommand options
pub struct SkeletonOpt {
    /// Change test conf fake field
    pub new_conf_field: String,
}

#[derive(Debug, Clone)]
/// Data that the Skeleton module needs to cache
pub struct SkeletonModuleDatas {
    /// Sender of all child threads (except the proxy thread)
    pub child_threads: Vec<mpsc::Sender<SkeletonMsg>>,
    /// Any data
    pub field: usize,
}

#[derive(Debug, Copy, Clone)]
/// Skeleton module
pub struct SkeletonModule {}

impl Default for SkeletonModule {
    fn default() -> SkeletonModule {
        SkeletonModule {}
    }
}

impl DursModule<DuRsConf, DursMsg> for SkeletonModule {
    type ModuleConf = SkeletonConf;
    type ModuleOpt = SkeletonOpt;

    fn name() -> ModuleStaticName {
        ModuleStaticName(MODULE_NAME)
    }
    fn priority() -> ModulePriority {
        //ModulePriority::Recommended()
        ModulePriority::Optional()
    }
    fn ask_required_keys() -> RequiredKeys {
        RequiredKeys::None()
    }
    fn have_subcommand() -> bool {
        true
    }
    fn exec_subcommand(
        soft_meta_datas: &SoftwareMetaDatas<DuRsConf>,
        _keys: RequiredKeysContent,
        module_conf: Self::ModuleConf,
        subcommand_args: Self::ModuleOpt,
    ) {
        let mut conf = soft_meta_datas.conf.clone();
        let new_skeleton_conf = SkeletonConf {
            test_fake_conf_field: subcommand_args.new_conf_field.clone(),
        };
        conf.set_module_conf(
            MODULE_NAME.to_owned(),
            serde_json::value::to_value(new_skeleton_conf)
                .expect("Fail to jsonifie SkeletonConf !"),
        );
        duniter_conf::write_conf_file(&soft_meta_datas.profile, &conf)
            .expect("Fail to write new conf file ! ");
        println!(
            "Succesfully exec skeleton subcommand whit terminal name : {} and conf={:?}!",
            subcommand_args.new_conf_field, module_conf
        )
    }
    fn start(
        _soft_meta_datas: &SoftwareMetaDatas<DuRsConf>,
        _keys: RequiredKeysContent,
        _conf: Self::ModuleConf,
        router_sender: mpsc::Sender<RouterThreadMessage<DursMsg>>,
        load_conf_only: bool,
    ) -> Result<(), ModuleInitError> {
        let _start_time = SystemTime::now();

        // load conf
        if load_conf_only {
            // Check conf validity
            // ...
            let conf_valid = true;

            // If the configuration is valid, we return OK.
            if conf_valid {
                return Ok(());
            } else {
                // If the configuration is invalid, an error message is returned
                return Err(ModuleInitError::FailToLoadConf(
                    "write the details of the error here",
                ));
            }
        }

        // Instanciate Skeleton module datas
        let datas = SkeletonModuleDatas {
            child_threads: Vec::new(),
            field: 3,
        };

        // Create skeleton main thread channel
        let (skeleton_sender, skeleton_receiver): (
            mpsc::Sender<SkeletonMsg>,
            mpsc::Receiver<SkeletonMsg>,
        ) = mpsc::channel();

        // Create proxy channel
        let (proxy_sender, proxy_receiver): (mpsc::Sender<DursMsg>, mpsc::Receiver<DursMsg>) =
            mpsc::channel();

        // Launch a proxy thread that transform DursMsgContent() to SkeleonMsg::DursMsgContent(DursMsgContent())
        let router_sender_clone = router_sender.clone();
        let skeleton_sender_clone = skeleton_sender.clone();
        thread::spawn(move || {
            // Send skeleton module registration to router thread
            router_sender_clone
                .send(RouterThreadMessage::ModuleRegistration(
                    ModuleStaticName(MODULE_NAME),
                    proxy_sender, // Messages sent by the router will be received by your proxy thread
                    vec![ModuleRole::UserInterface], // Roles assigned to your module
                    vec![ModuleEvent::NewValidBlock], // Events to which your module subscribes
                    vec![],
                    vec![],
                ))
                .expect("Fatal error : skeleton module fail to register to router !"); // The registration of your module must be successful, in case of failure the program must be interrupted.

            // If we are here it means that your module has successfully registered, we indicate it in the debug level log, it can be helpful.
            debug!("Send skeleton module registration to router thread.");

            /*
             * Main loop of your proxy thread
             */
            loop {
                match proxy_receiver.recv() {
                    Ok(message) => {
                        let stop = if let DursMsg::Stop = message {
                            true
                        } else {
                            false
                        };
                        if skeleton_sender_clone
                            .send(SkeletonMsg::DursMsg(Box::new(message)))
                            .is_err()
                        {
                            // Log error
                            warn!(
                                "Skeleton proxy : fail to relay DursMsg to skeleton main thread !"
                            )
                        }
                        if stop {
                            break;
                        }
                    }
                    Err(e) => {
                        // Log error
                        warn!("{}", e);
                        break;
                    }
                }
            }
        });

        /*
         * Main loop of your module
         */
        loop {
            // Get messages
            match skeleton_receiver.recv_timeout(Duration::from_millis(250)) {
                Ok(ref message) => match *message {
                    SkeletonMsg::DursMsg(ref durs_message) => {
                        match durs_message.deref() {
                            DursMsg::Stop => {
                                // Relay stop signal to all child threads
                                let _result_stop_propagation: Result<
                                    (),
                                    mpsc::SendError<SkeletonMsg>,
                                > = datas
                                    .child_threads
                                    .iter()
                                    .map(|t| t.send(SkeletonMsg::DursMsg(Box::new(DursMsg::Stop))))
                                    .collect();
                                // Relay stop signal to router
                                let _result = router_sender
                                    .send(RouterThreadMessage::ModuleMessage(DursMsg::Stop));
                                // Break main loop
                                break;
                            }
                            DursMsg::Event {
                                ref event_content, ..
                            } => match *event_content {
                                DursEvent::BlockchainEvent(ref blockchain_event) => {
                                    match *blockchain_event.deref() {
                                        BlockchainEvent::StackUpValidBlock(ref _block) => {
                                            // Do something when the node has stacked a new block at its local blockchain
                                        }
                                        BlockchainEvent::RevertBlocks(ref _blocks) => {
                                            // Do something when the node has destacked blocks from its local blockchain (roll back)
                                        }
                                        _ => {} // Do nothing for events that don't concern your module.
                                    }
                                }
                                DursEvent::NetworkEvent(ref network_event_box) => {
                                    match *network_event_box.deref() {
                                        NetworkEvent::ReceivePeers(ref _peers) => {
                                            // Do something when the node receive peers cards from network
                                        }
                                        NetworkEvent::ReceiveDocuments(ref _bc_documents) => {
                                            // Do something when the node receive blockchain documents from network
                                        }
                                        _ => {} // Do nothing for events that don't concern your module.
                                    }
                                }
                                _ => {} // Do nothing for DursEvent variants that don't concern your module.
                            },
                            _ => {} // Do nothing for DursMsgContent variants that don't concern your module.
                        }
                    }
                    SkeletonMsg::SkeletonThreadMsg(ref _child_thread_msg) => {
                        // Do something when receive a message from child thread.
                    }
                },
                Err(e) => match e {
                    mpsc::RecvTimeoutError::Disconnected => {
                        panic!("Disconnected skeleton module !");
                    }
                    mpsc::RecvTimeoutError::Timeout => {
                        // If you arrive here it's because your main thread did not receive anything at the end of the timeout.
                        // This is quite normal and happens regularly when there is little activity, there is nothing particular to do.
                    }
                },
            }
            // If you want your module's main thread to do things even when it doesn't receive any messages, this is the place where it can do them.
            // ...
        }
        // If we reach this point it means that the module has stopped correctly, so we return OK.
        Ok(())
    }
}
