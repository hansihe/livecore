use std::collections::HashMap;

use ring::signature::{self, KeyPair};
use ring::rand::SecureRandom;

use tokio::sync::mpsc;

use livecore_protocol as proto;
use proto::Uuid;

use crate::platform::{PeerConnectionManager, PeerTunnel};
use super::packet_sender::OrchPacketSender;

const HANDSHAKE_CHALLENGE_WRAP: &str = "__HANDSHAKE_CHALLENGE__";
const CHALLENGE_RESPONSE_LEN: usize = (HANDSHAKE_CHALLENGE_WRAP.len() * 2) + (32 * 2);

#[derive(Debug, PartialEq, Eq)]
pub enum FabricProtoState {
    Handshake1,
    Handshake2,
    Normal,
}

pub struct FabricState {
    pub(crate) proto_state: FabricProtoState,

    pub(crate) sender: OrchPacketSender,
    pub(crate) receiver: mpsc::Receiver<proto::OrchServerMsg>,

    pub(crate) peer_connector: Box<dyn PeerConnectionManager + Send>,
    pub(crate) peer_receiver: mpsc::Receiver<PeerConnMsg>,
    pub(crate) peer_receiver_sender: mpsc::Sender<PeerConnMsg>,

    pub(crate) peers: HashMap<Uuid, crate::peer::PeerState>,

    pub(crate) data_manager: crate::data::DataManager,

    pub(crate) node_classes: Vec<String>,
    pub(crate) auth_token: Option<String>,

    pub(crate) uuid: Option<Uuid>,
    pub(crate) keypair: signature::EcdsaKeyPair,

    pub(crate) orch_challenge: Option<[u8; 32]>,
    pub(crate) orch_pubkey: Option<signature::UnparsedPublicKey<Vec<u8>>>,

    pub(crate) rand: Box<dyn SecureRandom + Send>,
}

pub(crate) struct PeerConnMsg {
    pub uuid: Uuid,
    pub kind: PeerConnMsgKind,
}

pub(crate) enum PeerConnMsgKind {
    Connected,
}

impl FabricState {

    pub async fn main_loop(mut self) {
        loop {
            tokio::select! {
                msg = self.receiver.recv() => {
                    let msg = msg.expect("fabric packet sender dropped!");
                    self.handle_fabric_packet(msg);
                },
                msg = self.peer_receiver.recv() => {
                    let msg = msg.expect("can never happen, last sender always in FabricState");
                    match msg.kind {
                        PeerConnMsgKind::Connected => {
                            self.sender.send(proto::PeerConnectionSuccess {
                                peer_uuid: msg.uuid,
                            });
                        },
                    }
                },
            };
        }
    }

    fn transition(&mut self, to: FabricProtoState) {
        log::debug!("fabric proto state transition: {:?} -> {:?}", self.proto_state, to);
        self.proto_state = to;
    }

    pub fn start_handshake(&mut self) {
        assert_eq!(self.proto_state, FabricProtoState::Handshake1);

        log::info!("fabric: starting handshake...");

        let challenge = {
            let mut data = [0; 32];
            self.rand.fill(&mut data).unwrap();
            data
        };

        self.orch_challenge = Some(challenge);

        self.sender.send(proto::ClientHandshake {
            version: Default::default(),
            node_classes: self.node_classes.clone(),
            peer_connection_capabilities: vec![
                proto::PeerConnectionType::WebsocketClient,
                proto::PeerConnectionType::WebsocketServer,
            ],
            token: self.auth_token.clone(),
            pubkey: self.keypair.public_key().as_ref().to_owned(),
            challenge: proto::Challenge {
                challenge,
            },
        });
        self.transition(FabricProtoState::Handshake2)
    }

    pub fn handle_fabric_packet(&mut self, packet: proto::OrchServerMsg) {
        use proto::OrchServerMsg as OSM;
        match packet {
            OSM::ServerHandshake(msg) => self.handle_server_handshake(msg),
            OSM::ConnectPeer(msg) => self.handle_connect_peer(msg),

            OSM::ObjectManifest(msg) => self.data_manager.handle_object_manifest(msg),

            OSM::TestExit(_msg) => {
                log::info!("received test_exit packet, exitting immediately");
                std::process::exit(0);
            },
            _ => todo!(),
        }
    }

    fn handle_server_handshake(&mut self, msg: proto::ServerHandshake) {
        assert_eq!(self.proto_state, FabricProtoState::Handshake2);

        let algo = &signature::ECDSA_P256_SHA256_FIXED;
        self.orch_pubkey = Some(signature::UnparsedPublicKey::new(algo, msg.pubkey.clone()));

        self.uuid = Some(msg.client_uuid.clone());

        // Validate challenge response from the orchestrator.
        {
            let challenge = self.orch_challenge.take().unwrap();

            assert_eq!(msg.challenge_response.challenge_response.len(), CHALLENGE_RESPONSE_LEN);
            let challenge_range = (HANDSHAKE_CHALLENGE_WRAP.len())..(HANDSHAKE_CHALLENGE_WRAP.len() + 32);
            assert_eq!(&msg.challenge_response.challenge_response[challenge_range], &challenge);

            self.orch_pubkey.as_ref().unwrap().verify(
                &msg.challenge_response.challenge_response,
                &msg.challenge_response.signature,
            ).unwrap();
        }

        // Generate challenge response for the orchestrator.
        let challenge_response = {
            let mut ret_challenge = vec![0; 32];
            self.rand.fill(&mut ret_challenge[..]).unwrap();

            let mut challenge_response = Vec::new();
            challenge_response.extend(HANDSHAKE_CHALLENGE_WRAP.as_bytes());
            assert!(msg.challenge.challenge.len() == 32);
            challenge_response.extend(&msg.challenge.challenge);
            challenge_response.extend(&ret_challenge);
            challenge_response.extend(HANDSHAKE_CHALLENGE_WRAP.as_bytes());

            let signature = self.keypair.sign(
                &*self.rand,
                &challenge_response,
            ).unwrap();

            proto::ChallengeResponse {
                challenge_response,
                signature: signature.as_ref().to_owned(),
            }
        };

        log::info!(
            "fabric: connecting to orchestrator with pubkey {:x?}",
            msg.pubkey,
        );

        self.sender.send(proto::ClientHandshakeFinish {
            challenge_response,
        });

        self.transition(FabricProtoState::Normal);
        log::info!("fabric: connected!");
    }

    fn handle_connect_peer(&mut self, msg: proto::ConnectPeer) {
        let sender = self.peer_receiver_sender.clone();

        let connect_fut = self.peer_connector.start_connect_peer(
            PeerTunnel::new_dummy(),
            msg.connector,
            msg.self_nonce,
            msg.peer_nonce,
        );

        let peer_uuid = msg.peer_uuid;

        let fut = async move {
            let conn = match connect_fut.await {
                Ok(conn) => conn,
                Err(err) => {
                    log::error!("noo {:?}", err);
                    return;
                },
            };

            sender.send(PeerConnMsg {
                uuid: peer_uuid.clone(),
                kind: PeerConnMsgKind::Connected,
            }).await.ok().unwrap();
        };
        tokio::spawn(fut);
    }


}
