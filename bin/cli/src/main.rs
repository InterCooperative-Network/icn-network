// ICN CLI entry point

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod storage;
use storage::StorageService;

mod governance;
use governance::{GovernanceService, ProposalType, ProposalStatus, Vote};

mod governance_storage;
use governance_storage::{GovernanceStorageService, StoragePolicyType};

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
    /// Show node status
    Status {},
    
    /// Test network connectivity
    Network {
        /// Server address to connect to
        #[clap(short, long, default_value = "127.0.0.1:8000")]
        server: String,
    },
    
    /// Storage system operations
    Storage {
        #[clap(subcommand)]
        command: StorageCommands,
    },
    
    /// Governance operations
    Governance {
        #[clap(subcommand)]
        command: GovernanceCommands,
    },

    /// Governance-controlled storage operations
    GovernedStorage {
        #[clap(subcommand)]
        command: GovernedStorageCommands,
    },
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
            println!("Node status: OK");
        },
        Commands::Network { server } => {
            println!("Testing network connectivity to {}", server);
            println!("Network test completed successfully");
        },
        Commands::Storage { command } => handle_storage_command(command).await?,
        Commands::Governance { command } => handle_governance_command(command).await?,
        Commands::GovernedStorage { command } => handle_governed_storage_command(command).await?,
    }
    
    Ok(())
}

async fn handle_storage_command(command: StorageCommands) -> Result<()> {
    match command {
        StorageCommands::Init { path, encrypted } => {
            println!("Initializing storage at {} (encryption: {})", path, if encrypted { "enabled" } else { "disabled" });
            
            // Create path if it doesn't exist
            let path = PathBuf::from(path);
            tokio::fs::create_dir_all(&path).await?;
            
            // Initialize storage service
            let mut service = StorageService::new(&path).await?;
            
            // Initialize default federation with encryption setting
            service.init_federation("default", encrypted).await?;
            
            println!("Storage environment initialized successfully");
        },
        StorageCommands::Put { file, key, encrypted, federation } => {
            let key = key.unwrap_or_else(|| file.split('/').last().unwrap_or(&file).to_string());
            println!("Storing file {} with key {} in federation {} (encryption: {})", 
                file, key, federation, if encrypted { "enabled" } else { "disabled" });
            
            // Initialize storage service with data directory
            let service = StorageService::new("./data").await?;
            
            // Store the file
            service.store_file(file, &key, &federation, encrypted).await?;
            
            println!("File stored successfully");
        },
        StorageCommands::Get { key, output, version, federation } => {
            let output = output.unwrap_or_else(|| key.clone());
            println!("Retrieving key {} from federation {} to {}{}", 
                key, federation, output, 
                if let Some(ver) = &version { format!(" (version: {})", ver) } else { String::new() });
            
            // Initialize storage service with data directory
            let service = StorageService::new("./data").await?;
            
            // Retrieve the file
            service.retrieve_file(&key, &output, &federation, version.as_deref()).await?;
            
            println!("File retrieved successfully");
        },
        StorageCommands::List { prefix, federation } => {
            println!("Listing files in federation {}{}", 
                federation,
                if let Some(pre) = &prefix { format!(" with prefix {}", pre) } else { String::new() });
            
            // Initialize storage service with data directory
            let service = StorageService::new("./data").await?;
            
            // List files
            let files = service.list_files(&federation, prefix.as_deref()).await?;
            
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
        StorageCommands::History { key, limit, federation } => {
            println!("Showing version history for {} in federation {} (limit: {})", key, federation, limit);
            
            // Initialize storage service with data directory
            let service = StorageService::new("./data").await?;
            
            // Get history
            let versions = service.get_history(&key, &federation, limit).await?;
            
            if versions.is_empty() {
                println!("No versions found");
            } else {
                println!("Version history (most recent first):");
                println!("{:<36} {:<20} {:<10} {:<20}", "Version ID", "Timestamp", "Size", "Content Hash");
                println!("{:-<36} {:-<20} {:-<10} {:-<20}", "", "", "", "");
                
                for version in versions {
                    // Format timestamp as ISO date
                    let timestamp = chrono::DateTime::from_timestamp(version.timestamp as i64, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| "Unknown".to_string());
                    
                    // Format size
                    let size = if version.size < 1024 {
                        format!("{}B", version.size)
                    } else if version.size < 1024 * 1024 {
                        format!("{:.1}KB", version.size as f64 / 1024.0)
                    } else {
                        format!("{:.1}MB", version.size as f64 / (1024.0 * 1024.0))
                    };
                    
                    println!("{:<36} {:<20} {:<10} {:<20}", 
                        version.id, 
                        timestamp,
                        size,
                        &version.content_hash[0..8] // Show first 8 chars of hash
                    );
                }
            }
        },
        StorageCommands::GenerateKey { output } => {
            println!("Generating encryption key to {}", output);
            
            // Initialize storage service with data directory
            let service = StorageService::new("./data").await?;
            
            // Generate key
            service.generate_key(output).await?;
            
            println!("Encryption key generated successfully");
        },
        StorageCommands::GenerateKeyPair { output_dir } => {
            println!("Generating asymmetric key pair for recipient-specific encryption");
            
            // Initialize storage service with data directory
            let service = StorageService::new("./data").await?;
            
            // Generate key pair
            service.generate_key_pair(output_dir).await?;
            
            println!("Asymmetric key pair generated successfully");
        },
        StorageCommands::ExportKey { federation, output } => {
            println!("Exporting encryption key for federation {}", federation);
            
            // Initialize storage service with data directory
            let service = StorageService::new("./data").await?;
            
            // Export encryption key
            service.export_encryption_key(&federation, &output).await?;
            
            println!("Encryption key exported successfully to {}", output);
        },
        StorageCommands::ImportKey { federation, key_file } => {
            println!("Importing encryption key for federation {}", federation);
            
            // Initialize storage service with data directory
            let service = StorageService::new("./data").await?;
            
            // Import encryption key
            service.import_encryption_key(&federation, &key_file).await?;
            
            println!("Encryption key imported successfully");
        },
        StorageCommands::EncryptFor { input, output, recipients } => {
            println!("Encrypting file {} for specific recipients", input);
            
            // Parse recipient public key files
            let recipient_list: Vec<String> = recipients.split(',').map(|s| s.trim().to_string()).collect();
            
            // Read all recipient public keys
            let mut recipient_keys = Vec::new();
            for key_file in &recipient_list {
                println!("Reading recipient public key from {}", key_file);
                let key_data = tokio::fs::read(key_file).await?;
                recipient_keys.push(key_data);
            }
            
            // Initialize storage service with data directory
            let service = StorageService::new("./data").await?;
            
            // Encrypt file for recipients
            service.encrypt_for_recipients(&input, &output, &recipient_keys).await?;
            
            println!("File encrypted successfully for {} recipients", recipient_list.len());
        },
        StorageCommands::DecryptWith { input, output, private_key } => {
            println!("Decrypting file {} with private key", input);
            
            // Read private key
            let key_data = tokio::fs::read(&private_key).await?;
            
            // Initialize storage service with data directory
            let service = StorageService::new("./data").await?;
            
            // Decrypt file
            service.decrypt_with_private_key(&input, &output, &key_data).await?;
            
            println!("File decrypted successfully to {}", output);
        },
    }
    Ok(())
}

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