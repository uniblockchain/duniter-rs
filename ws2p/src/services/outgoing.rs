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

//! WS2P outgoing Services

use duniter_documents::CurrencyName;
use durs_network_documents::NodeFullId;
use services::*;
use std::collections::HashMap;
use std::sync::mpsc;
use *;

#[derive(Debug, Clone)]
/// Data allowing the service to manage an outgoing connection
pub struct OutgoingConnection {
    /// Endpoint
    pub endpoint: EndpointEnum,
    /// Controller channel
    pub controller: mpsc::Sender<Ws2pControllerOrder>,
}

#[derive(Debug, Copy, Clone)]
/// Endpoind whose last connection attempt failed
pub struct EndpointInError {
    /// Last attemp time
    pub last_attempt_time: u64,
    /// Error status
    pub error: WS2PConnectionState,
}

#[derive(Debug)]
/// Outgoing connection management service
pub struct WS2POutgoingService {
    /// Currency Name
    pub currency: CurrencyName,
    /// Local node datas
    pub self_node: MySelfWs2pNode,
    /// Outgoing connections quota
    pub quota: usize,
    /// List of established connections
    pub connections: HashMap<NodeFullId, OutgoingConnection>,
    /// List of endpoinds whose last connection attempt failed
    pub endpoints_in_error: HashMap<NodeFullId, EndpointInError>,
    /// List of endpoints that have never been contacted
    pub never_try_endpoints: Vec<EndpointEnum>,
    /// Service receiver
    pub receiver: mpsc::Receiver<Ws2pServiceSender>,
    /// Service sender
    pub sender: mpsc::Sender<Ws2pServiceSender>,
}

impl WS2POutgoingService {
    /// Instantiate WS2POutgoingService
    pub fn new(
        currency: CurrencyName,
        ws2p_conf: &WS2PConf,
        self_node: MySelfWs2pNode,
    ) -> WS2POutgoingService {
        // Create service channel
        let (sender, receiver): (
            mpsc::Sender<Ws2pServiceSender>,
            mpsc::Receiver<Ws2pServiceSender>,
        ) = mpsc::channel();

        WS2POutgoingService {
            currency,
            quota: ws2p_conf.outcoming_quota,
            connections: HashMap::with_capacity(ws2p_conf.outcoming_quota),
            endpoints_in_error: HashMap::new(),
            never_try_endpoints: Vec::new(),
            self_node,
            receiver,
            sender,
        }
    }

    /// Connect to WSPv2 Endpoint
    pub fn connect_to_ws2p_v2_endpoint(&self, endpoint: &EndpointEnum) -> Result<(), WsError> {
        match controllers::outgoing_connections::connect_to_ws2p_v2_endpoint(
            &self.currency,
            &self.sender,
            &self.self_node,
            endpoint.node_full_id(),
            endpoint,
        ) {
            Ok(_) => Ok(()),
            Err(_) => Err(WsError::UnknownError),
        }
    }
}