use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

use anyhow::Context;
use libp2p::core::upgrade;
use libp2p::gossipsub::error::GossipsubHandlerError;
use libp2p::gossipsub::{
    Gossipsub, GossipsubEvent, GossipsubMessage, IdentTopic, MessageAuthenticity, MessageId,
    ValidationMode,
};
use libp2p::identity::Keypair;
use libp2p::swarm::{SwarmBuilder, SwarmEvent};
use libp2p::{gossipsub, mplex, noise, tcp, Multiaddr, PeerId, Swarm, Transport};

use crate::app::{App, Message};

pub enum Transmission {
    Message { message: Message },
}

pub struct Connection {
    pub swarm: Swarm<Gossipsub>,
    pub log: Vec<String>,
    pub current_topic: IdentTopic,
}

impl Connection {
    pub async fn new() -> Result<Self, anyhow::Error> {
        // Create a Gossipsub topic
        let current_topic = IdentTopic::new("test-net");

        let connection = Self {
            swarm: Self::generate_swarm(&current_topic)?,
            log: vec![],
            current_topic,
        };

        Ok(connection)
    }

    pub fn push_log_entry(&mut self, message: &str) {
        self.log.push(format!("{}", message));
    }

    pub fn generate_swarm(topic: &IdentTopic) -> Result<Swarm<Gossipsub>, anyhow::Error> {
        let id_keys = Keypair::generate_ed25519();
        let peer_id = PeerId::from(id_keys.public());

        // Create a keypair for authenticated encryption of the transport.
        let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
            .into_authentic(&id_keys)
            .context("Signing libp2p-noise static DH keypair failed.")?;

        // Create a tokio-based TCP transport use noise for authenticated
        // encryption and Mplex for multiplexing of substreams on a TCP stream.
        let transport = tcp::TokioTcpConfig::new()
            .nodelay(true)
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
            .multiplex(mplex::MplexConfig::new())
            .boxed();

        // Create a Swarm to manage peers and events
        let mut swarm = {
            // To content-address message, we can take the hash of message and use it as an ID.
            let message_id_fn = |message: &GossipsubMessage| {
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                MessageId::from(s.finish().to_string())
            };

            // Set a custom gossipsub
            let gossipsub_config = gossipsub::GossipsubConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
                .validation_mode(ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
                .message_id_fn(message_id_fn) // content-address messages. No two messages of the
                // same content will be propagated.
                .build()
                .expect("Valid config");
            // build a gossipsub network behaviour
            let mut gossipsub: gossipsub::Gossipsub =
                gossipsub::Gossipsub::new(MessageAuthenticity::Signed(id_keys), gossipsub_config)
                    .expect("Correct configuration");

            // subscribes to our topic
            gossipsub.subscribe(topic).unwrap();

            // build the swarm
            SwarmBuilder::new(transport, gossipsub, peer_id)
                .executor(Box::new(|fut| {
                    tokio::spawn(fut);
                }))
                .build()
        };
        // Listen on all interfaces and whatever port the OS assigns
        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

        Ok(swarm)
    }

    pub fn dial(&mut self, addr: Multiaddr) -> Result<(), anyhow::Error> {
        self.push_log_entry(format!("dialing: {}", addr).as_str());

        self.swarm.dial(addr)?;
        Ok(())
    }
}

pub fn handle_connection_event(
    connection_event: SwarmEvent<GossipsubEvent, GossipsubHandlerError>,
    app: &mut App,
) -> Result<(), anyhow::Error> {
    match connection_event {
        SwarmEvent::NewListenAddr { address, .. } => {
            app.connection
                .push_log_entry(format!("Listening on {:?}", address).as_str());
        }
        SwarmEvent::Behaviour(GossipsubEvent::Message {
            propagation_source: peer_id,
            message_id: id,
            message,
        }) => {
            app.connection.log.push(format!(
                "Got message: {} with id: {} from peer: {:?}",
                String::from_utf8_lossy(&message.data),
                id,
                peer_id
            ));
            app.history.push(Message::new(
                String::from_utf8_lossy(&message.data).to_string(),
                message.source,
            ))
        }
        SwarmEvent::Behaviour(event) => {
            app.connection
                .push_log_entry(format!("{:?}", event).as_str());
        }
        _ => {}
    }

    Ok(())
}
