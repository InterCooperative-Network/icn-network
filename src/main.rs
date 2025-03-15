use std::env;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::time::Duration;
use log::{info, error};

// Update imports with our fixed modules
use crate::config::NodeConfig;
use crate::identity::Identity;
use crate::storage::Storage;
use crate::crypto::CryptoUtils;
use crate::federation::FederationSystem;
use crate::federation_governance::{FederationGovernance, ProposalType, Deliberation, GovernanceParticipationScore};
use crate::cross_federation_governance::CrossFederationGovernance;
use crate::resource_sharing::ResourceSharingSystem;
use crate::reputation::{ReputationSystem, AttestationType, Evidence, Attestation, TrustScore, SybilIndicators};

// Include modules
mod config;
mod identity;
mod storage;
mod crypto;
mod federation;
mod federation_governance;
mod cross_federation_governance;
mod resource_sharing;
mod reputation;

// Simplified PeerInfo for now
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub addr: SocketAddr,
    pub did: String,
    pub node_id: String,
    pub connected_at: u64,
    pub last_seen: u64,
    pub is_active: bool,
}

// Simplified NetworkManager for now
pub struct NetworkManager {
    listen_addr: SocketAddr,
    tls_enabled: bool,
}

impl NetworkManager {
    pub fn new(listen_addr: SocketAddr, tls_enabled: bool) -> Result<Self, Box<dyn Error>> {
        Ok(NetworkManager {
            listen_addr,
            tls_enabled,
        })
    }
    
    pub fn start(&self) -> Result<(), Box<dyn Error>> {
        info!("Starting network manager on {}", self.listen_addr);
        Ok(())
    }
    
    pub fn connect_to_peer(&self, addr: SocketAddr) -> Result<(), Box<dyn Error>> {
        info!("Connecting to peer: {}", addr);
        Ok(())
    }
}

// Simplified MutualCreditSystem
pub struct MutualCreditSystem {
    identity: Arc<Identity>,
    storage: Arc<Storage>,
    crypto: Arc<CryptoUtils>,
    reputation: Option<Arc<ReputationSystem>>,
}

impl MutualCreditSystem {
    pub fn new(
        identity: Arc<Identity>,
        storage: Arc<Storage>,
        crypto: Arc<CryptoUtils>,
    ) -> Self {
        MutualCreditSystem {
            identity,
            storage,
            crypto,
            reputation: None,
        }
    }
    
    pub fn set_reputation_system(&mut self, reputation: Arc<ReputationSystem>) {
        self.reputation = Some(reputation);
    }
    
    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        info!("Starting mutual credit system");
        Ok(())
    }
    
    pub async fn stop(&self) -> Result<(), Box<dyn Error>> {
        info!("Stopping mutual credit system");
        Ok(())
    }
}

// Main ICN Node structure
pub struct IcnNode {
    config: NodeConfig,
    identity: Arc<Identity>,
    network: NetworkManager,
    storage: Arc<Storage>,
    economic: Arc<MutualCreditSystem>,
    federation: Arc<FederationSystem>,
    governance: Arc<FederationGovernance>,
    cross_federation_governance: Arc<CrossFederationGovernance>,
    resource_sharing: Arc<ResourceSharingSystem>,
    reputation: Arc<ReputationSystem>,
    peers: Arc<Mutex<Vec<PeerInfo>>>,
}

impl IcnNode {
    // Create a new ICN node
    pub fn new(
        coop_id: String,
        node_id: String,
        did: String,
        storage_path: String,
    ) -> Result<Self, Box<dyn Error>> {
        info!("Initializing ICN Node...");
        
        // Initialize components
        let storage = Arc::new(Storage::new(&storage_path));
        let identity = Arc::new(Identity::new(
            coop_id.clone(),
            node_id.clone(),
            did.clone(),
            storage.clone(),
        )?);
        
        let crypto = Arc::new(CryptoUtils::new());
        
        // Create reputation system first since others depend on it
        let reputation = Arc::new(ReputationSystem::new(
            identity.clone(),
            storage.clone(),
            crypto.clone(),
        ));
        
        // Create economic system with reputation
        let mut economic = MutualCreditSystem::new(
            identity.clone(),
            storage.clone(),
            crypto.clone(),
        );
        economic.set_reputation_system(reputation.clone());
        let economic = Arc::new(economic);
        
        // Create federation system
        let federation = Arc::new(FederationSystem::new(
            identity.clone(),
            storage.clone(),
            economic.clone(),
        ));
        
        // Create governance with reputation
        let mut governance = FederationGovernance::new(
            identity.clone(),
            storage.clone(),
        );
        governance.set_reputation_system(reputation.clone());
        let governance = Arc::new(governance);
        
        let cross_federation_governance = Arc::new(CrossFederationGovernance::new(
            identity.clone(),
            storage.clone(),
        ));
        
        let resource_sharing = Arc::new(ResourceSharingSystem::new(
            identity.clone(),
            storage.clone(),
        ));
        
        let peers = Arc::new(Mutex::new(Vec::new()));
        
        let network = NetworkManager::new(
            identity.listen_addr.parse::<SocketAddr>()?, 
            identity.tls
        )?;
        
        let config = NodeConfig {
            node_id: node_id.clone(),
            coop_id: coop_id.clone(),
            node_type: "primary".to_string(),
            listen_addr: identity.listen_addr.clone(),
            peers: Vec::new(),
            discovery_interval: 30,
            health_check_interval: 10,
            data_dir: storage_path,
            cert_dir: "/etc/icn/certs".to_string(),
            log_dir: "/var/log/icn".to_string(),
            log_level: "info".to_string(),
            tls: config::TlsConfig {
                enabled: identity.tls,
                cert_file: "/etc/icn/certs/node.crt".to_string(),
                key_file: "/etc/icn/certs/node.key".to_string(),
                ca_file: "/etc/icn/certs/ca.crt".to_string(),
                verify_client: true,
                verify_hostname: true,
            },
        };
        
        Ok(IcnNode {
            config,
            identity,
            network,
            storage,
            economic,
            federation,
            governance,
            cross_federation_governance,
            resource_sharing,
            reputation,
            peers,
        })
    }
    
    // Start the ICN node
    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        info!("Starting ICN Node: {}", self.identity.node_id);
        info!("Cooperative: {}", self.identity.coop_id);
        info!("DID: {}", self.identity.did);
        
        // Start the network manager
        self.network.start()?;
        
        // Connect to initial peers if provided
        if !self.config.peers.is_empty() {
            for peer_addr in &self.config.peers {
                match peer_addr.parse::<SocketAddr>() {
                    Ok(addr) => {
                        println!("Connecting to peer: {}", addr);
                        let _ = self.network.connect_to_peer(addr);
                    },
                    Err(e) => println!("Invalid peer address: {} - {}", peer_addr, e),
                }
            }
        }
        
        // Start periodic tasks
        self.start_discovery();
        self.start_health_check();
        
        // Initialize systems
        self.economic.start().await?;
        
        info!("Ready to facilitate transactions between cooperative members");
        info!("Ready to handle federation transactions and governance");
        info!("Ready to participate in cross-federation coordination");
        info!("Ready to manage resource sharing between federations");
        
        Ok(())
    }
    
    // Start peer discovery process
    fn start_discovery(&self) {
        let peers_clone = Arc::clone(&self.peers);
        let discovery_interval = self.config.discovery_interval;
        
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(discovery_interval));
                println!("Running peer discovery...");
                println!("Connected peers: {}", peers_clone.lock().unwrap().len());
            }
        });
    }
    
    // Start health check process
    fn start_health_check(&self) {
        let peers_clone = Arc::clone(&self.peers);
        let health_check_interval = self.config.health_check_interval;
        
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(health_check_interval));
                println!("Running health check...");
                let healthy_peers = peers_clone.lock().unwrap().len();
                println!("Healthy peers: {}", healthy_peers);
            }
        });
    }

    pub async fn stop(&self) -> Result<(), Box<dyn Error>> {
        info!("Stopping ICN Node...");
        self.economic.stop().await?;
        info!("ICN Node stopped");
        Ok(())
    }

    // Create an attestation for another cooperative or member
    pub async fn create_attestation(
        &self,
        subject_did: &str, 
        attestation_type: AttestationType,
        score: f64,
        claims: serde_json::Value,
        evidence: Vec<Evidence>,
    ) -> Result<Attestation, Box<dyn Error>> {
        info!("Creating attestation for {}", subject_did);
        
        // Default quorum threshold is 1 (just us)
        let quorum_threshold = 1;
        
        // Default expiration is 365 days
        let expiration_days = Some(365);
        
        self.reputation.attestation_manager().create_attestation(
            subject_did,
            attestation_type,
            score,
            claims,
            evidence,
            quorum_threshold,
            expiration_days,
        )
    }
    
    // Create a multi-party attestation that requires signatures from multiple cooperatives
    pub async fn create_multi_party_attestation(
        &self,
        subject_did: &str,
        attestation_type: AttestationType,
        score: f64,
        claims: serde_json::Value,
        evidence: Vec<Evidence>,
        quorum_threshold: u32,
    ) -> Result<Attestation, Box<dyn Error>> {
        info!("Creating multi-party attestation for {}", subject_did);
        
        // Multi-party attestations last for 180 days by default
        let expiration_days = Some(180);
        
        self.reputation.attestation_manager().create_attestation(
            subject_did,
            attestation_type,
            score,
            claims,
            evidence,
            quorum_threshold,
            expiration_days,
        )
    }
    
    // Sign an existing attestation to support a multi-party attestation
    pub async fn sign_attestation(
        &self,
        attestation_id: &str,
    ) -> Result<Attestation, Box<dyn Error>> {
        info!("Signing attestation {}", attestation_id);
        
        // Create signature data
        let signature_data = format!("sign:{}", attestation_id);
        
        // Sign the data
        let signature = self.identity.sign(signature_data.as_bytes())?;
        
        // Add our signature to the attestation
        self.reputation.attestation_manager().sign_attestation(
            attestation_id,
            &self.identity.did,
            signature.to_bytes().to_vec(),
        )
    }
    
    // Calculate the trust score for an entity based on attestations
    pub async fn calculate_trust_score(
        &self,
        did: &str,
    ) -> Result<TrustScore, Box<dyn Error>> {
        self.reputation.calculate_trust_score(did)
    }
    
    // Check for sybil indicators in a DID's attestation profile
    pub async fn check_sybil_indicators(
        &self,
        did: &str,
    ) -> Result<SybilIndicators, Box<dyn Error>> {
        self.reputation.sybil_resistance().check_sybil_indicators(did)
    }
    
    // Calculate indirect trust between two DIDs through the trust graph
    pub async fn calculate_indirect_trust(
        &self,
        source_did: &str,
        target_did: &str,
    ) -> Result<Option<f64>, Box<dyn Error>> {
        // Default maximum depth is 5 hops, and minimum threshold is 0.5
        let max_depth = 5;
        let min_trust_threshold = 0.5;
        
        self.reputation.trust_graph().calculate_indirect_trust(
            source_did,
            target_did,
            max_depth,
            min_trust_threshold,
        )
    }
    
    // Create a new governance proposal
    pub fn create_proposal(
        &self,
        federation_id: &str,
        proposal_type: ProposalType,
        title: &str,
        description: &str,
        voting_duration_days: u64,
        quorum: u64,
        changes: serde_json::Value,
    ) -> Result<String, Box<dyn Error>> {
        // Convert days to seconds
        let voting_duration = voting_duration_days * 24 * 60 * 60;
        
        let proposal = self.governance.create_proposal(
            federation_id,
            proposal_type,
            title,
            description,
            voting_duration,
            quorum,
            changes,
        )?;
        
        Ok(proposal.id.clone())
    }
    
    // Vote on a proposal
    pub fn vote_on_proposal(
        &self,
        proposal_id: &str,
        vote: bool,
    ) -> Result<(), Box<dyn Error>> {
        // Since vote is async, we need to block on it in this sync context
        // In a real implementation, this would be handled properly with async/await
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(self.governance.vote(proposal_id, vote))
    }
    
    // Add a deliberation to a proposal
    pub fn add_deliberation(
        &self,
        proposal_id: &str,
        comment: &str,
        references: Vec<String>,
    ) -> Result<Deliberation, Box<dyn Error>> {
        // Since add_deliberation is async, we need to block on it in this sync context
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(self.governance.add_deliberation(proposal_id, comment, references))
    }
    
    // Get all deliberations for a proposal
    pub fn get_proposal_deliberations(
        &self,
        proposal_id: &str,
    ) -> Result<Vec<Deliberation>, Box<dyn Error>> {
        self.governance.get_deliberations(proposal_id)
    }
    
    // Calculate governance participation score for a member
    pub fn get_governance_score(
        &self,
        member_did: &str,
    ) -> Result<GovernanceParticipationScore, Box<dyn Error>> {
        // Since calculate_governance_score is async, we need to block on it in this sync context
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(self.governance.calculate_governance_score(member_did))
    }
    
    // Get comprehensive trust score
    pub fn get_comprehensive_trust_score(
        &self,
        did: &str,
    ) -> Result<f64, Box<dyn Error>> {
        // Make sure governance reputation is up to date
        let runtime = tokio::runtime::Runtime::new().unwrap();
        if let Ok(governance_score) = runtime.block_on(self.governance.calculate_governance_score(did)) {
            // Calculate an overall score based on the governance participation
            // Convert usize fields to f64 for calculation
            let proposals_created = governance_score.proposals_created as f64;
            let proposals_voted = governance_score.proposals_voted as f64;
            let deliberations_count = governance_score.deliberations_count as f64;
            
            let score_value = (
                proposals_created * 2.0 + 
                proposals_voted * 1.0 + 
                deliberations_count * 1.5
            ) / 10.0;
            
            // Ensure score is in range 0.0-1.0
            let normalized_score = score_value.min(1.0).max(0.0);
            
            // Create attestation through the attestation manager
            let trust_score = self.reputation.calculate_trust_score(did)?;
            return Ok(trust_score.overall_score);
        }
        
        // If we couldn't get a governance score, just return the current trust score
        let trust_score = self.reputation.calculate_trust_score(did)?;
        Ok(trust_score.overall_score)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    env_logger::init();
    
    // Example usage
    let node = IcnNode::new(
        "coop123".to_string(),
        "node1".to_string(),
        "did:icn:coop123:node1".to_string(),
        "data".to_string()
    )?;
    
    node.start().await?;
    
    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    
    node.stop().await?;
    
    Ok(())
} 