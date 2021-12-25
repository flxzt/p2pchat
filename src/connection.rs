use anyhow::Context;
use futures::executor::block_on;
use libp2p::identity::Keypair;
use libp2p::ping::{Ping, PingConfig};
use libp2p::{Multiaddr, PeerId, Swarm};

pub struct Connection {
    pub swarm: Swarm<Ping>,
    pub log: Vec<String>,
}

impl Connection {
    pub fn new() -> Result<Self, anyhow::Error> {
        let swarm = Self::regenerate_swarm()
            .context("regenerate_swarm() failed in Connection::new() with Err {}")?;

        Ok(Self {
            swarm,
            log: vec![],
        })
    }

    pub fn push_log_entry(&mut self, message: &str) {
        self.log.push(format!("{}", message));
    }

    pub fn regenerate_swarm() -> Result<Swarm<Ping>, anyhow::Error> {
        let local_key = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        let transport = block_on(libp2p::development_transport(local_key))?;

        let behaviour = Ping::new(PingConfig::new().with_keep_alive(true));
        let mut swarm = Swarm::new(transport, behaviour, local_peer_id);
        swarm
            .listen_on(
                "/ip4/0.0.0.0/tcp/0"
                    .parse()
                    .context("addr parse() failed in regenerate_swarm()")?,
            )
            .context("swarm.listen_on() failed in regenerate_swarm()")?;

        Ok(swarm)
    }

    pub fn dial(&mut self, addr: Multiaddr) -> Result<(), anyhow::Error> {
        self.push_log_entry(format!("dialing: {}", addr).as_str());

        self.swarm.dial(addr)?;
        Ok(())
    }
}
