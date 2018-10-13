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

//! Relay messages between durs modules.

use duniter_message::*;
use duniter_module::*;
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::mpsc::RecvTimeoutError;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;

/// Start broadcasting thread
fn start_broadcasting_thread(
    start_time: SystemTime,
    run_duration_in_secs: u64,
    receiver: &mpsc::Receiver<RooterThreadMessage<DursMsg>>,
    external_followers: &[mpsc::Sender<DursMsgContent>],
) {
    // Define variables
    let mut modules_senders: HashMap<ModuleStaticName, mpsc::Sender<DursMsg>> = HashMap::new();
    let mut pool_msgs: HashMap<DursMsgReceiver, Vec<DursMsgContent>> = HashMap::new();
    let mut events_subscriptions: HashMap<ModuleEvent, Vec<ModuleStaticName>> = HashMap::new();
    let mut roles: HashMap<ModuleRole, Vec<ModuleStaticName>> = HashMap::new();

    loop {
        match receiver.recv_timeout(Duration::from_secs(1)) {
            Ok(mess) => {
                match mess {
                    RooterThreadMessage::ModuleSender(
                        module_static_name,
                        module_sender,
                        sender_roles,
                        events_subscription,
                    ) => {
                        // For all events
                        for event in events_subscription {
                            // Send pending message of this event
                            for msg in pool_msgs
                                .get(&DursMsgReceiver::Event(event))
                                .unwrap_or(&Vec::with_capacity(0))
                            {
                                module_sender
                                    .send(DursMsg(DursMsgReceiver::Event(event), msg.clone()))
                                    .unwrap_or_else(|_| {
                                        panic!(
                                            "Fatal error: fail to relay DursMsg to {:?} !",
                                            module_static_name
                                        )
                                    });
                            }
                            // Store event subscription
                            events_subscriptions
                                .entry(event)
                                .or_insert_with(Vec::new)
                                .push(module_static_name);
                        }
                        // For all roles
                        for role in sender_roles {
                            // Send pending message for this role
                            for msg in pool_msgs
                                .get(&DursMsgReceiver::Role(role))
                                .unwrap_or(&Vec::with_capacity(0))
                            {
                                module_sender
                                    .send(DursMsg(DursMsgReceiver::Role(role), msg.clone()))
                                    .unwrap_or_else(|_| {
                                        panic!(
                                            "Fatal error: fail to relay DursMsg to {:?} !",
                                            module_static_name
                                        )
                                    });
                            }
                            // Store sender roles
                            roles
                                .entry(role)
                                .or_insert_with(Vec::new)
                                .push(module_static_name);
                        }
                        // Add this sender to modules_senders
                        modules_senders.insert(module_static_name, module_sender);
                    }
                    RooterThreadMessage::ModuleMessage(msg) => match msg.0 {
                        DursMsgReceiver::One(_) => {}
                        DursMsgReceiver::All => {
                            for (module_static_name, module_sender) in &modules_senders {
                                module_sender.send(msg.clone()).unwrap_or_else(|_| {
                                    panic!(
                                        "Fatal error: fail to relay DursMsg to {:?} !",
                                        module_static_name
                                    )
                                });
                            }
                            // Detect stop message
                            let stop = if let DursMsgContent::Stop() = msg.1 {
                                true
                            } else {
                                false
                            };
                            // Send message to external followers
                            for external_follower in external_followers {
                                external_follower.send(msg.1.clone()).expect(
                                    "Fatal error: fail to relay DursMsg to external followers !",
                                );
                            }
                            // Send message to all modules
                            send_msg_to_several_receivers(
                                msg,
                                &modules_senders
                                    .keys()
                                    .cloned()
                                    .collect::<Vec<ModuleStaticName>>(),
                                &modules_senders,
                            );
                            // Stop thread if its requested
                            if stop {
                                break;
                            }
                        }
                        DursMsgReceiver::Event(event) => {
                            // the node to be started less than 20 seconds ago,
                            // keep the message in memory to be able to send it back to modules not yet plugged
                            store_msg_in_pool(
                                start_time,
                                run_duration_in_secs,
                                msg.clone(),
                                &mut pool_msgs,
                            );
                            // Get list of receivers
                            let receivers = events_subscriptions
                                .get(&event)
                                .unwrap_or(&Vec::with_capacity(0))
                                .to_vec();
                            // Send msg to receivers
                            send_msg_to_several_receivers(msg, &receivers, &modules_senders)
                        }
                        DursMsgReceiver::Role(role) => {
                            // If the node to be started less than 20 seconds ago,
                            // keep the message in memory to be able to send it back to modules not yet plugged
                            store_msg_in_pool(
                                start_time,
                                run_duration_in_secs,
                                msg.clone(),
                                &mut pool_msgs,
                            );
                            // Get list of receivers
                            let receivers =
                                roles.get(&role).unwrap_or(&Vec::with_capacity(0)).to_vec();
                            // Send msg to receivers
                            send_msg_to_several_receivers(msg, &receivers, &modules_senders)
                        }
                    },
                }
            }
            Err(e) => match e {
                RecvTimeoutError::Timeout => continue,
                RecvTimeoutError::Disconnected => {
                    panic!("Fatal error : rooter thread disconnnected !")
                }
            },
        }
    }
}

/// Send msg to several receivers
fn send_msg_to_several_receivers(
    msg: DursMsg,
    receivers: &[ModuleStaticName],
    modules_senders: &HashMap<ModuleStaticName, mpsc::Sender<DursMsg>>,
) {
    // Send message by copy To all modules that subscribed to this event
    for module_static_name in &receivers[1..] {
        if let Some(module_sender) = modules_senders.get(module_static_name) {
            module_sender.send(msg.clone()).unwrap_or_else(|_| {
                panic!(
                    "Fatal error: fail to relay DursMsg to {:?} !",
                    module_static_name
                )
            });
        }
    }
    // Send message by move to the last module to be revceive
    if !receivers.is_empty() {
        if let Some(module_sender) = modules_senders.get(&receivers[0]) {
            module_sender.send(msg).unwrap_or_else(|_| {
                panic!("Fatal error: fail to relay DursMsg to {:?} !", receivers[0])
            });
        }
    }
}

/// If the node to be started less than 20 seconds ago,
/// keep the message in memory to be able to send it back to modules not yet plugged
fn store_msg_in_pool(
    start_time: SystemTime,
    run_duration_in_secs: u64,
    msg: DursMsg,
    pool_msgs: &mut HashMap<DursMsgReceiver, Vec<DursMsgContent>>,
) {
    if run_duration_in_secs > 0
        && SystemTime::now()
            .duration_since(start_time)
            .expect("Duration error !")
            .as_secs()
            < 20
    {
        pool_msgs.entry(msg.0).or_insert_with(Vec::new).push(msg.1);
    } else if !pool_msgs.is_empty() {
        // Clear pool_msgs
        pool_msgs.clear();
    }
}

/// Start rooter thread
pub fn start_rooter<DC: DuniterConf>(
    run_duration_in_secs: u64,
    external_followers: Vec<mpsc::Sender<DursMsgContent>>,
) -> mpsc::Sender<RooterThreadMessage<DursMsg>> {
    let start_time = SystemTime::now();

    // Create rooter channel
    let (rooter_sender, rooter_receiver): (
        mpsc::Sender<RooterThreadMessage<DursMsg>>,
        mpsc::Receiver<RooterThreadMessage<DursMsg>>,
    ) = mpsc::channel();

    // Create rooter thread
    thread::spawn(move || {
        // Create broadcasting thread channel
        let (broadcasting_sender, broadcasting_receiver): (
            mpsc::Sender<RooterThreadMessage<DursMsg>>,
            mpsc::Receiver<RooterThreadMessage<DursMsg>>,
        ) = mpsc::channel();

        // Create broadcasting thread
        thread::spawn(move || {
            start_broadcasting_thread(
                start_time,
                run_duration_in_secs,
                &broadcasting_receiver,
                &external_followers,
            );
        });

        // Define variables
        let mut modules_senders: HashMap<ModuleStaticName, mpsc::Sender<DursMsg>> = HashMap::new();
        let mut pool_msgs: HashMap<ModuleStaticName, Vec<DursMsgContent>> = HashMap::new();

        // Wait to receiver modules senders
        loop {
            match rooter_receiver.recv_timeout(Duration::from_secs(1)) {
                Ok(mess) => {
                    match mess {
                        RooterThreadMessage::ModuleSender(
                            module_static_name,
                            module_sender,
                            events_subscription,
                            sender_roles,
                        ) => {
                            // Send pending messages destined specifically to this module
                            if let Some(msgs) = pool_msgs.remove(&module_static_name) {
                                for msg in msgs {
                                    module_sender
                                        .send(DursMsg(
                                            DursMsgReceiver::One(module_static_name),
                                            msg,
                                        ))
                                        .unwrap_or_else(|_| {
                                            panic!(
                                                "Fatal error: fail to relay DursMsg to {:?} !",
                                                module_static_name
                                            )
                                        });
                                }
                            }
                            // Add this sender to modules_senders
                            modules_senders.insert(module_static_name, module_sender.clone());
                            // Relay to broadcasting thread
                            broadcasting_sender
                                .send(RooterThreadMessage::ModuleSender(
                                    module_static_name,
                                    module_sender,
                                    events_subscription,
                                    sender_roles,
                                ))
                                .expect("Fail to relay message to broadcasting thread !");
                            // Log the number of modules_senders received
                            info!(
                                "Rooter thread receive {} module senders",
                                modules_senders.len()
                            );
                        }
                        RooterThreadMessage::ModuleMessage(msg) => {
                            trace!("Rooter thread receive ModuleMessage({:?})", msg);
                            match msg.0 {
                                DursMsgReceiver::All => {
                                    let stop = if let DursMsgContent::Stop() = msg.1 {
                                        true
                                    } else {
                                        false
                                    };
                                    broadcasting_sender
                                        .send(RooterThreadMessage::ModuleMessage(msg))
                                        .expect("Fail to relay message to broadcasting thread !");
                                    if stop {
                                        break;
                                    }
                                }
                                DursMsgReceiver::Role(_module_role) => broadcasting_sender
                                    .send(RooterThreadMessage::ModuleMessage(msg))
                                    .expect("Fail to relay message to broadcasting thread !"),
                                DursMsgReceiver::Event(_module_event) => broadcasting_sender
                                    .send(RooterThreadMessage::ModuleMessage(msg))
                                    .expect("Fail to relay message to broadcasting thread !"),
                                DursMsgReceiver::One(module_static_name) => {
                                    if let Some(module_sender) =
                                        modules_senders.get(&module_static_name)
                                    {
                                        module_sender.send(msg).unwrap_or_else(|_| {
                                            panic!(
                                                "Fatal error: fail to relay DursMsg to {:?} !",
                                                module_static_name
                                            )
                                        });
                                    } else if SystemTime::now()
                                        .duration_since(start_time)
                                        .expect("Duration error !")
                                        .as_secs()
                                        < 20
                                    {
                                        pool_msgs
                                            .entry(module_static_name)
                                            .or_insert_with(Vec::new)
                                            .push(msg.1);
                                    } else {
                                        if !pool_msgs.is_empty() {
                                            pool_msgs = HashMap::with_capacity(0);
                                        }
                                        warn!(
                                            "Message for unknow receiver : {:?}.",
                                            module_static_name
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => match e {
                    RecvTimeoutError::Timeout => continue,
                    RecvTimeoutError::Disconnected => {
                        panic!("Fatal error : rooter thread disconnnected !")
                    }
                },
            }
            if run_duration_in_secs > 0
                && SystemTime::now()
                    .duration_since(start_time)
                    .expect("Duration error !")
                    .as_secs()
                    > run_duration_in_secs
            {
                broadcasting_sender
                    .send(RooterThreadMessage::ModuleMessage(DursMsg(
                        DursMsgReceiver::All,
                        DursMsgContent::Stop(),
                    )))
                    .expect("Fail to relay stop message to broadcasting thread !");
                break;
            }
        }
        info!("Rooter thread stop.")
    });

    rooter_sender
}