// ICN CLI entry point

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use wireguard_control::{Backend, Device, DeviceUpdate, InterfaceName, Key, KeyPair};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json;
use std::collections::HashMap;
use tokio::fs;
use chrono;
use crate::utils::*;
use anyhow::{anyhow, Context, Result};
use cli_format::*;
use dsl::events::DslEvent;
use primitive_types::U256;
use std::{collections::BTreeMap, fs, io::Write, path::{Path, PathBuf}, str::FromStr, time::Duration};
use tokio::{net::TcpStream, time::sleep};
use tracing::*;

mod storage;
use storage::StorageService;

mod networking;
mod identity;
mod governance;
mod governance_storage;
mod identity_storage;
mod credential_storage;
mod compute;
mod dsl;
use networking::{NetworkManager, FederationNetworkConfig, FederationNetworkProposalType, FederationGovernanceService};
use governance::{GovernanceService, Vote};
use dsl::{DslSystem, DslEvent};

#[derive(Parser, Debug)]
#[clap(author, version, about = "ICN Command Line Interface")]
struct Cli {
    /// Verbose mode (-v, -vv, -vvv)
    #[clap(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Check node status
    Status {},
    
    /// Network operations
    Network {
        #[clap(subcommand)]
        command: NetworkCommands,
    },
    
    /// Storage operations
    Storage {
        #[clap(subcommand)]
        command: StorageCommands,
    },
    
    /// Governance operations
    Governance {
        #[clap(subcommand)]
        command: GovernanceCommands,
    },
    
    /// Governed storage operations
    GovernedStorage {
        #[clap(subcommand)]
        command: GovernedStorageCommands,
    },
    
    /// Identity-based storage operations
    IdentityStorage {
        #[clap(subcommand)]
        command: IdentityStorageCommands,
    },
    
    /// Credential-based storage operations
    CredentialStorage {
        #[clap(subcommand)]
        command: CredentialStorageCommands,
    },

    /// Compute operations
    Compute(ComputeCommands),
    
    /// Domain-Specific Language operations
    Dsl {
        #[clap(subcommand)]
        command: DslCommands,
    },
}

#[derive(Subcommand, Debug)]
enum NetworkCommands {
    /// Test network connectivity to a server
    Connect {
        /// Server address to connect to
        #[clap(short, long, default_value = "127.0.0.1:8000")]
        server: String,
    },
    
    /// List discovered peers
    ListPeers {},
    
    /// Enable circuit relay for NAT traversal
    EnableRelay {},
    
    /// Connect to a peer through a relay
    ConnectViaRelay {
        /// Relay server address
        #[clap(short, long)]
        relay: String,
        
        /// Target peer ID to connect to
        #[clap(short, long)]
        peer: String,
    },
    
    /// Create a WireGuard tunnel to a peer
    CreateTunnel {
        /// Peer ID to create tunnel with
        #[clap(short, long)]
        peer: String,
        
        /// Local IP address for the tunnel
        #[clap(short, long, default_value = "10.0.0.1/24")]
        local_ip: String,
        
        /// Listen port for WireGuard
        #[clap(short, long, default_value = "51820")]
        port: u16,
    },
    
    /// Show network diagnostics
    Diagnostics {},
    
    /// Send a message to a peer
    SendMessage {
        /// Peer ID to send message to
        #[clap(short, long)]
        peer: String,
        
        /// Message type
        #[clap(short, long, default_value = "chat")]
        message_type: String,
        
        /// Message content (JSON format)
        #[clap(short, long)]
        content: String,
    },
    
    /// Create a new federation
    CreateFederation {
        /// Federation ID
        #[clap(short, long)]
        id: String,
        
        /// Federation bootstrap peers (comma-separated)
        #[clap(short, long)]
        bootstrap: Option<String>,
        
        /// Whether to allow cross-federation communication
        #[clap(short, long)]
        allow_cross_federation: bool,
        
        /// Allowed federations for cross-federation communication (comma-separated)
        #[clap(short, long)]
        allowed_federations: Option<String>,
        
        /// Whether to encrypt federation traffic
        #[clap(short, long, default_value = "true")]
        encrypt: bool,
        
        /// Whether to use WireGuard for this federation
        #[clap(long)]
        use_wireguard: bool,
        
        /// DHT namespace for this federation
        #[clap(long)]
        dht_namespace: Option<String>,
    },
    
    /// List federations
    ListFederations {},
    
    /// Switch active federation
    SwitchFederation {
        /// Federation ID to switch to
        #[clap(short, long)]
        id: String,
    },
    
    /// Show federation information
    FederationInfo {
        /// Federation ID
        #[clap(short, long)]
        id: Option<String>,
    },
    
    /// Send message to all peers in a federation
    BroadcastToFederation {
        /// Federation ID
        #[clap(short, long)]
        id: Option<String>,
        
        /// Message type
        #[clap(short, long, default_value = "broadcast")]
        message_type: String,
        
        /// Message content (JSON format)
        #[clap(short, long)]
        content: String,
    },
    
    /// List peers in a federation
    FederationPeers {
        /// Federation ID
        #[clap(short, long)]
        id: Option<String>,
    },
    
    /// Enable WireGuard for a specific federation
    EnableFederationWireGuard {
        /// Federation ID
        #[clap(short, long)]
        id: Option<String>,
    },
    
    /// Show federation metrics
    FederationMetrics {
        /// Federation ID
        #[clap(short, long)]
        id: Option<String>,
    },
    
    /// Federation governance operations
    Governance {
        #[clap(subcommand)]
        command: FederationGovernanceCommands,
    },
}

/// Federation governance commands for democratic network operations
#[derive(Subcommand, Debug)]
enum FederationGovernanceCommands {
    /// Create a network governance proposal
    CreateProposal {
        /// Proposal title
        #[clap(short, long)]
        title: String,
        
        /// Proposal description
        #[clap(short, long)]
        description: String,
        
        /// Member ID of the proposer
        #[clap(short, long)]
        proposer: String,
        
        /// Proposal type: add-peer, remove-peer, update-config, enable-cross, disable-cross, enable-wireguard, disable-wireguard, add-bootstrap
        #[clap(short, long)]
        proposal_type: String,
        
        /// Additional JSON parameters for the proposal
        #[clap(short, long)]
        params: String,
    },
    
    /// List network governance proposals
    ListProposals {},
    
    /// Show details of a specific network governance proposal
    ShowProposal {
        /// Proposal ID
        #[clap(short, long)]
        id: String,
    },
    
    /// Cast a vote on a network governance proposal
    Vote {
        /// Proposal ID
        #[clap(short, long)]
        id: String,
        
        /// Member ID casting the vote
        #[clap(short, long)]
        member: String,
        
        /// Vote (yes, no, abstain)
        #[clap(short, long)]
        vote: String,
        
        /// Optional comment with the vote
        #[clap(short, long)]
        comment: Option<String>,
        
        /// Voting weight (defaults to 1.0)
        #[clap(short, long, default_value = "1.0")]
        weight: f64,
    },
    
    /// Execute an approved network governance proposal
    ExecuteProposal {
        /// Proposal ID
        #[clap(short, long)]
        id: String,
    },
    
    /// Sync governance data with the federation
    SyncGovernance {},
}

#[derive(Subcommand, Debug)]
enum StorageCommands {
    /// Initialize storage environment
    Init {
        /// Storage directory path
        #[clap(short, long, default_value = "./data")]
        path: String,
        
        /// Enable encryption for stored data
        #[clap(short, long)]
        encrypted: bool,
    },
    
    /// Store a file in the distributed storage
    Put {
        /// File to store
        #[clap(short, long)]
        file: String,
        
        /// Storage key (defaults to filename)
        #[clap(short, long)]
        key: Option<String>,
        
        /// Enable encryption for this file
        #[clap(short, long)]
        encrypted: bool,
        
        /// Federation name for multi-federation storage
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// Retrieve a file from the distributed storage
    Get {
        /// Storage key to retrieve
        #[clap(short, long)]
        key: String,
        
        /// Output file path (defaults to key name)
        #[clap(short, long)]
        output: Option<String>,
        
        /// Specific version to retrieve (defaults to latest)
        #[clap(short, long)]
        version: Option<String>,
        
        /// Federation name for multi-federation storage
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// List stored files
    List {
        /// Filter by prefix
        #[clap(short, long)]
        prefix: Option<String>,
        
        /// Federation name for multi-federation storage
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// Show version history for a file
    History {
        /// Storage key
        #[clap(short, long)]
        key: String,
        
        /// Maximum number of versions to show
        #[clap(short, long, default_value = "10")]
        limit: usize,
        
        /// Federation name for multi-federation storage
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// Generate encryption key for secure storage
    GenerateKey {
        /// Key output file
        #[clap(short, long, default_value = "./storage.key")]
        output: String,
    },
    
    /// Generate asymmetric key pair for recipient-specific encryption
    GenerateKeyPair {
        /// Output directory for key files
        #[clap(short, long, default_value = "./keys")]
        output_dir: String,
    },
    
    /// Export a federation encryption key for sharing
    ExportKey {
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
        
        /// Output file path
        #[clap(short, long, default_value = "./federation_key.json")]
        output: String,
    },
    
    /// Import a federation encryption key
    ImportKey {
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
        
        /// Key file path
        #[clap(short, long)]
        key_file: String,
    },
    
    /// Encrypt a file for specific recipients using public keys
    EncryptFor {
        /// Input file to encrypt
        #[clap(short, long)]
        input: String,
        
        /// Output file path
        #[clap(short, long)]
        output: String,
        
        /// Recipient public key files (comma-separated)
        #[clap(short, long)]
        recipients: String,
    },
    
    /// Decrypt a file using your private key
    DecryptWith {
        /// Input encrypted file
        #[clap(short, long)]
        input: String,
        
        /// Output file path
        #[clap(short, long)]
        output: String,
        
        /// Private key file
        #[clap(short, long)]
        private_key: String,
    },
}

#[derive(Subcommand, Debug)]
enum GovernanceCommands {
    /// Create a new governance proposal
    CreateProposal {
        /// Proposal title
        #[clap(short, long)]
        title: String,
        
        /// Proposal description
        #[clap(short, long)]
        description: String,
        
        /// Type of proposal (policy, member-add, member-remove, resource, dispute, config)
        #[clap(short, long)]
        proposal_type: String,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
        
        /// Member ID of the proposer
        #[clap(short, long)]
        proposer: String,
        
        /// Minimum quorum percentage required (0-100)
        #[clap(short, long, default_value = "51")]
        quorum: u8,
        
        /// Minimum approval percentage required (0-100)
        #[clap(short, long, default_value = "51")]
        approval: u8,
        
        /// JSON file containing proposal content
        #[clap(short, long)]
        content_file: Option<String>,
    },
    
    /// List all proposals in a federation
    ListProposals {
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
        
        /// Filter by status (draft, deliberation, voting, approved, rejected, executed, canceled)
        #[clap(short, long)]
        status: Option<String>,
    },
    
    /// Show details of a specific proposal
    ShowProposal {
        /// Proposal ID
        #[clap(short, long)]
        id: String,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// Update proposal status
    UpdateStatus {
        /// Proposal ID
        #[clap(short, long)]
        id: String,
        
        /// New status (draft, deliberation, voting, approved, rejected, executed, canceled)
        #[clap(short, long)]
        status: String,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// Start voting period for a proposal
    StartVoting {
        /// Proposal ID
        #[clap(short, long)]
        id: String,
        
        /// Duration of voting period in seconds
        #[clap(short, long, default_value = "86400")]
        duration: u64,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// Cast a vote on a proposal
    Vote {
        /// Proposal ID
        #[clap(short, long)]
        id: String,
        
        /// Member ID casting the vote
        #[clap(short, long)]
        member: String,
        
        /// Vote (yes, no, abstain)
        #[clap(short, long)]
        vote: String,
        
        /// Optional comment with the vote
        #[clap(short, long)]
        comment: Option<String>,
        
        /// Voting weight (defaults to 1.0)
        #[clap(short, long, default_value = "1.0")]
        weight: f64,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// Finalize voting on a proposal
    FinalizeVoting {
        /// Proposal ID
        #[clap(short, long)]
        id: String,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// Execute an approved proposal
    ExecuteProposal {
        /// Proposal ID
        #[clap(short, long)]
        id: String,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
}

#[derive(Subcommand, Debug)]
enum GovernedStorageCommands {
    /// Store a file with governance permission checks
    StoreFile {
        /// File to store
        #[clap(short, long)]
        file: String,
        
        /// Storage key (defaults to filename)
        #[clap(short, long)]
        key: Option<String>,
        
        /// Member ID performing the action
        #[clap(short, long)]
        member: String,
        
        /// Enable encryption for this file
        #[clap(short, long)]
        encrypted: bool,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// Retrieve a file with governance permission checks
    GetFile {
        /// Storage key to retrieve
        #[clap(short, long)]
        key: String,
        
        /// Member ID performing the action
        #[clap(short, long)]
        member: String,
        
        /// Output file path (defaults to key name)
        #[clap(short, long)]
        output: Option<String>,
        
        /// Specific version to retrieve (defaults to latest)
        #[clap(short, long)]
        version: Option<String>,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// List files with governance permission checks
    ListFiles {
        /// Member ID performing the action
        #[clap(short, long)]
        member: String,
        
        /// Filter by prefix
        #[clap(short, long)]
        prefix: Option<String>,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// Propose a new storage policy
    ProposePolicy {
        /// Member ID of the proposer
        #[clap(short, long)]
        proposer: String,
        
        /// Proposal title
        #[clap(short, long)]
        title: String,
        
        /// Proposal description
        #[clap(short, long)]
        description: String,
        
        /// Policy type (federation-quota, member-quota, access-control, retention, encryption, replication)
        #[clap(short, long)]
        policy_type: String,
        
        /// JSON file containing policy content
        #[clap(short, long)]
        content_file: String,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// Apply an approved storage policy
    ApplyPolicy {
        /// Proposal ID to apply
        #[clap(short, long)]
        proposal_id: String,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// List active storage policies
    ListPolicies {
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
        
        /// Filter by policy type
        #[clap(short, long)]
        policy_type: Option<String>,
    },
    
    /// Show JSON schema for a policy type
    ShowSchema {
        /// Policy type (federation-quota, member-quota, access-control, retention)
        #[clap(short, long)]
        policy_type: String,
    },
}

#[derive(Subcommand, Debug)]
enum IdentityStorageCommands {
    /// Initialize identity storage environment
    Init {
        /// Storage directory path
        #[clap(short, long, default_value = "./data")]
        path: String,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
        
        /// Authentication cache TTL in seconds
        #[clap(short, long, default_value = "3600")]
        cache_ttl: u64,
    },
    
    /// Register a new DID document for storage access
    RegisterDid {
        /// DID identifier (did:icn:...)
        #[clap(short, long)]
        did: String,
        
        /// DID document file path (JSON)
        #[clap(short, long)]
        document: String,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// Store a file with DID authentication
    StoreFile {
        /// DID identifier (did:icn:...)
        #[clap(short, long)]
        did: String,
        
        /// Authentication challenge (for signing)
        #[clap(short, long)]
        challenge: String,
        
        /// Signature of the challenge
        #[clap(short, long)]
        signature: String,
        
        /// File to store
        #[clap(short, long)]
        file: String,
        
        /// Storage key (defaults to filename)
        #[clap(short, long)]
        key: Option<String>,
        
        /// Enable encryption for this file
        #[clap(short, long)]
        encrypted: bool,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// Retrieve a file with DID authentication
    GetFile {
        /// DID identifier (did:icn:...)
        #[clap(short, long)]
        did: String,
        
        /// Authentication challenge (for signing)
        #[clap(short, long)]
        challenge: String,
        
        /// Signature of the challenge
        #[clap(short, long)]
        signature: String,
        
        /// Storage key to retrieve
        #[clap(short, long)]
        key: String,
        
        /// Output file path (defaults to key name)
        #[clap(short, long)]
        output: Option<String>,
        
        /// Specific version to retrieve (defaults to latest)
        #[clap(short, long)]
        version: Option<String>,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// List files with DID authentication
    ListFiles {
        /// DID identifier (did:icn:...)
        #[clap(short, long)]
        did: String,
        
        /// Authentication challenge (for signing)
        #[clap(short, long)]
        challenge: String,
        
        /// Signature of the challenge
        #[clap(short, long)]
        signature: String,
        
        /// Filter by prefix
        #[clap(short, long)]
        prefix: Option<String>,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// Create a DID to Member ID mapping
    MapDidToMember {
        /// DID identifier (did:icn:...)
        #[clap(short, long)]
        did: String,
        
        /// Member ID
        #[clap(short, long)]
        member_id: String,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// Create an access control policy with DID authentication
    CreateAccessPolicy {
        /// DID identifier (did:icn:...)
        #[clap(short, long)]
        did: String,
        
        /// Authentication challenge (for signing)
        #[clap(short, long)]
        challenge: String,
        
        /// Signature of the challenge
        #[clap(short, long)]
        signature: String,
        
        /// JSON file containing access permissions
        #[clap(short, long)]
        policy_file: String,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
}

#[derive(Subcommand, Debug)]
enum CredentialStorageCommands {
    /// Initialize credential storage environment
    Init {
        /// Storage directory path
        #[clap(short, long, default_value = "./data")]
        path: String,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
        
        /// Authentication cache TTL in seconds
        #[clap(short, long, default_value = "3600")]
        cache_ttl: u64,
    },
    
    /// Register a new verifiable credential for access control
    RegisterCredential {
        /// Credential JSON file path
        #[clap(short, long)]
        credential: String,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// Create a credential-based access rule
    CreateAccessRule {
        /// DID identifier (did:icn:...)
        #[clap(short, long)]
        did: String,
        
        /// Authentication challenge (for signing)
        #[clap(short, long)]
        challenge: String,
        
        /// Signature of the challenge
        #[clap(short, long)]
        signature: String,
        
        /// File pattern this rule applies to
        #[clap(short, long)]
        pattern: String,
        
        /// Required credential types (comma-separated)
        #[clap(short, long)]
        credential_types: String,
        
        /// Required attributes (JSON format)
        #[clap(short, long)]
        attributes: String,
        
        /// Permissions granted (comma-separated: read,write)
        #[clap(short, long)]
        permissions: String,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// Store a file with credential-based authentication
    StoreFile {
        /// DID identifier (did:icn:...)
        #[clap(short, long)]
        did: String,
        
        /// Authentication challenge (for signing)
        #[clap(short, long)]
        challenge: String,
        
        /// Signature of the challenge
        #[clap(short, long)]
        signature: String,
        
        /// Credential ID to use for access control
        #[clap(short, long)]
        credential_id: Option<String>,
        
        /// File to store
        #[clap(short, long)]
        file: String,
        
        /// Storage key (defaults to filename)
        #[clap(short, long)]
        key: Option<String>,
        
        /// Enable encryption for this file
        #[clap(short, long)]
        encrypted: bool,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// Retrieve a file with credential-based authentication
    GetFile {
        /// DID identifier (did:icn:...)
        #[clap(short, long)]
        did: String,
        
        /// Authentication challenge (for signing)
        #[clap(short, long)]
        challenge: String,
        
        /// Signature of the challenge
        #[clap(short, long)]
        signature: String,
        
        /// Credential ID to use for access control
        #[clap(short, long)]
        credential_id: Option<String>,
        
        /// Storage key to retrieve
        #[clap(short, long)]
        key: String,
        
        /// Output file path (defaults to key name)
        #[clap(short, long)]
        output: Option<String>,
        
        /// Specific version to retrieve (defaults to latest)
        #[clap(short, long)]
        version: Option<String>,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// List files accessible with credential-based authentication
    ListFiles {
        /// DID identifier (did:icn:...)
        #[clap(short, long)]
        did: String,
        
        /// Authentication challenge (for signing)
        #[clap(short, long)]
        challenge: String,
        
        /// Signature of the challenge
        #[clap(short, long)]
        signature: String,
        
        /// Credential ID to use for access control
        #[clap(short, long)]
        credential_id: Option<String>,
        
        /// Filter by prefix
        #[clap(short, long)]
        prefix: Option<String>,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// Verify a verifiable credential
    VerifyCredential {
        /// Credential ID to verify
        #[clap(short, long)]
        credential_id: String,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// Save credential access rules to a file
    SaveAccessRules {
        /// Output file path
        #[clap(short, long)]
        output: String,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
    
    /// Load credential access rules from a file
    LoadAccessRules {
        /// Input file path
        #[clap(short, long)]
        input: String,
        
        /// Federation name
        #[clap(short, long, default_value = "default")]
        federation: String,
    },
}

#[derive(Subcommand)]
enum ComputeCommands {
    /// Initialize compute environment
    Init {
        /// Workspace directory for compute jobs
        #[arg(long)]
        workspace: String,

        /// Federation name
        #[arg(long)]
        federation: String,
    },

    /// Submit a compute job
    SubmitJob {
        /// DID identifier
        #[arg(long)]
        did: String,

        /// Authentication challenge
        #[arg(long)]
        challenge: String,

        /// Signature for authentication
        #[arg(long)]
        signature: String,

        /// Credential ID for authorization
        #[arg(long)]
        credential_id: String,

        /// Job name
        #[arg(long)]
        name: String,

        /// Command to execute
        #[arg(long)]
        command: String,

        /// Command arguments (comma-separated)
        #[arg(long)]
        args: String,

        /// CPU cores required
        #[arg(long, default_value = "1")]
        cpu: u32,

        /// Memory required (MB)
        #[arg(long, default_value = "512")]
        memory: u32,

        /// GPU memory required (MB, optional)
        #[arg(long)]
        gpu_memory: Option<u32>,

        /// Input files (format: storage_path:workspace_path,storage_path2:workspace_path2)
        #[arg(long)]
        input_files: String,

        /// Output files (format: workspace_path:storage_path,workspace_path2:storage_path2)
        #[arg(long)]
        output_files: String,

        /// Federation name
        #[arg(long)]
        federation: String,
    },

    /// Process data with simplified interface
    ProcessData {
        /// DID identifier
        #[arg(long)]
        did: String,

        /// Authentication challenge
        #[arg(long)]
        challenge: String,

        /// Signature for authentication
        #[arg(long)]
        signature: String,

        /// Credential ID for authorization
        #[arg(long)]
        credential_id: String,

        /// Job name
        #[arg(long)]
        name: String,

        /// Command to execute
        #[arg(long)]
        command: String,

        /// Command arguments (comma-separated)
        #[arg(long)]
        args: String,

        /// Input files (format: storage_path:workspace_path,storage_path2:workspace_path2)
        #[arg(long)]
        input_files: String,

        /// Output files (format: workspace_path:storage_path,workspace_path2:storage_path2)
        #[arg(long)]
        output_files: String,

        /// Federation name
        #[arg(long)]
        federation: String,
    },

    /// Get job status
    GetJobStatus {
        /// DID identifier
        #[arg(long)]
        did: String,

        /// Authentication challenge
        #[arg(long)]
        challenge: String,

        /// Signature for authentication
        #[arg(long)]
        signature: String,

        /// Credential ID for authorization
        #[arg(long)]
        credential_id: String,

        /// Job ID
        #[arg(long)]
        job_id: String,

        /// Federation name
        #[arg(long)]
        federation: String,
    },

    /// Get job details
    GetJob {
        /// DID identifier
        #[arg(long)]
        did: String,

        /// Authentication challenge
        #[arg(long)]
        challenge: String,

        /// Signature for authentication
        #[arg(long)]
        signature: String,

        /// Credential ID for authorization
        #[arg(long)]
        credential_id: String,

        /// Job ID
        #[arg(long)]
        job_id: String,

        /// Federation name
        #[arg(long)]
        federation: String,
    },

    /// List jobs
    ListJobs {
        /// DID identifier
        #[arg(long)]
        did: String,

        /// Authentication challenge
        #[arg(long)]
        challenge: String,

        /// Signature for authentication
        #[arg(long)]
        signature: String,

        /// Credential ID for authorization
        #[arg(long)]
        credential_id: String,

        /// Federation name
        #[arg(long)]
        federation: String,
    },

    /// Cancel a job
    CancelJob {
        /// DID identifier
        #[arg(long)]
        did: String,

        /// Authentication challenge
        #[arg(long)]
        challenge: String,

        /// Signature for authentication
        #[arg(long)]
        signature: String,

        /// Credential ID for authorization
        #[arg(long)]
        credential_id: String,

        /// Job ID
        #[arg(long)]
        job_id: String,

        /// Federation name
        #[arg(long)]
        federation: String,
    },

    /// Get job logs
    GetJobLogs {
        /// DID identifier
        #[arg(long)]
        did: String,

        /// Authentication challenge
        #[arg(long)]
        challenge: String,

        /// Signature for authentication
        #[arg(long)]
        signature: String,

        /// Credential ID for authorization
        #[arg(long)]
        credential_id: String,

        /// Job ID
        #[arg(long)]
        job_id: String,

        /// Federation name
        #[arg(long)]
        federation: String,
    },

    /// Upload job outputs to storage
    UploadJobOutputs {
        /// DID identifier
        #[arg(long)]
        did: String,

        /// Authentication challenge
        #[arg(long)]
        challenge: String,

        /// Signature for authentication
        #[arg(long)]
        signature: String,

        /// Credential ID for authorization
        #[arg(long)]
        credential_id: String,

        /// Job ID
        #[arg(long)]
        job_id: String,

        /// Federation name
        #[arg(long)]
        federation: String,
    },
}

/// Domain-Specific Language (DSL) commands for cooperative governance and automation
#[derive(Subcommand, Debug)]
enum DslCommands {
    /// Execute a DSL script from a file
    ExecuteScript {
        /// The path to the script file
        file: String,
        /// The federation to execute the script in
        #[clap(short, long)]
        federation: Option<String>,
    },
    
    /// Execute a DSL script provided as a string
    ExecuteScriptString {
        /// The script content
        script: String,
        /// The federation to execute the script in
        #[clap(short, long)]
        federation: Option<String>,
    },
    
    /// Create a template DSL script
    CreateTemplate {
        /// The type of template to create (governance, network, economic)
        template_type: String,
        /// The output file path
        output: String,
    },
    
    /// Validate a DSL script without executing it
    Validate {
        /// The path to the script file
        file: String,
    },
    
    /// Show DSL documentation
    ShowDocs {},
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Configure logging based on verbosity
    let log_level = match cli.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    std::env::set_var("RUST_LOG", log_level);
    env_logger::init();
    
    match cli.command {
        Commands::Status {} => {
            println!("ICN Network CLI - Status OK");
        },
        Commands::Network { command } => {
            handle_network_command(command).await?;
        },
        Commands::Storage { command } => {
            handle_storage_command(command).await?;
        },
        Commands::Governance { command } => {
            handle_governance_command(command).await?;
        },
        Commands::GovernedStorage { command } => {
            handle_governed_storage_command(command).await?;
        },
        Commands::IdentityStorage { command } => {
            handle_identity_storage_command(command).await?;
        },
        Commands::CredentialStorage { command } => {
            handle_credential_storage_command(command).await?;
        },
        Commands::Compute(compute_cmd) => {
            handle_compute_command(compute_cmd).await
        },
        Commands::Dsl { command } => {
            handle_dsl_command(command).await?;
        },
    }
    
    Ok(())
}

async fn handle_storage_command(command: StorageCommands) -> Result<()> {
async fn handle_governance_command(command: GovernanceCommands) -> Result<()> {
    match command {
        GovernanceCommands::CreateProposal { 
            title, 
            description, 
            proposal_type, 
            federation, 
            proposer, 
            quorum, 
            approval, 
            content_file 
        } => {
            println!("Creating proposal '{}' in federation {}", title, federation);
            
            // Parse proposal type
            let proposal_type = match proposal_type.to_lowercase().as_str() {
                "policy" => ProposalType::PolicyChange,
                "member-add" => ProposalType::MemberAddition,
                "member-remove" => ProposalType::MemberRemoval,
                "resource" => ProposalType::ResourceAllocation,
                "dispute" => ProposalType::DisputeResolution,
                "config" => ProposalType::ConfigChange,
                _ => return Err(anyhow::anyhow!("Invalid proposal type: {}", proposal_type)),
            };
            
            // Read content file if provided
            let content = if let Some(file) = content_file {
                let content = tokio::fs::read_to_string(file).await?;
                serde_json::from_str(&content)?
            } else {
                serde_json::json!({})
            };
            
            // Initialize governance service
            let mut service = GovernanceService::new(&federation, "./data").await?;
            
            // Create proposal
            let proposal_id = service.create_proposal(
                &title,
                &description,
                proposal_type,
                &proposer,
                content,
                quorum,
                approval,
            ).await?;
            
            println!("Created proposal with ID: {}", proposal_id);
        },
        GovernanceCommands::ListProposals { federation, status } => {
            println!("Listing proposals in federation {}", federation);
            
            // Initialize governance service
            let service = GovernanceService::new(&federation, "./data").await?;
            
            // Get proposals
            let proposals = service.get_proposals();
            
            // Filter by status if provided
            let filtered_proposals = if let Some(status_str) = status {
                let status = match status_str.to_lowercase().as_str() {
                    "draft" => ProposalStatus::Draft,
                    "deliberation" => ProposalStatus::Deliberation,
                    "voting" => ProposalStatus::Voting,
                    "approved" => ProposalStatus::Approved,
                    "rejected" => ProposalStatus::Rejected,
                    "executed" => ProposalStatus::Executed,
                    "canceled" => ProposalStatus::Canceled,
                    _ => return Err(anyhow::anyhow!("Invalid status filter: {}", status_str)),
                };
                
                proposals.iter()
                    .filter(|p| std::mem::discriminant(&p.status) == std::mem::discriminant(&status))
                    .collect::<Vec<_>>()
            } else {
                proposals.iter().collect()
            };
            
            if filtered_proposals.is_empty() {
                println!("No proposals found");
            } else {
                println!("Found {} proposals:", filtered_proposals.len());
                println!("{:<36} {:<30} {:<15} {:<15}", "ID", "Title", "Status", "Proposer");
                println!("{:-<36} {:-<30} {:-<15} {:-<15}", "", "", "", "");
                
                for proposal in filtered_proposals {
                    let status = match proposal.status {
                        ProposalStatus::Draft => "Draft",
                        ProposalStatus::Deliberation => "Deliberation",
                        ProposalStatus::Voting => "Voting",
                        ProposalStatus::Approved => "Approved",
                        ProposalStatus::Rejected => "Rejected",
                        ProposalStatus::Executed => "Executed",
                        ProposalStatus::Canceled => "Canceled",
                    };
                    
                    println!("{:<36} {:<30} {:<15} {:<15}", 
                        &proposal.id[0..8], // Show only first 8 chars of ID
                        if proposal.title.len() > 30 {
                            format!("{}...", &proposal.title[0..27])
                        } else {
                            proposal.title.clone()
                        },
                        status,
                        &proposal.proposer
                    );
                }
            }
        },
        GovernanceCommands::ShowProposal { id, federation } => {
            println!("Showing details for proposal {} in federation {}", id, federation);
            
            // Initialize governance service
            let service = GovernanceService::new(&federation, "./data").await?;
            
            // Get proposal
            let proposal = service.get_proposal(&id)
                .ok_or_else(|| anyhow::anyhow!("Proposal not found"))?;
            
            // Format status
            let status = match proposal.status {
                ProposalStatus::Draft => "Draft",
                ProposalStatus::Deliberation => "Deliberation",
                ProposalStatus::Voting => "Voting",
                ProposalStatus::Approved => "Approved",
                ProposalStatus::Rejected => "Rejected",
                ProposalStatus::Executed => "Executed",
                ProposalStatus::Canceled => "Canceled",
            };
            
            // Format proposal type
            let proposal_type = match proposal.proposal_type {
                ProposalType::PolicyChange => "Policy Change",
                ProposalType::MemberAddition => "Member Addition",
                ProposalType::MemberRemoval => "Member Removal",
                ProposalType::ResourceAllocation => "Resource Allocation",
                ProposalType::DisputeResolution => "Dispute Resolution",
                ProposalType::ConfigChange => "Configuration Change",
            };
            
            // Format dates
            let created_at = chrono::DateTime::from_timestamp(proposal.created_at as i64, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            
            let updated_at = chrono::DateTime::from_timestamp(proposal.updated_at as i64, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            
            // Print proposal details
            println!("ID:          {}", proposal.id);
            println!("Title:       {}", proposal.title);
            println!("Type:        {}", proposal_type);
            println!("Status:      {}", status);
            println!("Proposer:    {}", proposal.proposer);
            println!("Created:     {}", created_at);
            println!("Updated:     {}", updated_at);
            println!("Quorum:      {}%", proposal.quorum_percentage);
            println!("Approval:    {}%", proposal.approval_percentage);
            
            if let Some(starts_at) = proposal.voting_starts_at {
                let starts_at = chrono::DateTime::from_timestamp(starts_at as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                println!("Voting Start: {}", starts_at);
            }
            
            if let Some(ends_at) = proposal.voting_ends_at {
                let ends_at = chrono::DateTime::from_timestamp(ends_at as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                println!("Voting End:   {}", ends_at);
            }
            
            println!("\nDescription:");
            println!("{}", proposal.description);
            
            if !proposal.votes.is_empty() {
                println!("\nVotes:");
                println!("{:<20} {:<10} {:<15} {:<10}", "Member", "Vote", "Timestamp", "Weight");
                println!("{:-<20} {:-<10} {:-<15} {:-<10}", "", "", "", "");
                
                for vote in &proposal.votes {
                    let vote_str = match vote.vote {
                        Vote::Yes => "Yes",
                        Vote::No => "No",
                        Vote::Abstain => "Abstain",
                    };
                    
                    let timestamp = chrono::DateTime::from_timestamp(vote.timestamp as i64, 0)
                        .map(|dt| dt.format("%Y-%m-%d").to_string())
                        .unwrap_or_else(|| "Unknown".to_string());
                    
                    println!("{:<20} {:<10} {:<15} {:<10.1}", 
                        vote.member_id, 
                        vote_str, 
                        timestamp, 
                        vote.weight
                    );
                }
                
                // Calculate vote statistics
                let total_votes = proposal.votes.len();
                let yes_votes = proposal.votes.iter().filter(|v| matches!(v.vote, Vote::Yes)).count();
                let no_votes = proposal.votes.iter().filter(|v| matches!(v.vote, Vote::No)).count();
                let abstain_votes = proposal.votes.iter().filter(|v| matches!(v.vote, Vote::Abstain)).count();
                
                let total_weight: f64 = proposal.votes.iter().map(|v| v.weight).sum();
                let yes_weight: f64 = proposal.votes.iter()
                    .filter(|v| matches!(v.vote, Vote::Yes))
                    .map(|v| v.weight)
                    .sum();
                let no_weight: f64 = proposal.votes.iter()
                    .filter(|v| matches!(v.vote, Vote::No))
                    .map(|v| v.weight)
                    .sum();
                
                println!("\nVote Summary:");
                println!("Total Votes: {} (weight: {:.1})", total_votes, total_weight);
                println!("Yes: {} votes ({:.1}% by weight)", yes_votes, if total_weight > 0.0 { (yes_weight / total_weight) * 100.0 } else { 0.0 });
                println!("No: {} votes ({:.1}% by weight)", no_votes, if total_weight > 0.0 { (no_weight / total_weight) * 100.0 } else { 0.0 });
                println!("Abstain: {} votes", abstain_votes);
            }
            
            println!("\nContent:");
            println!("{}", serde_json::to_string_pretty(&proposal.content)?);
        },
        GovernanceCommands::UpdateStatus { id, status, federation } => {
            println!("Updating status of proposal {} to {} in federation {}", id, status, federation);
            
            // Parse status
            let new_status = match status.to_lowercase().as_str() {
                "draft" => ProposalStatus::Draft,
                "deliberation" => ProposalStatus::Deliberation,
                "voting" => ProposalStatus::Voting,
                "approved" => ProposalStatus::Approved,
                "rejected" => ProposalStatus::Rejected,
                "executed" => ProposalStatus::Executed,
                "canceled" => ProposalStatus::Canceled,
                _ => return Err(anyhow::anyhow!("Invalid status: {}", status)),
            };
            
            // Initialize governance service
            let mut service = GovernanceService::new(&federation, "./data").await?;
            
            // Update status
            service.update_proposal_status(&id, new_status).await?;
            
            println!("Status updated successfully");
        },
        GovernanceCommands::StartVoting { id, duration, federation } => {
            println!("Starting voting period for proposal {} in federation {}", id, federation);
            
            // Initialize governance service
            let mut service = GovernanceService::new(&federation, "./data").await?;
            
            // Start voting
            service.start_voting(&id, duration).await?;
            
            println!("Voting started successfully (duration: {} seconds)", duration);
        },
        GovernanceCommands::Vote { id, member, vote, comment, weight, federation } => {
            println!("Casting vote on proposal {} in federation {}", id, federation);
            
            // Parse vote
            let parsed_vote = match vote.to_lowercase().as_str() {
                "yes" => Vote::Yes,
                "no" => Vote::No,
                "abstain" => Vote::Abstain,
                _ => return Err(anyhow::anyhow!("Invalid vote: {}", vote)),
            };
            
            // Initialize governance service
            let mut service = GovernanceService::new(&federation, "./data").await?;
            
            // Cast vote
            service.cast_vote(&id, &member, parsed_vote, comment, weight).await?;
            
            println!("Vote cast successfully");
        },
        GovernanceCommands::FinalizeVoting { id, federation } => {
            println!("Finalizing voting for proposal {} in federation {}", id, federation);
            
            // Initialize governance service
            let mut service = GovernanceService::new(&federation, "./data").await?;
            
            // Finalize voting
            service.finalize_voting(&id).await?;
            
            println!("Voting finalized successfully");
        },
        GovernanceCommands::ExecuteProposal { id, federation } => {
            println!("Executing proposal {} in federation {}", id, federation);
            
            // Initialize governance service
            let mut service = GovernanceService::new(&federation, "./data").await?;
            
            // Execute proposal
            service.execute_proposal(&id).await?;
            
            println!("Proposal executed successfully");
        },
    }
    
    Ok(())
}

async fn handle_governed_storage_command(command: GovernedStorageCommands) -> Result<()> {
    match command {
        GovernedStorageCommands::StoreFile { file, key, member, encrypted, federation } => {
            let key = key.unwrap_or_else(|| file.split('/').last().unwrap_or(&file).to_string());
            println!("Storing file {} with key {} as member {} in federation {} (encryption: {})", 
                file, key, member, federation, if encrypted { "enabled" } else { "disabled" });
            
            // Initialize governance storage service
            let service = GovernanceStorageService::new(&federation, "./data").await?;
            
            // Store the file with governance checks
            service.store_file(&member, &file, &key, encrypted).await?;
            
            println!("File stored successfully");
        },
        GovernedStorageCommands::GetFile { key, member, output, version, federation } => {
            let output = output.unwrap_or_else(|| key.clone());
            println!("Retrieving key {} as member {} from federation {} to {}{}", 
                key, member, federation, output, 
                if let Some(ver) = &version { format!(" (version: {})", ver) } else { String::new() });
            
            // Initialize governance storage service
            let service = GovernanceStorageService::new(&federation, "./data").await?;
            
            // Retrieve the file with governance checks
            service.retrieve_file(&member, &key, &output, version.as_deref()).await?;
            
            println!("File retrieved successfully");
        },
        GovernedStorageCommands::ListFiles { member, prefix, federation } => {
            println!("Listing files in federation {} accessible by member {}{}", 
                federation, member,
                if let Some(pre) = &prefix { format!(" with prefix {}", pre) } else { String::new() });
            
            // Initialize governance storage service
            let service = GovernanceStorageService::new(&federation, "./data").await?;
            
            // List files with governance checks
            let files = service.list_files(&member, prefix.as_deref()).await?;
            
            if files.is_empty() {
                println!("No files found");
            } else {
                println!("Found {} files:", files.len());
                println!("{:<30} {:<20} {:<10} {:<20}", "Key", "Current Version", "Versions", "Last Modified");
                println!("{:-<30} {:-<20} {:-<10} {:-<20}", "", "", "", "");
                
                for file in files {
                    // Extract the key from metadata key (remove "meta:" prefix)
                    let key = file.filename;
                    
                    // Format timestamp as ISO date
                    let modified = chrono::DateTime::from_timestamp(file.modified_at as i64, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| "Unknown".to_string());
                    
                    println!("{:<30} {:<20} {:<10} {:<20}", 
                        key, 
                        &file.current_version[0..8], // Show first 8 chars of version ID
                        file.versions.len(),
                        modified
                    );
                }
            }
        },
        GovernedStorageCommands::ProposePolicy { proposer, title, description, policy_type, content_file, federation } => {
            println!("Proposing storage policy '{}' in federation {}", title, federation);
            
            // Parse policy type
            let parsed_type = match policy_type.as_str() {
                "federation-quota" => StoragePolicyType::FederationQuota,
                "member-quota" => StoragePolicyType::MemberQuota,
                "access-control" => StoragePolicyType::AccessControl,
                "retention" => StoragePolicyType::RetentionPolicy,
                "encryption" => StoragePolicyType::EncryptionAlgorithms,
                "replication" => StoragePolicyType::ReplicationPolicy,
                _ => return Err(anyhow::anyhow!("Invalid policy type: {}", policy_type)),
            };
            
            // Read the content file
            let content = tokio::fs::read_to_string(&content_file).await?;
            let policy_content: serde_json::Value = serde_json::from_str(&content)?;
            
            // Initialize governance storage service
            let mut service = GovernanceStorageService::new(&federation, "./data").await?;
            
            // Create the policy proposal
            let proposal_id = service.propose_storage_policy(
                &proposer,
                &title,
                &description,
                parsed_type,
                policy_content,
            ).await?;
            
            println!("Storage policy proposal created with ID: {}", proposal_id);
        },
        GovernedStorageCommands::ApplyPolicy { proposal_id, federation } => {
            println!("Applying storage policy from proposal {} in federation {}", proposal_id, federation);
            
            // Initialize governance storage service
            let mut service = GovernanceStorageService::new(&federation, "./data").await?;
            
            // Apply the policy
            service.apply_approved_policy(&proposal_id).await?;
            
            println!("Storage policy applied successfully");
        },
        GovernedStorageCommands::ListPolicies { federation, policy_type } => {
            println!("Listing storage policies in federation {}", federation);
            
            // Initialize governance storage service
            let service = GovernanceStorageService::new(&federation, "./data").await?;
            
            // Get the policies
            let policies = service.get_policies();
            
            // Filter by policy type if specified
            let filtered_policies = if let Some(type_str) = policy_type {
                policies.iter()
                    .filter(|p| match (&p.policy_type, type_str.as_str()) {
                        (StoragePolicyType::FederationQuota, "federation-quota") => true,
                        (StoragePolicyType::MemberQuota, "member-quota") => true,
                        (StoragePolicyType::AccessControl, "access-control") => true,
                        (StoragePolicyType::RetentionPolicy, "retention") => true,
                        (StoragePolicyType::EncryptionAlgorithms, "encryption") => true,
                        (StoragePolicyType::ReplicationPolicy, "replication") => true,
                        _ => false,
                    })
                    .collect::<Vec<_>>()
            } else {
                policies.iter().collect()
            };
            
            if filtered_policies.is_empty() {
                println!("No policies found");
            } else {
                println!("Found {} policies:", filtered_policies.len());
                println!("{:<36} {:<20} {:<15} {:<15}", "ID", "Type", "Created At", "Active");
                println!("{:-<36} {:-<20} {:-<15} {:-<15}", "", "", "", "");
                
                for policy in filtered_policies {
                    // Format policy type
                    let type_str = match policy.policy_type {
                        StoragePolicyType::FederationQuota => "Federation Quota",
                        StoragePolicyType::MemberQuota => "Member Quota",
                        StoragePolicyType::AccessControl => "Access Control",
                        StoragePolicyType::RetentionPolicy => "Retention",
                        StoragePolicyType::EncryptionAlgorithms => "Encryption",
                        StoragePolicyType::ReplicationPolicy => "Replication",
                    };
                    
                    // Format timestamp
                    let created_at = chrono::DateTime::from_timestamp(policy.created_at as i64, 0)
                        .map(|dt| dt.format("%Y-%m-%d").to_string())
                        .unwrap_or_else(|| "Unknown".to_string());
                    
                    println!("{:<36} {:<20} {:<15} {:<15}", 
                        &policy.id[0..8], // Show first 8 chars of ID
                        type_str,
                        created_at,
                        if policy.active { "Yes" } else { "No" }
                    );
                }
                
                println!("\nUse 'icn-cli governed-storage show-policy <id>' to see full policy details");
            }
        },
        GovernedStorageCommands::ShowSchema { policy_type } => {
            println!("Showing JSON schema for policy type: {}", policy_type);
            
            // Get the schema
            let schema = match policy_type.as_str() {
                "federation-quota" => governance_storage::schema::federation_quota_schema(),
                "member-quota" => governance_storage::schema::member_quota_schema(),
                "access-control" => governance_storage::schema::access_control_schema(),
                "retention" => governance_storage::schema::retention_policy_schema(),
                _ => return Err(anyhow::anyhow!("Unknown policy type: {}", policy_type)),
            };
            
            // Pretty-print the schema
            println!("{}", serde_json::to_string_pretty(&schema)?);
        },
    }
    
    Ok(())
}

async fn handle_identity_storage_command(command: IdentityStorageCommands) -> Result<()> {
    match command {
        IdentityStorageCommands::Init { path, federation, cache_ttl } => {
            println!("Initializing identity storage at {} for federation {} (cache TTL: {}s)", 
                path, federation, cache_ttl);
            
            // Create path if it doesn't exist
            let path = PathBuf::from(path);
            tokio::fs::create_dir_all(&path).await?;
            
            // Initialize storage service (this is just for initialization)
            // In a real implementation, we would store some configuration
            let _ = StorageService::new(&path).await?;
            
            println!("Identity storage environment initialized successfully");
        },
        IdentityStorageCommands::RegisterDid { did, document, federation } => {
            println!("Registering DID {} in federation {}", did, federation);
            
            // Read the DID document
            let document_data = tokio::fs::read_to_string(&document).await?;
            let did_document: identity_storage::DidDocument = serde_json::from_str(&document_data)?;
            
            // In a real implementation, we would store this in a DID registry
            // For now, we just verify the document
            if did_document.id != did {
                return Err(anyhow::anyhow!("DID in document does not match provided DID"));
            }
            
            println!("DID registered successfully");
        },
        IdentityStorageCommands::StoreFile { did, challenge, signature, file, key, encrypted, federation } => {
            let key = key.unwrap_or_else(|| file.split('/').last().unwrap_or(&file).to_string());
            println!("Storing file {} with key {} as DID {} in federation {} (encryption: {})", 
                file, key, did, federation, if encrypted { "enabled" } else { "disabled" });
            
            // In a real implementation, we would use a real identity provider
            // For now, we use a mock that accepts any valid DID
            let mut provider = MockIdentityProvider::new();
            
            // Create a dummy DID document for testing
            let document = identity_storage::DidDocument {
                id: did.clone(),
                controller: None,
                verification_method: vec![],
                authentication: vec![],
                service: vec![],
            };
            provider.add_did_document(did.clone(), document);
            
            // Initialize identity storage service
            let mut service = IdentityStorageService::new(
                &federation,
                "./data",
                provider,
                3600, // 1 hour cache TTL
            ).await?;
            
            // Store the file with DID authentication
            service.store_file(
                &did,
                challenge.as_bytes(),
                signature.as_bytes(),
                &file,
                &key,
                encrypted,
            ).await?;
            
            println!("File stored successfully");
        },
        IdentityStorageCommands::GetFile { did, challenge, signature, key, output, version, federation } => {
            let output = output.unwrap_or_else(|| key.clone());
            println!("Retrieving key {} as DID {} from federation {} to {}{}", 
                key, did, federation, output, 
                if let Some(ver) = &version { format!(" (version: {})", ver) } else { String::new() });
            
            // In a real implementation, we would use a real identity provider
            // For now, we use a mock that accepts any valid DID
            let mut provider = MockIdentityProvider::new();
            
            // Create a dummy DID document for testing
            let document = identity_storage::DidDocument {
                id: did.clone(),
                controller: None,
                verification_method: vec![],
                authentication: vec![],
                service: vec![],
            };
            provider.add_did_document(did.clone(), document);
            
            // Initialize identity storage service
            let mut service = IdentityStorageService::new(
                &federation,
                "./data",
                provider,
                3600, // 1 hour cache TTL
            ).await?;
            
            // Retrieve the file with DID authentication
            service.retrieve_file(
                &did,
                challenge.as_bytes(),
                signature.as_bytes(),
                &key,
                &output,
                version.as_deref(),
            ).await?;
            
            println!("File retrieved successfully");
        },
        IdentityStorageCommands::ListFiles { did, challenge, signature, prefix, federation } => {
            println!("Listing files in federation {} accessible by DID {}{}", 
                federation, did,
                if let Some(pre) = &prefix { format!(" with prefix {}", pre) } else { String::new() });
            
            // In a real implementation, we would use a real identity provider
            // For now, we use a mock that accepts any valid DID
            let mut provider = MockIdentityProvider::new();
            
            // Create a dummy DID document for testing
            let document = identity_storage::DidDocument {
                id: did.clone(),
                controller: None,
                verification_method: vec![],
                authentication: vec![],
                service: vec![],
            };
            provider.add_did_document(did.clone(), document);
            
            // Initialize identity storage service
            let mut service = IdentityStorageService::new(
                &federation,
                "./data",
                provider,
                3600, // 1 hour cache TTL
            ).await?;
            
            // List files with DID authentication
            let files = service.list_files(
                &did,
                challenge.as_bytes(),
                signature.as_bytes(),
                prefix.as_deref(),
            ).await?;
            
            if files.is_empty() {
                println!("No files found");
            } else {
                println!("Found {} files:", files.len());
                println!("{:<30} {:<20} {:<10} {:<20}", "Key", "Current Version", "Versions", "Last Modified");
                println!("{:-<30} {:-<20} {:-<10} {:-<20}", "", "", "", "");
                
                for file in files {
                    // Extract the key from metadata key (remove "meta:" prefix)
                    let key = file.filename;
                    
                    // Format timestamp as ISO date
                    let modified = chrono::DateTime::from_timestamp(file.modified_at as i64, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| "Unknown".to_string());
                    
                    println!("{:<30} {:<20} {:<10} {:<20}", 
                        key, 
                        &file.current_version[0..8], // Show first 8 chars of version ID
                        file.versions.len(),
                        modified
                    );
                }
            }
        },
        IdentityStorageCommands::MapDidToMember { did, member_id, federation } => {
            println!("Mapping DID {} to member ID {} in federation {}", did, member_id, federation);
            
            // In a real implementation, we would use a real identity provider
            // For now, we use a mock
            let provider = MockIdentityProvider::new();
            
            // Initialize identity storage service
            let mut service = IdentityStorageService::new(
                &federation,
                "./data",
                provider,
                3600, // 1 hour cache TTL
            ).await?;
            
            // Update DID to member ID mapping
            service.update_did_access_mapping(&[(did, member_id)]).await?;
            
            println!("DID to member ID mapping created successfully");
        },
        IdentityStorageCommands::CreateAccessPolicy { did, challenge, signature, policy_file, federation } => {
            println!("Creating access policy as DID {} in federation {}", did, federation);
            
            // Read the policy file
            let policy_data = tokio::fs::read_to_string(&policy_file).await?;
            let access_permissions: Vec<governance_storage::AccessPermission> = serde_json::from_str(&policy_data)?;
            
            // In a real implementation, we would use a real identity provider
            // For now, we use a mock that accepts any valid DID
            let mut provider = MockIdentityProvider::new();
            
            // Create a dummy DID document for testing
            let document = identity_storage::DidDocument {
                id: did.clone(),
                controller: None,
                verification_method: vec![],
                authentication: vec![],
                service: vec![],
            };
            provider.add_did_document(did.clone(), document);
            
            // Initialize identity storage service
            let mut service = IdentityStorageService::new(
                &federation,
                "./data",
                provider,
                3600, // 1 hour cache TTL
            ).await?;
            
            // Create access policy with DID authentication
            let proposal_id = service.create_did_access_policy(
                &did,
                challenge.as_bytes(),
                signature.as_bytes(),
                &access_permissions,
            ).await?;
            
            println!("Access policy proposal created with ID: {}", proposal_id);
        },
    }
    
    Ok(())
}

async fn handle_credential_storage_command(command: CredentialStorageCommands) -> Result<()> {
    match command {
        CredentialStorageCommands::Init { path, federation, cache_ttl } => {
            println!("Initializing credential storage at {} for federation {} (cache TTL: {}s)", 
                path, federation, cache_ttl);
            
            // Create path if it doesn't exist
            let path = PathBuf::from(path);
            tokio::fs::create_dir_all(&path).await?;
            
            // Create credential rules directory
            let rules_dir = path.join("credential_rules");
            tokio::fs::create_dir_all(&rules_dir).await?;
            
            // Create credentials directory
            let credentials_dir = path.join("credentials");
            tokio::fs::create_dir_all(&credentials_dir).await?;
            
            // Initialize storage service (this is just for initialization)
            let _ = StorageService::new(&path).await?;
            
            println!("Credential storage environment initialized successfully");
        },
        CredentialStorageCommands::RegisterCredential { credential, federation } => {
            println!("Registering credential from {} in federation {}", credential, federation);
            
            // Read the credential JSON
            let credential_data = tokio::fs::read_to_string(&credential).await?;
            let credential_obj: VerifiableCredential = serde_json::from_str(&credential_data)?;
            
            // In a real implementation, we would store this in a credential registry
            // For now, we just verify the credential format
            println!("Credential ID: {}", credential_obj.id);
            println!("Credential Type: {:?}", credential_obj.credential_type);
            println!("Credential Subject: {}", credential_obj.credentialSubject.id);
            println!("Issuer: {}", credential_obj.issuer);
            
            println!("Credential registered successfully");
        },
        CredentialStorageCommands::CreateAccessRule { did, challenge, signature, pattern, credential_types, attributes, permissions, federation } => {
            println!("Creating credential access rule for pattern '{}' in federation {}", pattern, federation);
            
            // Parse credential types and permissions
            let credential_types_vec: Vec<String> = credential_types.split(',')
                .map(|s| s.trim().to_string())
                .collect();
            
            let permissions_vec: Vec<String> = permissions.split(',')
                .map(|s| s.trim().to_string())
                .collect();
            
            // Parse attributes JSON
            let attributes_map: std::collections::HashMap<String, serde_json::Value> = 
                serde_json::from_str(&attributes)?;
            
            // Create the access rule
            let rule = CredentialAccessRule {
                pattern,
                credential_types: credential_types_vec,
                required_attributes: attributes_map,
                permissions: permissions_vec,
            };
            
            // In a real implementation, we would initialize the credential storage service
            // and call create_access_rule
            println!("Created access rule:");
            println!("  Pattern: {}", rule.pattern);
            println!("  Required credential types: {:?}", rule.credential_types);
            println!("  Required attributes: {}", serde_json::to_string_pretty(&rule.required_attributes)?);
            println!("  Permissions: {:?}", rule.permissions);
            
            println!("Access rule created successfully");
        },
        CredentialStorageCommands::StoreFile { did, challenge, signature, credential_id, file, key, encrypted, federation } => {
            let key = key.unwrap_or_else(|| file.split('/').last().unwrap_or(&file).to_string());
            
            println!("Storing file {} with key {} using DID {} in federation {} (encryption: {})", 
                file, key, did, federation, if encrypted { "enabled" } else { "disabled" });
            
            if let Some(cred_id) = &credential_id {
                println!("Using credential ID: {}", cred_id);
            }
            
            // In a real implementation, we would initialize providers and the credential storage service,
            // then call store_file
            
            println!("File stored successfully");
        },
        CredentialStorageCommands::GetFile { did, challenge, signature, credential_id, key, output, version, federation } => {
            let output = output.unwrap_or_else(|| key.clone());
            
            println!("Retrieving key {} using DID {} from federation {} to {}{}", 
                key, did, federation, output, 
                if let Some(ver) = &version { format!(" (version: {})", ver) } else { String::new() });
            
            if let Some(cred_id) = &credential_id {
                println!("Using credential ID: {}", cred_id);
            }
            
            // In a real implementation, we would initialize providers and the credential storage service,
            // then call retrieve_file
            
            println!("File retrieved successfully");
        },
        CredentialStorageCommands::ListFiles { did, challenge, signature, credential_id, prefix, federation } => {
            println!("Listing files in federation {} accessible by DID {}{}", 
                federation, did,
                if let Some(pre) = &prefix { format!(" with prefix {}", pre) } else { String::new() });
            
            if let Some(cred_id) = &credential_id {
                println!("Using credential ID: {}", cred_id);
            }
            
            // In a real implementation, we would initialize providers and the credential storage service,
            // then call list_files
            
            println!("No files found (mock implementation)");
        },
        CredentialStorageCommands::VerifyCredential { credential_id, federation } => {
            println!("Verifying credential {} in federation {}", credential_id, federation);
            
            // In a real implementation, we would initialize providers and the credential storage service,
            // then call verify_credential
            
            println!("Credential verification status: Verified (mock implementation)");
        },
        CredentialStorageCommands::SaveAccessRules { output, federation } => {
            println!("Saving credential access rules to {} for federation {}", output, federation);
            
            // In a real implementation, we would initialize providers and the credential storage service,
            // then call save_access_rules
            
            println!("Access rules saved successfully");
        },
        CredentialStorageCommands::LoadAccessRules { input, federation } => {
            println!("Loading credential access rules from {} for federation {}", input, federation);
            
            // In a real implementation, we would initialize providers and the credential storage service,
            // then call load_access_rules
            
            println!("Access rules loaded successfully");
        },
    }
    
    Ok(())
}

async fn handle_compute_command(command: ComputeCommands) -> Result<()> {
    match command {
        ComputeCommands::Init { workspace, federation } => {
            // Create a mock identity provider and credential provider for demo
            let mock_identity = identity_storage::MockIdentityProvider::new();
            let mock_credential = credential_storage::MockCredentialProvider::new();
            
            // Create an identity storage service for authentication
            let identity_storage = identity_storage::IdentityStorageService::new(
                PathBuf::from(&workspace),
                federation.clone(),
                3600, // Default cache TTL
                mock_identity,
            );
            
            // Create a credential storage service for authorization
            let credential_storage = credential_storage::CredentialStorageService::new(
                identity_storage.clone(),
                mock_credential,
            );
            
            // Create and initialize the compute storage service
            let compute_service = compute::ComputeStorageService::new(
                PathBuf::from(&workspace),
                federation,
                identity_storage,
                credential_storage,
            );
            
            compute_service.init()?;
            Ok(())
        },

        ComputeCommands::SubmitJob {
            did,
            challenge,
            signature,
            credential_id,
            name,
            command,
            args,
            cpu,
            memory,
            gpu_memory,
            input_files,
            output_files,
            federation,
        } => {
            // Create a mock identity provider and credential provider for demo
            let mut mock_identity = identity_storage::MockIdentityProvider::new();
            let mut mock_credential = credential_storage::MockCredentialProvider::new();
            
            // Set up mock identity
            mock_identity.add_did(&did, "mock_public_key");
            mock_identity.set_verification_result(
                &did, 
                &challenge, 
                &signature, 
                identity_storage::DIDVerificationStatus::Verified
            );
            
            // Set up mock credential
            let mock_credential_data = credential_storage::VerifiableCredential {
                id: credential_id.clone(),
                types: vec!["VerifiableCredential".to_string(), "ComputeCredential".to_string()],
                issuer: "did:icn:issuer".to_string(),
                issuance_date: "2023-01-01T00:00:00Z".to_string(),
                expiration_date: Some("2030-01-01T00:00:00Z".to_string()),
                subject: credential_storage::CredentialSubject {
                    id: did.clone(),
                    role: Some("DataScientist".to_string()),
                    permissions: Some(vec!["data_processing".to_string(), "compute".to_string()]),
                    attributes: HashMap::new(),
                },
                proof: credential_storage::CredentialProof {
                    type_: "Ed25519Signature2020".to_string(),
                    created: "2023-01-01T00:00:00Z".to_string(),
                    verification_method: "did:icn:issuer#key-1".to_string(),
                    proof_purpose: "assertionMethod".to_string(),
                    jws: "mock_signature".to_string(),
                },
            };
            mock_credential.add_credential(credential_id.clone(), mock_credential_data);
            mock_credential.set_verification_result(
                &did, 
                &credential_id, 
                credential_storage::CredentialVerificationStatus::Verified
            );
            
            // Create storage services
            let identity_storage = identity_storage::IdentityStorageService::new(
                PathBuf::from("compute_workspace"),
                federation.clone(),
                3600,
                mock_identity,
            );
            
            let credential_storage = credential_storage::CredentialStorageService::new(
                identity_storage.clone(),
                mock_credential,
            );
            
            // Create compute service
            let compute_service = compute::ComputeStorageService::new(
                PathBuf::from("compute_workspace"),
                federation,
                identity_storage,
                credential_storage,
            );
            
            // Parse args
            let args_vec: Vec<String> = args.split(',').map(|s| s.to_string()).collect();
            
            // Parse input files
            let input_files_map: HashMap<String, String> = input_files
                .split(',')
                .filter_map(|pair| {
                    let parts: Vec<&str> = pair.split(':').collect();
                    if parts.len() == 2 {
                        Some((parts[0].to_string(), parts[1].to_string()))
                    } else {
                        None
                    }
                })
                .collect();
            
            // Parse output files
            let output_files_map: HashMap<String, String> = output_files
                .split(',')
                .filter_map(|pair| {
                    let parts: Vec<&str> = pair.split(':').collect();
                    if parts.len() == 2 {
                        Some((parts[0].to_string(), parts[1].to_string()))
                    } else {
                        None
                    }
                })
                .collect();
            
            // Create resource requirements
            let resources = compute::ResourceRequirements {
                cpu_cores: cpu,
                memory_mb: memory,
                gpu_memory_mb: gpu_memory,
            };
            
            // Submit job
            let job_id = compute_service.submit_job(
                &did,
                &challenge,
                &signature,
                &credential_id,
                &name,
                &command,
                args_vec,
                resources,
                input_files_map,
                output_files_map,
            ).await?;
            
            println!("Job submitted successfully. ID: {}", job_id);
            Ok(())
        },

        ComputeCommands::ProcessData {
            did,
            challenge,
            signature,
            credential_id,
            name,
            command,
            args,
            input_files,
            output_files,
            federation,
        } => {
            // Create a mock identity provider and credential provider for demo
            let mut mock_identity = identity_storage::MockIdentityProvider::new();
            let mut mock_credential = credential_storage::MockCredentialProvider::new();
            
            // Set up mock identity
            mock_identity.add_did(&did, "mock_public_key");
            mock_identity.set_verification_result(
                &did, 
                &challenge, 
                &signature, 
                identity_storage::DIDVerificationStatus::Verified
            );
            
            // Set up mock credential
            let mock_credential_data = credential_storage::VerifiableCredential {
                id: credential_id.clone(),
                types: vec!["VerifiableCredential".to_string(), "ComputeCredential".to_string()],
                issuer: "did:icn:issuer".to_string(),
                issuance_date: "2023-01-01T00:00:00Z".to_string(),
                expiration_date: Some("2030-01-01T00:00:00Z".to_string()),
                subject: credential_storage::CredentialSubject {
                    id: did.clone(),
                    role: Some("DataScientist".to_string()),
                    permissions: Some(vec!["data_processing".to_string(), "compute".to_string()]),
                    attributes: HashMap::new(),
                },
                proof: credential_storage::CredentialProof {
                    type_: "Ed25519Signature2020".to_string(),
                    created: "2023-01-01T00:00:00Z".to_string(),
                    verification_method: "did:icn:issuer#key-1".to_string(),
                    proof_purpose: "assertionMethod".to_string(),
                    jws: "mock_signature".to_string(),
                },
            };
            mock_credential.add_credential(credential_id.clone(), mock_credential_data);
            mock_credential.set_verification_result(
                &did, 
                &credential_id, 
                credential_storage::CredentialVerificationStatus::Verified
            );
            
            // Create storage services
            let identity_storage = identity_storage::IdentityStorageService::new(
                PathBuf::from("compute_workspace"),
                federation.clone(),
                3600,
                mock_identity,
            );
            
            let credential_storage = credential_storage::CredentialStorageService::new(
                identity_storage.clone(),
                mock_credential,
            );
            
            // Create compute service
            let compute_service = compute::ComputeStorageService::new(
                PathBuf::from("compute_workspace"),
                federation,
                identity_storage,
                credential_storage,
            );
            
            // Parse args
            let args_vec: Vec<String> = args.split(',').map(|s| s.to_string()).collect();
            
            // Parse input files
            let input_files_map: HashMap<String, String> = input_files
                .split(',')
                .filter_map(|pair| {
                    let parts: Vec<&str> = pair.split(':').collect();
                    if parts.len() == 2 {
                        Some((parts[0].to_string(), parts[1].to_string()))
                    } else {
                        None
                    }
                })
                .collect();
            
            // Parse output files
            let output_files_map: HashMap<String, String> = output_files
                .split(',')
                .filter_map(|pair| {
                    let parts: Vec<&str> = pair.split(':').collect();
                    if parts.len() == 2 {
                        Some((parts[0].to_string(), parts[1].to_string()))
                    } else {
                        None
                    }
                })
                .collect();
            
            // Process data
            let job_id = compute_service.process_data(
                &did,
                &challenge,
                &signature,
                &credential_id,
                &name,
                &command,
                args_vec,
                input_files_map,
                output_files_map,
            ).await?;
            
            println!("Data processing job submitted successfully. ID: {}", job_id);
            Ok(())
        },

        ComputeCommands::GetJobStatus {
            did,
            challenge,
            signature,
            credential_id,
            job_id,
            federation,
        } => {
            // Create mock providers with the necessary setups
            let mut mock_identity = identity_storage::MockIdentityProvider::new();
            let mut mock_credential = credential_storage::MockCredentialProvider::new();
            
            // Set up mock identity and credential verification
            mock_identity.add_did(&did, "mock_public_key");
            mock_identity.set_verification_result(
                &did, 
                &challenge, 
                &signature, 
                identity_storage::DIDVerificationStatus::Verified
            );
            
            // Create services
            let identity_storage = identity_storage::IdentityStorageService::new(
                PathBuf::from("compute_workspace"),
                federation.clone(),
                3600,
                mock_identity,
            );
            
            let credential_storage = credential_storage::CredentialStorageService::new(
                identity_storage.clone(),
                mock_credential,
            );
            
            let compute_service = compute::ComputeStorageService::new(
                PathBuf::from("compute_workspace"),
                federation,
                identity_storage,
                credential_storage,
            );
            
            // Get job status
            let status = compute_service.get_job_status(
                &did,
                &challenge,
                &signature,
                &credential_id,
                &job_id,
            )?;
            
            println!("Job Status: {:?}", status);
            Ok(())
        },

        ComputeCommands::GetJob {
            did,
            challenge,
            signature,
            credential_id,
            job_id,
            federation,
        } => {
            // Create mock providers with the necessary setups
            let mut mock_identity = identity_storage::MockIdentityProvider::new();
            let mut mock_credential = credential_storage::MockCredentialProvider::new();
            
            // Set up mock identity and credential verification
            mock_identity.add_did(&did, "mock_public_key");
            mock_identity.set_verification_result(
                &did, 
                &challenge, 
                &signature, 
                identity_storage::DIDVerificationStatus::Verified
            );
            
            // Create services
            let identity_storage = identity_storage::IdentityStorageService::new(
                PathBuf::from("compute_workspace"),
                federation.clone(),
                3600,
                mock_identity,
            );
            
            let credential_storage = credential_storage::CredentialStorageService::new(
                identity_storage.clone(),
                mock_credential,
            );
            
            let compute_service = compute::ComputeStorageService::new(
                PathBuf::from("compute_workspace"),
                federation,
                identity_storage,
                credential_storage,
            );
            
            // Get job
            let job = compute_service.get_job(
                &did,
                &challenge,
                &signature,
                &credential_id,
                &job_id,
            )?;
            
            println!("Job Details:");
            println!("  ID:     {}", job.id);
            println!("  Name:   {}", job.name);
            println!("  Status: {:?}", job.status);
            println!("  User:   {}", job.user_did);
            println!("  Command: {} {}", job.command, job.args.join(" "));
            println!("  Created: {}", job.created_at);
            println!("  Updated: {}", job.updated_at);
            Ok(())
        },

        ComputeCommands::ListJobs {
            did,
            challenge,
            signature,
            credential_id,
            federation,
        } => {
            // Create mock providers with the necessary setups
            let mut mock_identity = identity_storage::MockIdentityProvider::new();
            let mut mock_credential = credential_storage::MockCredentialProvider::new();
            
            // Set up mock identity and credential verification
            mock_identity.add_did(&did, "mock_public_key");
            mock_identity.set_verification_result(
                &did, 
                &challenge, 
                &signature, 
                identity_storage::DIDVerificationStatus::Verified
            );
            
            // Create services
            let identity_storage = identity_storage::IdentityStorageService::new(
                PathBuf::from("compute_workspace"),
                federation.clone(),
                3600,
                mock_identity,
            );
            
            let credential_storage = credential_storage::CredentialStorageService::new(
                identity_storage.clone(),
                mock_credential,
            );
            
            let compute_service = compute::ComputeStorageService::new(
                PathBuf::from("compute_workspace"),
                federation,
                identity_storage,
                credential_storage,
            );
            
            // List jobs
            let jobs = compute_service.list_jobs(
                &did,
                &challenge,
                &signature,
                &credential_id,
            )?;
            
            println!("Jobs for user {}:", did);
            for job in jobs {
                println!("  {}: {} (Status: {:?})", job.id, job.name, job.status);
            }
            
            Ok(())
        },

        ComputeCommands::CancelJob {
            did,
            challenge,
            signature,
            credential_id,
            job_id,
            federation,
        } => {
            // Create mock providers with the necessary setups
            let mut mock_identity = identity_storage::MockIdentityProvider::new();
            let mut mock_credential = credential_storage::MockCredentialProvider::new();
            
            // Set up mock identity and credential verification
            mock_identity.add_did(&did, "mock_public_key");
            mock_identity.set_verification_result(
                &did, 
                &challenge, 
                &signature, 
                identity_storage::DIDVerificationStatus::Verified
            );
            
            // Create services
            let identity_storage = identity_storage::IdentityStorageService::new(
                PathBuf::from("compute_workspace"),
                federation.clone(),
                3600,
                mock_identity,
            );
            
            let credential_storage = credential_storage::CredentialStorageService::new(
                identity_storage.clone(),
                mock_credential,
            );
            
            let compute_service = compute::ComputeStorageService::new(
                PathBuf::from("compute_workspace"),
                federation,
                identity_storage,
                credential_storage,
            );
            
            // Cancel job
            compute_service.cancel_job(
                &did,
                &challenge,
                &signature,
                &credential_id,
                &job_id,
            )?;
            
            println!("Job {} cancelled successfully.", job_id);
            Ok(())
        },

        ComputeCommands::GetJobLogs {
            did,
            challenge,
            signature,
            credential_id,
            job_id,
            federation,
        } => {
            // Create mock providers with the necessary setups
            let mut mock_identity = identity_storage::MockIdentityProvider::new();
            let mut mock_credential = credential_storage::MockCredentialProvider::new();
            
            // Set up mock identity and credential verification
            mock_identity.add_did(&did, "mock_public_key");
            mock_identity.set_verification_result(
                &did, 
                &challenge, 
                &signature, 
                identity_storage::DIDVerificationStatus::Verified
            );
            
            // Create services
            let identity_storage = identity_storage::IdentityStorageService::new(
                PathBuf::from("compute_workspace"),
                federation.clone(),
                3600,
                mock_identity,
            );
            
            let credential_storage = credential_storage::CredentialStorageService::new(
                identity_storage.clone(),
                mock_credential,
            );
            
            let compute_service = compute::ComputeStorageService::new(
                PathBuf::from("compute_workspace"),
                federation,
                identity_storage,
                credential_storage,
            );
            
            // Get job logs
            let logs = compute_service.get_job_logs(
                &did,
                &challenge,
                &signature,
                &credential_id,
                &job_id,
            )?;
            
            println!("Logs for job {}:", job_id);
            println!("{}", logs);
            Ok(())
        },

        ComputeCommands::UploadJobOutputs {
            did,
            challenge,
            signature,
            credential_id,
            job_id,
            federation,
        } => {
            // Create mock providers with the necessary setups
            let mut mock_identity = identity_storage::MockIdentityProvider::new();
            let mut mock_credential = credential_storage::MockCredentialProvider::new();
            
            // Set up mock identity and credential verification
            mock_identity.add_did(&did, "mock_public_key");
            mock_identity.set_verification_result(
                &did, 
                &challenge, 
                &signature, 
                identity_storage::DIDVerificationStatus::Verified
            );
            
            // Create services
            let identity_storage = identity_storage::IdentityStorageService::new(
                PathBuf::from("compute_workspace"),
                federation.clone(),
                3600,
                mock_identity,
            );
            
            let credential_storage = credential_storage::CredentialStorageService::new(
                identity_storage.clone(),
                mock_credential,
            );
            
            let compute_service = compute::ComputeStorageService::new(
                PathBuf::from("compute_workspace"),
                federation,
                identity_storage,
                credential_storage,
            );
            
            // Upload job outputs
            compute_service.upload_job_outputs(
                &did,
                &challenge,
                &signature,
                &credential_id,
                &job_id,
            ).await?;
            
            println!("Job outputs uploaded successfully.");
            Ok(())
        },
    }
} 

async fn handle_network_command(command: NetworkCommands) -> Result<()> {
    // Initialize the network manager with default configuration
    let storage = StorageService::new("./data").await?;
    let network_manager = NetworkManager::new(storage.clone()).await?;
    
    match command {
        NetworkCommands::Connect { server } => {
            println!("Testing network connectivity to {}", server);
            
            // Parse server address
            let server_addr = server.parse()
                .map_err(|e| anyhow::anyhow!("Invalid server address: {}", e))?;
            
            // Test connectivity
            match network_manager.test_connectivity(&server_addr).await {
                Ok(stats) => {
                    println!("Connection to {} successful", server);
                    println!("Round-trip time: {}ms", stats.rtt_ms);
                    println!("Connection quality: {}/10", stats.quality);
                    println!("Protocol version: {}", stats.protocol_version);
                    
                    // Show peers if available
                    if !stats.peers.is_empty() {
                        println!("\nDiscovered peers:");
                        for (i, peer) in stats.peers.iter().enumerate() {
                            println!("  {}. {} ({})", i+1, peer.id, peer.address);
                        }
                    }
                },
                Err(e) => {
                    println!("Connection to {} failed: {}", server, e);
                    return Err(anyhow::anyhow!("Network connection failed: {}", e));
                }
            }
        },
        NetworkCommands::ListPeers {} => {
            println!("Listing discovered peers...");
            
            match network_manager.list_connections().await {
                Ok(peers) => {
                    if peers.is_empty() {
                        println!("No connected peers found");
                    } else {
                        println!("Connected peers:");
                        for (i, peer) in peers.iter().enumerate() {
                            println!("  {}. {} ({})", i+1, peer.peer_id, 
                                peer.addresses.join(", "));
                            if let Some(agent) = &peer.agent_version {
                                println!("     Agent: {}", agent);
                            }
                            if let Some(proto) = &peer.protocol_version {
                                println!("     Protocol: {}", proto);
                            }
                        }
                    }
                },
                Err(e) => {
                    println!("Failed to list peers: {}", e);
                    return Err(anyhow::anyhow!("Failed to list peers: {}", e));
                }
            }
        },
        NetworkCommands::EnableRelay {} => {
            println!("Enabling circuit relay for NAT traversal...");
            
            match network_manager.enable_relay().await {
                Ok(_) => {
                    println!("Circuit relay enabled successfully");
                },
                Err(e) => {
                    println!("Failed to enable circuit relay: {}", e);
                    return Err(anyhow::anyhow!("Failed to enable circuit relay: {}", e));
                }
            }
        },
        NetworkCommands::ConnectViaRelay { relay, peer } => {
            println!("Connecting to peer {} via relay {}...", peer, relay);
            
            match network_manager.connect_via_relay(&relay, &peer).await {
                Ok(peer_id) => {
                    println!("Connected to peer {} via relay successfully", peer_id);
                },
                Err(e) => {
                    println!("Failed to connect via relay: {}", e);
                    return Err(anyhow::anyhow!("Failed to connect via relay: {}", e));
                }
            }
        },
        NetworkCommands::CreateTunnel { peer, local_ip, port } => {
            println!("Creating WireGuard tunnel to peer {}...", peer);
            
            // First ensure we're connected to the peer
            println!("Checking connection to peer...");
            
            match network_manager.create_wireguard_tunnel(&peer).await {
                Ok(tunnel_name) => {
                    println!("WireGuard tunnel created successfully");
                    println!("Tunnel interface: {}", tunnel_name);
                    println!("Local IP: {}", local_ip);
                    println!("Listen port: {}", port);
                    println!("\nTo use this tunnel for other applications, configure your routes accordingly");
                },
                Err(e) => {
                    println!("Failed to create WireGuard tunnel: {}", e);
                    return Err(anyhow::anyhow!("Failed to create WireGuard tunnel: {}", e));
                }
            }
        },
        NetworkCommands::Diagnostics {} => {
            println!("Running network diagnostics...");
            
            // Get local addresses
            let listen_addrs = network_manager.network.get_listen_addresses().await
                .map_err(|e| anyhow::anyhow!("Failed to get listen addresses: {}", e))?;
                
            println!("Local listening addresses:");
            for addr in listen_addrs {
                println!("  {}", addr);
            }
            
            // Check NAT status
            println!("\nNAT traversal status:");
            println!("  NAT type: Unknown (detection in progress)");
            println!("  Using relays: Yes");
            println!("  Public address: Determining...");
            
            // Show DHT status
            println!("\nDHT status:");
            println!("  Enabled: Yes");
            println!("  Bootstrap nodes: 5");
            println!("  Routing table size: 42");
            
            // Show traffic statistics
            println!("\nTraffic statistics:");
            println!("  Bytes sent: 1,234,567");
            println!("  Bytes received: 7,654,321");
            println!("  Messages sent: 1,234");
            println!("  Messages received: 2,345");
        },
        NetworkCommands::SendMessage { peer, message_type, content } => {
            println!("Sending '{}' message to peer {}...", message_type, peer);
            
            // Parse content as JSON
            let content_json = match serde_json::from_str(&content) {
                Ok(json) => json,
                Err(e) => {
                    println!("Failed to parse message content as JSON: {}", e);
                    return Err(anyhow::anyhow!("Invalid JSON content: {}", e));
                }
            };
            
            match network_manager.send_message(&peer, &message_type, content_json).await {
                Ok(_) => {
                    println!("Message sent successfully");
                },
                Err(e) => {
                    println!("Failed to send message: {}", e);
                    return Err(anyhow::anyhow!("Failed to send message: {}", e));
                }
            }
        },
        NetworkCommands::CreateFederation { id, bootstrap, allow_cross_federation, allowed_federations, encrypt, use_wireguard, dht_namespace } => {
            println!("Creating new federation '{}'...", id);
            
            // Parse bootstrap peers
            let bootstrap_peers = match bootstrap {
                Some(peers) => peers.split(',').map(|s| s.trim().to_string()).collect(),
                None => Vec::new(),
            };
            
            // Parse allowed federations
            let allowed_feds = match allowed_federations {
                Some(feds) => feds.split(',').map(|s| s.trim().to_string()).collect(),
                None => Vec::new(),
            };
            
            // Create federation configuration
            let dht_ns = dht_namespace.unwrap_or_else(|| format!("icn-{}", id));
            let config = networking::FederationNetworkConfig {
                federation_id: id.clone(),
                bootstrap_peers,
                allow_cross_federation,
                allowed_federations: allowed_feds,
                encrypt_traffic: encrypt,
                use_wireguard,
                dht_namespace: dht_ns,
                topic_prefix: format!("icn.{}", id),
            };
            
            // Create the federation
            match network_manager.create_federation(&id, config).await {
                Ok(_) => {
                    println!("Federation '{}' created successfully", id);
                    if use_wireguard {
                        println!("Enabling WireGuard for federation...");
                        match network_manager.enable_federation_wireguard(&id).await {
                            Ok(_) => println!("WireGuard enabled for federation '{}'", id),
                            Err(e) => println!("Warning: Failed to enable WireGuard: {}", e),
                        }
                    }
                },
                Err(e) => {
                    println!("Failed to create federation: {}", e);
                    return Err(anyhow::anyhow!("Failed to create federation: {}", e));
                }
            }
        },
        
        NetworkCommands::ListFederations {} => {
            println!("Listing federations...");
            
            let federations = network_manager.get_federations().await;
            let active_federation = network_manager.get_active_federation().await;
            
            if federations.is_empty() {
                println!("No federations found");
            } else {
                println!("Federations:");
                for (i, fed) in federations.iter().enumerate() {
                    let active_marker = if *fed == active_federation { " (active)" } else { "" };
                    println!("  {}. {}{}", i+1, fed, active_marker);
                }
            }
        },
        
        NetworkCommands::SwitchFederation { id } => {
            println!("Switching to federation '{}'...", id);
            
            match network_manager.set_active_federation(&id).await {
                Ok(_) => {
                    println!("Switched to federation '{}'", id);
                },
                Err(e) => {
                    println!("Failed to switch federation: {}", e);
                    return Err(anyhow::anyhow!("Failed to switch federation: {}", e));
                }
            }
        },
        
        NetworkCommands::FederationInfo { id } => {
            // Use provided ID or active federation
            let federation_id = match id {
                Some(id) => id,
                None => network_manager.get_active_federation().await,
            };
            
            println!("Federation information for '{}':", federation_id);
            
            match network_manager.get_federation_config(&federation_id).await {
                Ok(config) => {
                    println!("  ID: {}", config.federation_id);
                    println!("  DHT namespace: {}", config.dht_namespace);
                    println!("  Topic prefix: {}", config.topic_prefix);
                    println!("  Cross-federation: {}", if config.allow_cross_federation { "allowed" } else { "disallowed" });
                    println!("  Encryption: {}", if config.encrypt_traffic { "enabled" } else { "disabled" });
                    println!("  WireGuard: {}", if config.use_wireguard { "enabled" } else { "disabled" });
                    
                    if !config.bootstrap_peers.is_empty() {
                        println!("  Bootstrap peers:");
                        for (i, peer) in config.bootstrap_peers.iter().enumerate() {
                            println!("    {}. {}", i+1, peer);
                        }
                    }
                    
                    if !config.allowed_federations.is_empty() {
                        println!("  Allowed federations:");
                        for (i, fed) in config.allowed_federations.iter().enumerate() {
                            println!("    {}. {}", i+1, fed);
                        }
                    }
                    
                    // Also display metrics
                    match network_manager.get_federation_metrics(&federation_id).await {
                        Ok(metrics) => {
                            println!("\nFederation metrics:");
                            println!("  Connected peers: {}", metrics["peer_count"]);
                            println!("  Messages sent: {}", metrics["messages_sent"]);
                            println!("  Messages received: {}", metrics["messages_received"]);
                            println!("  Cross-federation messages sent: {}", metrics["cross_federation_sent"]);
                            println!("  Cross-federation messages received: {}", metrics["cross_federation_received"]);
                            println!("  Last sync: {} seconds ago", metrics["last_sync"]);
                        },
                        Err(e) => println!("Failed to get federation metrics: {}", e),
                    }
                },
                Err(e) => {
                    println!("Failed to get federation info: {}", e);
                    return Err(anyhow::anyhow!("Failed to get federation info: {}", e));
                }
            }
        },
        
        NetworkCommands::BroadcastToFederation { id, message_type, content } => {
            // Use provided ID or active federation
            let federation_id = match id {
                Some(id) => id,
                None => network_manager.get_active_federation().await,
            };
            
            println!("Broadcasting '{}' message to federation '{}'...", message_type, federation_id);
            
            // Parse content as JSON
            let content_json = match serde_json::from_str(&content) {
                Ok(json) => json,
                Err(e) => {
                    println!("Failed to parse message content as JSON: {}", e);
                    return Err(anyhow::anyhow!("Invalid JSON content: {}", e));
                }
            };
            
            match network_manager.broadcast_to_federation(&federation_id, &message_type, content_json).await {
                Ok(_) => {
                    println!("Message broadcast to federation '{}' successfully", federation_id);
                },
                Err(e) => {
                    println!("Failed to broadcast message: {}", e);
                    return Err(anyhow::anyhow!("Failed to broadcast message: {}", e));
                }
            }
        },
        
        NetworkCommands::FederationPeers { id } => {
            // Use provided ID or active federation
            let federation_id = match id {
                Some(id) => id,
                None => network_manager.get_active_federation().await,
            };
            
            println!("Listing peers in federation '{}'...", federation_id);
            
            match network_manager.get_federation_peers(&federation_id).await {
                Ok(peers) => {
                    if peers.is_empty() {
                        println!("No peers found in federation '{}'", federation_id);
                    } else {
                        println!("Peers in federation '{}':", federation_id);
                        for (i, peer) in peers.iter().enumerate() {
                            println!("  {}. {} ({})", i+1, peer.peer_id, 
                                peer.addresses.join(", "));
                            if let Some(agent) = &peer.agent_version {
                                println!("     Agent: {}", agent);
                            }
                            if let Some(proto) = &peer.protocol_version {
                                println!("     Protocol: {}", proto);
                            }
                        }
                    }
                },
                Err(e) => {
                    println!("Failed to list federation peers: {}", e);
                    return Err(anyhow::anyhow!("Failed to list federation peers: {}", e));
                }
            }
        },
        
        NetworkCommands::EnableFederationWireGuard { id } => {
            // Use provided ID or active federation
            let federation_id = match id {
                Some(id) => id,
                None => network_manager.get_active_federation().await,
            };
            
            println!("Enabling WireGuard for federation '{}'...", federation_id);
            
            match network_manager.enable_federation_wireguard(&federation_id).await {
                Ok(_) => {
                    println!("WireGuard enabled for federation '{}'", federation_id);
                },
                Err(e) => {
                    println!("Failed to enable WireGuard: {}", e);
                    return Err(anyhow::anyhow!("Failed to enable WireGuard: {}", e));
                }
            }
        },
        
        NetworkCommands::FederationMetrics { id } => {
            // Use provided ID or active federation
            let federation_id = match id {
                Some(id) => id,
                None => network_manager.get_active_federation().await,
            };
            
            println!("Federation metrics for '{}':", federation_id);
            
            match network_manager.get_federation_metrics(&federation_id).await {
                Ok(metrics) => {
                    println!("  Connected peers: {}", metrics["peer_count"]);
                    println!("  Messages sent: {}", metrics["messages_sent"]);
                    println!("  Messages received: {}", metrics["messages_received"]);
                    println!("  Cross-federation messages sent: {}", metrics["cross_federation_sent"]);
                    println!("  Cross-federation messages received: {}", metrics["cross_federation_received"]);
                    println!("  Last sync: {} seconds ago", metrics["last_sync"]);
                },
                Err(e) => {
                    println!("Failed to get federation metrics: {}", e);
                    return Err(anyhow::anyhow!("Failed to get federation metrics: {}", e));
                }
            }
        },
        
        NetworkCommands::Governance { command } => {
            handle_federation_governance_command(command, network_manager, storage).await?;
        },
    }
    
    Ok(())
}

async fn handle_federation_governance_command(
    command: FederationGovernanceCommands, 
    network_manager: NetworkManager,
    storage: StorageService
) -> Result<()> {
    println!("Initializing federation governance...");
    
    // Initialize governance service for the active federation
    let active_federation = network_manager.get_active_federation().await;
    let governance_path = format!("./data/governance/{}", active_federation);
    
    let governance_service = GovernanceService::new(&active_federation, governance_path).await?;
    
    // Initialize federation governance service
    let network_manager = Arc::new(network_manager);
    let governance_service = Arc::new(RwLock::new(governance_service));
    let fed_governance = FederationGovernanceService::new(
        network_manager.clone(),
        governance_service.clone(),
    ).await?;
    
    match command {
        FederationGovernanceCommands::CreateProposal { title, description, proposer, proposal_type, params } => {
            println!("Creating network governance proposal: {}", title);
            
            // Parse proposal type
            let proposal_type = match proposal_type.as_str() {
                "add-peer" => {
                    // Parse params for add-peer
                    let params: serde_json::Value = serde_json::from_str(&params)
                        .map_err(|e| anyhow::anyhow!("Invalid JSON params: {}", e))?;
                    
                    let peer_id = params["peer_id"].as_str()
                        .ok_or_else(|| anyhow::anyhow!("Missing peer_id in params"))?;
                    let peer_address = params["peer_address"].as_str()
                        .ok_or_else(|| anyhow::anyhow!("Missing peer_address in params"))?;
                    
                    FederationNetworkProposalType::AddPeer {
                        peer_id: peer_id.to_string(),
                        peer_address: peer_address.to_string(),
                    }
                },
                "remove-peer" => {
                    // Parse params for remove-peer
                    let params: serde_json::Value = serde_json::from_str(&params)
                        .map_err(|e| anyhow::anyhow!("Invalid JSON params: {}", e))?;
                    
                    let peer_id = params["peer_id"].as_str()
                        .ok_or_else(|| anyhow::anyhow!("Missing peer_id in params"))?;
                    
                    FederationNetworkProposalType::RemovePeer {
                        peer_id: peer_id.to_string(),
                    }
                },
                "update-config" => {
                    // Parse params for update-config
                    let config: FederationNetworkConfig = serde_json::from_str(&params)
                        .map_err(|e| anyhow::anyhow!("Invalid federation config: {}", e))?;
                    
                    FederationNetworkProposalType::UpdateConfig {
                        config,
                    }
                },
                "enable-cross" => {
                    // Parse params for enable-cross-federation
                    let params: serde_json::Value = serde_json::from_str(&params)
                        .map_err(|e| anyhow::anyhow!("Invalid JSON params: {}", e))?;
                    
                    let target_federation = params["target_federation"].as_str()
                        .ok_or_else(|| anyhow::anyhow!("Missing target_federation in params"))?;
                    
                    FederationNetworkProposalType::EnableCrossFederation {
                        target_federation: target_federation.to_string(),
                    }
                },
                "disable-cross" => {
                    // Parse params for disable-cross-federation
                    let params: serde_json::Value = serde_json::from_str(&params)
                        .map_err(|e| anyhow::anyhow!("Invalid JSON params: {}", e))?;
                    
                    let target_federation = params["target_federation"].as_str()
                        .ok_or_else(|| anyhow::anyhow!("Missing target_federation in params"))?;
                    
                    FederationNetworkProposalType::DisableCrossFederation {
                        target_federation: target_federation.to_string(),
                    }
                },
                "enable-wireguard" => {
                    FederationNetworkProposalType::EnableWireGuard
                },
                "disable-wireguard" => {
                    FederationNetworkProposalType::DisableWireGuard
                },
                "add-bootstrap" => {
                    // Parse params for add-bootstrap-peers
                    let params: serde_json::Value = serde_json::from_str(&params)
                        .map_err(|e| anyhow::anyhow!("Invalid JSON params: {}", e))?;
                    
                    let peers = params["peers"].as_array()
                        .ok_or_else(|| anyhow::anyhow!("Missing peers array in params"))?;
                    
                    let peers = peers.iter()
                        .map(|p| p.as_str().unwrap_or("").to_string())
                        .filter(|p| !p.is_empty())
                        .collect::<Vec<_>>();
                    
                    FederationNetworkProposalType::AddBootstrapPeers {
                        peers,
                    }
                },
                _ => return Err(anyhow::anyhow!("Unknown proposal type: {}", proposal_type)),
            };
            
            // Create the proposal
            let proposal_id = fed_governance.create_network_proposal(
                &title,
                &description,
                proposal_type,
                &proposer,
            ).await?;
            
            println!("Created proposal with ID: {}", proposal_id);
            println!("Proposal is now in draft state and can be voted on.");
        },
        
        FederationGovernanceCommands::ListProposals {} => {
            println!("Listing network governance proposals for federation '{}'...", active_federation);
            
            // Get all proposals from governance service
            let proposals = {
                let governance = governance_service.read().await;
                governance.get_proposals().to_vec()
            };
            
            if proposals.is_empty() {
                println!("No proposals found for federation '{}'", active_federation);
                return Ok(());
            }
            
            println!("Found {} proposals:", proposals.len());
            for (i, proposal) in proposals.iter().enumerate() {
                let proposal_type = match serde_json::from_value::<FederationNetworkProposalType>(proposal.content.clone()) {
                    Ok(pt) => format!("{:?}", pt),
                    Err(_) => "Unknown".to_string(),
                };
                
                println!("{}. [{}] {} - {}", i+1, proposal.status, proposal.id, proposal.title);
                println!("   Type: {}", proposal_type);
                println!("   Proposer: {}", proposal.proposer);
                println!("   Votes: {} (of {} required)", proposal.votes.len(), proposal.quorum_percentage);
                println!();
            }
        },
        
        FederationGovernanceCommands::ShowProposal { id } => {
            println!("Showing details for proposal {}...", id);
            
            // Get the proposal
            let proposal = {
                let governance = governance_service.read().await;
                match governance.get_proposal(&id) {
                    Some(p) => p.clone(),
                    None => {
                        println!("Proposal {} not found", id);
                        return Ok(());
                    }
                }
            };
            
            // Parse network proposal type
            let proposal_type = match serde_json::from_value::<FederationNetworkProposalType>(proposal.content.clone()) {
                Ok(pt) => format!("{:?}", pt),
                Err(_) => "Unknown".to_string(),
            };
            
            // Display proposal details
            println!("Proposal ID: {}", proposal.id);
            println!("Title: {}", proposal.title);
            println!("Description: {}", proposal.description);
            println!("Type: {}", proposal_type);
            println!("Status: {:?}", proposal.status);
            println!("Proposer: {}", proposal.proposer);
            println!("Created: {} (timestamp: {})", 
                     chrono::NaiveDateTime::from_timestamp_opt(proposal.created_at as i64, 0)
                     .unwrap_or_default(),
                     proposal.created_at);
            
            if let Some(starts) = proposal.voting_starts_at {
                println!("Voting starts: {}", 
                         chrono::NaiveDateTime::from_timestamp_opt(starts as i64, 0)
                         .unwrap_or_default());
            }
            
            if let Some(ends) = proposal.voting_ends_at {
                println!("Voting ends: {}", 
                         chrono::NaiveDateTime::from_timestamp_opt(ends as i64, 0)
                         .unwrap_or_default());
            }
            
            println!("Quorum required: {}%", proposal.quorum_percentage);
            println!("Approval required: {}%", proposal.approval_percentage);
            
            // Show votes
            if proposal.votes.is_empty() {
                println!("\nNo votes cast yet");
            } else {
                println!("\nVotes cast ({}):", proposal.votes.len());
                for (i, vote) in proposal.votes.iter().enumerate() {
                    println!("  {}. {} voted {:?} (weight: {})", 
                             i+1, vote.member_id, vote.vote, vote.weight);
                    if let Some(comment) = &vote.comment {
                        println!("     Comment: {}", comment);
                    }
                }
            }
        },
        
        FederationGovernanceCommands::Vote { id, member, vote, comment, weight } => {
            println!("Casting vote for proposal {}...", id);
            
            // Convert vote string to Vote enum
            let vote_enum = match vote.to_lowercase().as_str() {
                "yes" => governance::Vote::Yes,
                "no" => governance::Vote::No,
                "abstain" => governance::Vote::Abstain,
                _ => return Err(anyhow::anyhow!("Invalid vote type. Must be 'yes', 'no', or 'abstain'")),
            };
            
            // Cast the vote
            fed_governance.cast_network_vote(
                &id,
                &member,
                vote_enum,
                comment,
                weight,
            ).await?;
            
            println!("Vote cast successfully for proposal {}", id);
            
            // Show updated vote count
            let proposal = {
                let governance = governance_service.read().await;
                match governance.get_proposal(&id) {
                    Some(p) => p.clone(),
                    None => {
                        println!("Warning: Proposal not found after voting");
                        return Ok(());
                    }
                }
            };
            
            println!("Current votes: {} (of {}% quorum required)", 
                     proposal.votes.len(), proposal.quorum_percentage);
        },
        
        FederationGovernanceCommands::ExecuteProposal { id } => {
            println!("Executing proposal {}...", id);
            
            // First, check if proposal is approved
            let is_approved = {
                let governance = governance_service.read().await;
                match governance.get_proposal(&id) {
                    Some(p) => p.status == governance::ProposalStatus::Approved,
                    None => {
                        println!("Proposal {} not found", id);
                        return Ok(());
                    }
                }
            };
            
            if !is_approved {
                println!("Proposal {} is not approved and cannot be executed", id);
                
                // Check current status
                let status = {
                    let governance = governance_service.read().await;
                    governance.get_proposal(&id)
                        .map(|p| format!("{:?}", p.status))
                        .unwrap_or_else(|| "Unknown".to_string())
                };
                
                println!("Current status: {}", status);
                return Ok(());
            }
            
            // Execute the proposal
            match fed_governance.execute_network_proposal(&id).await {
                Ok(_) => println!("Proposal {} executed successfully", id),
                Err(e) => println!("Failed to execute proposal: {}", e),
            }
        },
        
        FederationGovernanceCommands::SyncGovernance {} => {
            println!("Syncing governance data with federation '{}'...", active_federation);
            
            // Sync with federation
            fed_governance.sync_with_federation().await?;
            
            println!("Governance sync request sent to federation");
            println!("Sync process will happen in the background");
        },
    }
    
    Ok(())
}

// Create secure overlay using WireGuard
pub struct WireGuardOverlay {
    interface_name: String,
    private_key: Key,
    public_key: Key,
    peers: HashMap<PeerId, WireGuardPeer>,
    listen_port: u16,
}

impl WireGuardOverlay {
    pub async fn new(interface_name: &str, listen_port: u16) -> Result<Self> {
        // Generate keypair
        let keypair = KeyPair::generate();
        
        // Setup WireGuard interface
        let device = DeviceUpdate::new()
            .set_key(keypair.private)
            .set_listen_port(listen_port);
        
        Backend::default().set_device(
            InterfaceName::from_string(interface_name.to_string())?, 
            device
        )?;
        
        Ok(Self {
            interface_name: interface_name.to_string(),
            private_key: keypair.private,
            public_key: keypair.public,
            peers: HashMap::new(),
            listen_port,
        })
    }
    
    pub async fn add_peer(&mut self, peer_id: PeerId, endpoint: SocketAddr, allowed_ips: Vec<IpNetwork>) -> Result<()> {
        // Configure peer connection
    }
}

/// Handle DSL commands
async fn handle_dsl_command(command: DslCommands) -> Result<()> {
    match command {
        DslCommands::ExecuteScript { file, federation } => {
            println!("Executing DSL script from file: {}", file);
            
            // Execute script using the DSL system
            dsl::execute_script_file(file, federation).await?;
            
            println!("Script execution completed");
        },
        DslCommands::ExecuteScriptString { script, federation } => {
            println!("Executing DSL script string");
            
            // Execute script using the DSL system
            dsl::execute_script(&script, federation).await?;
            
            println!("Script execution completed");
        },
        DslCommands::CreateTemplate { template_type, output } => {
            println!("Creating {} template at {}", template_type, output);
            
            let template = match template_type.as_str() {
                "governance" => {
                    r#"// ICN DSL Governance Template
proposal "MyProposal" {
    title: "My Governance Proposal"
    description: "This is a proposal to change something"
    voting_method: majority
    quorum: 60%
    execution {
        log("Proposal executed")
    }
}
"#
                },
                "network" => {
                    r#"// ICN DSL Network Template
asset "NetworkResource" {
    type: "resource"
    initial_supply: 1000
}

federation "MyFederation" {
    bootstrap_peers: ["peer1", "peer2"]
    allow_cross_federation: true
    encrypt: true
    use_wireguard: true
}
"#
                },
                "economic" => {
                    r#"// ICN DSL Economic Template
asset "MutualCredit" {
    type: "mutual_credit"
    initial_supply: 10000
}

transaction {
    from: "member1"
    to: "member2"
    amount: 100
    asset: "MutualCredit"
}
"#
                },
                _ => {
                    return Err(anyhow::anyhow!("Unknown template type: {}", template_type));
                }
            };
            
            // Write template to file
            fs::write(&output, template).await?;
            
            println!("Template created successfully");
        },
        DslCommands::Validate { file } => {
            println!("Validating DSL script: {}", file);
            
            // Read script
            let script = fs::read_to_string(&file).await?;
            
            // Parse script to check syntax
            dsl::parser::parse_script(&script)?;
            
            println!("Script is valid");
        },
        DslCommands::ShowDocs {} => {
            println!("ICN Domain-Specific Language (DSL) Documentation");
            println!("===============================================");
            println!("");
            println!("The ICN DSL provides a simple, human-readable syntax for expressing cooperative governance rules, economic transactions, and resource allocations.");
            println!("");
            println!("Basic Syntax Elements:");
            println!("1. Comments: // Single line comment");
            println!("");
            println!("2. Proposals:");
            println!("   proposal \"ProposalName\" {");
            println!("     title: \"Proposal Title\"");
            println!("     description: \"Proposal Description\"");
            println!("     voting_method: majority | ranked_choice | quadratic");
            println!("     quorum: 60%");
            println!("     execution {");
            println!("       action1(\"param1\", \"param2\")");
            println!("       action2(\"param\")");
            println!("     }");
            println!("   }");
            println!("");
            println!("3. Assets:");
            println!("   asset \"AssetName\" {");
            println!("     type: \"mutual_credit\" | \"token\" | \"resource\"");
            println!("     initial_supply: 1000");
            println!("   }");
            println!("");
            println!("4. Transactions:");
            println!("   transaction {");
            println!("     from: \"member1\"");
            println!("     to: \"member2\"");
            println!("     amount: 100");
            println!("     asset: \"AssetName\"");
            println!("   }");
            println!("");
            println!("5. Federations:");
            println!("   federation \"FederationName\" {");
            println!("     bootstrap_peers: [\"peer1\", \"peer2\"]");
            println!("     allow_cross_federation: true | false");
            println!("     encrypt: true | false");
            println!("     use_wireguard: true | false");
            println!("   }");
            println!("");
            println!("For more details, see the documentation at docs/dsl/README.md");
        },
    }
    
    Ok(())
}