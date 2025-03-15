use std::env;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, error};
use tokio::signal;
use log::{info, error};

// Modules for our ICN node
mod config;
mod identity;
mod networking;
mod storage;
mod crypto;
mod economic;
mod federation;
mod federation_governance;
mod cross_federation_governance;
mod resource_sharing;
mod reputation;

use config::NodeConfig;
use identity::Identity;
use networking::{NetworkManager, PeerInfo};
use storage::Storage;
use crypto::CryptoUtils;
use economic::MutualCreditSystem;
use federation::FederationSystem;
use federation_governance::FederationGovernance;
use cross_federation_governance::CrossFederationGovernance;
use resource_sharing::ResourceSharingSystem;
use reputation::ReputationSystem;

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
    pub async fn new(
        coop_id: String,
        node_id: String,
        did: String,
        storage_path: std::path::PathBuf,
    ) -> Result<Self, Box<dyn Error>> {
        info!("Initializing ICN Node...");
        
        // Initialize components
        let storage = Arc::new(Storage::new(storage_path)?);
        let identity = Arc::new(Identity::new(
            coop_id.clone(),
            node_id.clone(),
            did.clone(),
            storage.clone(),
        )?);
        let network = NetworkManager::new(identity.listen_addr.parse::<SocketAddr>()?, identity.tls.clone())?;
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
        
        Ok(IcnNode {
            config: NodeConfig::new(coop_id, node_id, did, identity.listen_addr.clone(), identity.tls.clone()),
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
        attestation_type: reputation::AttestationType,
        score: f64,
        claims: serde_json::Value,
        evidence: Vec<reputation::Evidence>,
    ) -> Result<reputation::Attestation, Box<dyn Error>> {
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
        attestation_type: reputation::AttestationType,
        score: f64,
        claims: serde_json::Value,
        evidence: Vec<reputation::Evidence>,
        quorum_threshold: u32,
    ) -> Result<reputation::Attestation, Box<dyn Error>> {
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
    
    // Sign an existing attestation (for multi-party attestations)
    pub async fn sign_attestation(
        &self,
        attestation_id: &str,
    ) -> Result<reputation::Attestation, Box<dyn Error>> {
        info!("Signing attestation {}", attestation_id);
        
        // Generate signature using our identity
        let signature_data = format!("sign:{}", attestation_id);
        let signature = self.economic.crypto.sign(signature_data.as_bytes())?;
        
        // Add our signature to the attestation
        self.reputation.attestation_manager().sign_attestation(
            attestation_id,
            &self.identity.did,
            signature.to_bytes().to_vec(),
        )
    }
    
    // Calculate trust score for a DID
    pub async fn calculate_trust_score(
        &self,
        did: &str,
    ) -> Result<reputation::TrustScore, Box<dyn Error>> {
        info!("Calculating trust score for {}", did);
        self.reputation.calculate_trust_score(did)
    }
    
    // Check for potential Sybil attack patterns
    pub async fn check_sybil_indicators(
        &self,
        did: &str,
    ) -> Result<reputation::SybilIndicators, Box<dyn Error>> {
        info!("Checking Sybil indicators for {}", did);
        self.reputation.sybil_resistance().check_sybil_indicators(did)
    }
    
    // Calculate indirect trust between DIDs that don't have direct attestations
    pub async fn calculate_indirect_trust(
        &self,
        source_did: &str,
        target_did: &str,
    ) -> Result<Option<f64>, Box<dyn Error>> {
        info!("Calculating indirect trust from {} to {}", source_did, target_did);
        
        // Default parameters: max depth of 3, minimum trust threshold of 0.5
        self.reputation.trust_graph().calculate_indirect_trust(
            source_did, 
            target_did,
            3,  // Max depth
            0.5, // Minimum trust threshold
        )
    }

    // Create a new governance proposal
    pub async fn create_proposal(
        &self,
        federation_id: &str,
        proposal_type: federation_governance::ProposalType,
        title: &str,
        description: &str,
        voting_duration_days: u64,
        quorum: u64,
        changes: serde_json::Value,
    ) -> Result<federation_governance::Proposal, Box<dyn Error>> {
        info!("Creating proposal: {}", title);
        
        // Convert days to seconds for voting duration
        let voting_duration = voting_duration_days * 24 * 60 * 60;
        
        self.governance.create_proposal(
            federation_id,
            proposal_type,
            title,
            description,
            voting_duration,
            quorum,
            changes,
        )
    }
    
    // Vote on a governance proposal
    pub async fn vote_on_proposal(
        &self,
        proposal_id: &str,
        vote: bool,
    ) -> Result<(), Box<dyn Error>> {
        info!("Voting on proposal: {}", proposal_id);
        self.governance.vote(proposal_id, vote).await
    }
    
    // Add a deliberation comment to a proposal
    pub async fn add_deliberation(
        &self,
        proposal_id: &str,
        comment: &str,
        references: Vec<String>,
    ) -> Result<federation_governance::Deliberation, Box<dyn Error>> {
        info!("Adding deliberation to proposal: {}", proposal_id);
        self.governance.add_deliberation(proposal_id, comment, references).await
    }
    
    // Get all deliberations for a proposal
    pub async fn get_proposal_deliberations(
        &self,
        proposal_id: &str,
    ) -> Result<Vec<federation_governance::Deliberation>, Box<dyn Error>> {
        info!("Getting deliberations for proposal: {}", proposal_id);
        self.governance.get_deliberations(proposal_id)
    }
    
    // Calculate governance participation score for a member
    pub async fn get_governance_score(
        &self,
        member_did: &str,
    ) -> Result<federation_governance::GovernanceParticipationScore, Box<dyn Error>> {
        info!("Calculating governance score for: {}", member_did);
        self.governance.calculate_governance_score(member_did).await
    }
    
    // Get comprehensive trust score for a DID, including governance component
    pub async fn get_comprehensive_trust_score(
        &self,
        did: &str,
    ) -> Result<reputation::TrustScore, Box<dyn Error>> {
        info!("Calculating comprehensive trust score for: {}", did);
        
        // First, ensure we have up-to-date governance reputation
        let _ = self.governance.calculate_governance_score(did).await;
        
        // Then get the overall trust score which will include governance attestations
        self.reputation.calculate_trust_score(did)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    env_logger::init();

    // Create storage directory
    let storage_path = std::path::PathBuf::from("data");
    std::fs::create_dir_all(&storage_path)?;

    // Create node
    let node = IcnNode::new(
        "test-coop".to_string(),
        "test-node".to_string(),
        "test-did:test:test-coop:test-node".to_string(),
        storage_path,
    ).await?;

    // Start node
    node.start().await?;

    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Received shutdown signal");
            node.stop().await?;
        }
        Err(err) => {
            error!("Error waiting for shutdown signal: {}", err);
        }
    }

    Ok(())
} 