use std::collections::HashMap;

use ring::rand::SecureRandom;
use ring::signature;

use tokio::sync::mpsc;

use crate::data::DataManager;
use crate::platform::PeerConnectionManager;
use super::{Fabric, FabricProtoState};
use super::packet_sender::OrchPacketSender;
use super::state::FabricState;

pub struct FabricBuilder {
    auth_token: Option<String>,
    node_classes: Vec<String>,
    rand: Option<Box<dyn SecureRandom + Send>>,
    keypair: Option<signature::EcdsaKeyPair>,
}
impl FabricBuilder {
    pub fn new() -> Self {
        Self {
            auth_token: None,
            node_classes: Vec::new(),
            rand: None,
            keypair: None,
        }
    }

    pub fn with_auth_token(mut self, auth_token: String) -> Self {
        self.auth_token = Some(auth_token);
        self
    }

    pub fn with_node_class(mut self, class: String) -> Self {
        self.node_classes.push(class);
        self
    }
    pub fn with_node_classes(mut self, classes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for class in classes {
            self.node_classes.push(class.into());
        }
        self
    }

    pub fn with_random(mut self, rand: Box<dyn SecureRandom + Send>) -> Self {
        self.rand = Some(rand);
        self
    }

    pub fn with_keypair(mut self, keypair: signature::EcdsaKeyPair) -> Self {
        self.keypair = Some(keypair);
        self
    }

    pub fn start(
        self,
        sender: OrchPacketSender,
        peer_connector: Box<dyn PeerConnectionManager + Send>
    ) -> Fabric {
        let (recv_sender, recv_receiver) = mpsc::channel(3);

        let rand = self.rand.unwrap_or_else(|| {
            Box::new(ring::rand::SystemRandom::new())
        });

        let keypair = self.keypair.unwrap_or_else(|| {
            let algo = &signature::ECDSA_P256_SHA256_FIXED_SIGNING;
            let pkcs8_bytes = signature::EcdsaKeyPair::generate_pkcs8(
                algo, &*rand).unwrap();
            signature::EcdsaKeyPair::from_pkcs8(
                algo, pkcs8_bytes.as_ref()).unwrap()
        });

        let (peer_receiver_sender, peer_receiver) = mpsc::channel(3);

        let mut fabric_state = FabricState {
            proto_state: FabricProtoState::Handshake1,

            sender,
            receiver: recv_receiver,

            peer_connector,
            peer_receiver,
            peer_receiver_sender,

            peers: HashMap::new(),
            data_manager: DataManager::new(),

            node_classes: self.node_classes,
            auth_token: self.auth_token,

            uuid: None,
            keypair,

            orch_challenge: None,
            orch_pubkey: None,

            rand,
        };

        tokio::spawn(async move {
            fabric_state.start_handshake();
            fabric_state.main_loop().await;
        });

        Fabric {
            fabric_packet_in: recv_sender,
        }
    }
}
