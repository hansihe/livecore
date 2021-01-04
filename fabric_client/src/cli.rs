use std::pin::Pin;

use clap::Clap;
use futures::{Stream, StreamExt, Sink, SinkExt};

use tokio::net::{TcpListener, UnixListener};

use livecore_protocol as proto;

#[derive(Clap, Debug, PartialEq)]
enum FabricProtocol {
    Websocket,
    IPC,
}

#[derive(Clap)]
#[clap(version = "0.1", author = "Hans Elias B. Josephsen")]
struct Opts {
    /// The URL of the fabric to connect to.
    fabric_url: String,

    /// Auth token used when connecting to the fabric.
    /// If the fabric requires authorization, then this token may be required.
    /// Otherwise, this is usually provided to grant the node a set of additional
    /// privileges, like sourcing media.
    #[clap(short, long)]
    token: Option<String>,

    /// Which classes this node should join the fabric with.
    /// Depending on the orchestrator implementation, this will decide what
    /// roles the node will have in the fabric.
    #[clap(long = "class")]
    classes: Vec<String>,

    /// Specify a path to bind a unix socket where other peers can connect to.
    #[clap(long)]
    ipc_peer_bind: Option<String>,

    /// Specify an address to listen for incoming peer WebSocket connections.
    #[clap(long)]
    ws_peer_bind: Option<String>,

    #[clap(long = "fabric-protocol", default_value = "websocket", arg_enum)]
    fabric_protocol: FabricProtocol,
}

type PacketSink = Pin<Box<dyn Sink<Vec<u8>, Error = ()> + Send>>;
type PacketSource = Pin<Box<dyn Stream<Item = Result<Vec<u8>, ()>> + Send>>;

async fn connect_ws(url: String) -> (PacketSource, PacketSink) {
    log::info!("connecting to WS fabric {}", url);
    let ws = fabric_client::platform::connect(url).await.unwrap();

    let source = ws.source.map(|v| match v {
        Ok(msg) => Ok(msg.bytes().to_owned()),
        Err(_err) => Err(()),
    });

    let sink = ws.sink
        .sink_map_err(|_err| ())
        .with(|v: Vec<u8>| async { Ok(v.into()) });

    (Box::pin(source), Box::pin(sink))
}

async fn connect_ipc(path: String) -> (PacketSource, PacketSink) {
    use tokio::net::UnixStream;
    use tokio_util::codec::{Framed, LengthDelimitedCodec};

    log::info!("connecting to IPC fabric {}", path);

    let stream = UnixStream::connect(path).await.unwrap();
    let framed = Framed::new(stream, LengthDelimitedCodec::new());

    let (sink, source) = framed.split();

    let source = source.map(|v| match v {
        Ok(bytes) => Ok(bytes.as_ref().to_owned()),
        Err(_) => Err(()),
    });

    let sink = sink
        .sink_map_err(|_err| ())
        .with(|v: Vec<u8>| async { Ok(v.into()) });

    (Box::pin(source), Box::pin(sink))
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let opts: Opts = Opts::parse();

    let (mut transport_receiver, mut transport_sender) = match opts.fabric_protocol {
        FabricProtocol::Websocket => connect_ws(opts.fabric_url.clone()).await,
        FabricProtocol::IPC => connect_ipc(opts.fabric_url.clone()).await,
    };

    use fabric_client::platform::peer_connection_manager_impl::NativePeerConnectionManagerBuilder;
    let mut peer_connector_builder = NativePeerConnectionManagerBuilder::new();

    if let Some(addr) = opts.ws_peer_bind {
        log::info!("binding to peer WS addr {}", addr);
        let listener = TcpListener::bind(addr).await.unwrap();
        peer_connector_builder = peer_connector_builder.with_ws_listener(listener);
    }

    if let Some(addr) = opts.ipc_peer_bind {
        log::info!("binding to peer IPC addr {}", addr);
        let listener = UnixListener::bind(addr).unwrap();
        peer_connector_builder = peer_connector_builder.with_ipc_listener(listener);
    }

    let peer_connector = peer_connector_builder.build();

    //let mut ws = fabric_client::platform::connect(opts.fabric_url.clone()).await.unwrap();
    let (sender, mut receiver) = fabric_client::OrchPacketSender::new();

    tokio::spawn(async move {
        while let Some(msg) = receiver.recv().await {
            transport_sender.send(msg).await.unwrap();
        }
    });

    let mut fabric_builder = fabric_client::FabricBuilder::new()
        .with_node_classes(opts.classes);

    if let Some(token) = opts.token {
        fabric_builder = fabric_builder.with_auth_token(token);
    }

    let fabric = fabric_builder.start(
        sender,
        Box::new(peer_connector),
    );

    while let Some(resp) = transport_receiver.next().await {
        match resp {
            Ok(val) => {
                let msg = proto::OrchServerMsg::deserialize(&val).unwrap();
                fabric.handle_fabric_packet(msg).await;
                //state.handle_fabric_packet(msg);
            },
            Err(()) => {
                println!("disconnect");
            }
        }
    }

    //let keypair = {
    //    use ring::signature;
    //    let algo = &signature::ECDSA_P256_SHA256_FIXED_SIGNING;

    //    let pkcs8_bytes = signature::EcdsaKeyPair::generate_pkcs8(
    //        algo, &rng).unwrap();
    //    signature::EcdsaKeyPair::from_pkcs8(
    //        algo, pkcs8_bytes.as_ref()).unwrap()
    //};


    //let mut challenge = proto::Challenge {
    //    challenge: [0; 32],
    //};
    //rng.fill(&mut challenge.challenge[..]).unwrap();

    //let client_handshake = proto::ClientHandshake {
    //    version: Default::default(),
    //    token: opts.token.clone(),
    //    pubkey: keypair.public_key().as_ref().to_owned(),
    //    challenge: challenge.clone(),
    //};
    //let msg: proto::OrchClientMsg = client_handshake.into();
    //let serialized = msg.serialize().unwrap();
    //ws.sink.send(WebsocketMessage::Text(serialized)).await.unwrap();

    //let resp = ws.source.next().await.unwrap().unwrap();
    //let text = resp.text().unwrap();
    //let msg: proto::ServerHandshake = proto::OrchServerMsg::deserialize(text)
    //    .unwrap().try_into().unwrap();

    //let server_pubkey = {
    //    use ring::signature;
    //    let algo = &signature::ECDSA_P256_SHA256_FIXED;
    //    signature::UnparsedPublicKey::new(algo, msg.pubkey.clone())
    //};

    //const HANDSHAKE_CHALLENGE_WRAP: &str = "__HANDSHAKE_CHALLENGE__";
    //const CHALLENGE_RESPONSE_LEN: usize = (HANDSHAKE_CHALLENGE_WRAP.len() * 2) + (32 * 2);

    //assert_eq!(msg.challenge_response.challenge_response.len(), CHALLENGE_RESPONSE_LEN);
    //let challenge_range = (HANDSHAKE_CHALLENGE_WRAP.len())..(HANDSHAKE_CHALLENGE_WRAP.len() + 32);
    //assert_eq!(&msg.challenge_response.challenge_response[challenge_range], &challenge.challenge[..]);
   
    //server_pubkey.verify(
    //    &msg.challenge_response.challenge_response,
    //    &msg.challenge_response.signature,
    //).unwrap();

    //let mut ret_challenge = vec![0; 32];
    //rng.fill(&mut ret_challenge[..]).unwrap();

    //let mut challenge_response = Vec::new();
    //challenge_response.extend(HANDSHAKE_CHALLENGE_WRAP.as_bytes());
    //assert!(msg.challenge.challenge.len() == 32);
    //challenge_response.extend(&msg.challenge.challenge);
    //challenge_response.extend(&ret_challenge);
    //challenge_response.extend(HANDSHAKE_CHALLENGE_WRAP.as_bytes());

    //let signature = keypair.sign(&rng, &challenge_response).unwrap();

    //let handshake_finish = proto::ClientHandshakeFinish {
    //    challenge_response: proto::ChallengeResponse {
    //        challenge_response,
    //        signature: signature.as_ref().to_owned(),
    //    },
    //};
    //let msg: proto::OrchClientMsg = client_handshake.into();
    //let serialized = msg.serialize().unwrap();
    //ws.sink.send(WebsocketMessage::Text(serialized)).await.unwrap();

}
