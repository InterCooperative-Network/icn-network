//! Governance-controlled storage system for ICN
//!
//! This module integrates the governance and storage systems, allowing
//! federation-wide policies for storage management and access control.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, info, warn};

use crate::governance::{GovernanceService, ProposalType, Vote};
use crate::storage::{StorageService, FederationConfig, VersionedFileMetadata};

/// Storage policy types that can be governed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StoragePolicyType {
    /// Total storage quota for the federation
    FederationQuota,
    /// Per-member storage quotas
    MemberQuota,
    /// List of allowed encryption algorithms
    EncryptionAlgorithms,
    /// Access control policy (who can access what)
    AccessControl,
    /// Data retention policy
    RetentionPolicy,
    /// Backup and replication policy
    ReplicationPolicy,
}

/// Storage policy defined by governance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePolicy {
    /// Policy ID
    pub id: String,
    /// Policy type
    pub policy_type: StoragePolicyType,
    /// Policy content (JSON)
    pub content: serde_json::Value,
    /// Federation this policy applies to
    pub federation: String,
    /// Creation timestamp
    pub created_at: u64,
    /// Last updated timestamp
    pub updated_at: u64,
    /// Whether the policy is active
    pub active: bool,
}

/// Access permissions for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPermission {
    /// Member ID
    pub member_id: String,
    /// Path or key pattern this permission applies to
    pub path_pattern: String,
    /// Whether read access is granted
    pub can_read: bool,
    /// Whether write access is granted
    pub can_write: bool,
    /// Whether the member can grant access to others
    pub can_grant: bool,
}

/// Storage quota definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageQuota {
    /// Member ID (or "federation" for federation-wide quota)
    pub target_id: String,
    /// Maximum storage in bytes
    pub max_bytes: u64,
    /// Maximum number of files
    pub max_files: Option<u64>,
    /// Maximum file size in bytes
    pub max_file_size: Option<u64>,
}

/// Retention policy for data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Path pattern this policy applies to
    pub path_pattern: String,
    /// Maximum retention period in seconds (None = forever)
    pub max_age_seconds: Option<u64>,
    /// Minimum number of versions to keep
    pub min_versions: Option<u32>,
    /// Maximum number of versions to keep
    pub max_versions: Option<u32>,
}

/// Integrated service for governance-controlled storage
pub struct GovernanceStorageService {
    /// Path to data directory
    data_path: PathBuf,
    /// Federation name
    federation: String,
    /// Storage service
    storage_service: StorageService,
    /// Governance service
    governance_service: GovernanceService,
    /// Active storage policies
    policies: Vec<StoragePolicy>,
}

impl GovernanceStorageService {
    /// Create a new governance storage service
    pub async fn new(federation: &str, data_path: impl Into<PathBuf>) -> Result<Self> {
        let data_path = data_path.into();
        let policy_path = data_path.join("policies").join(federation);
        
        // Create policy directory if it doesn't exist
        fs::create_dir_all(&policy_path).await?;
        
        // Initialize storage service
        let storage_service = StorageService::new(&data_path).await?;
        
        // Initialize governance service
        let governance_service = GovernanceService::new(federation, &data_path).await?;
        
        // Load existing policies
        let policies = Self::load_policies(&policy_path).await?;
        
        Ok(Self {
            data_path,
            federation: federation.to_string(),
            storage_service,
            governance_service,
            policies,
        })
    }
    
    /// Get active policies
    pub fn get_policies(&self) -> &[StoragePolicy] {
        &self.policies
    }
    
    /// Load policies from disk
    async fn load_policies(policy_path: &PathBuf) -> Result<Vec<StoragePolicy>> {
        let mut policies = Vec::new();
        
        if !policy_path.exists() {
            return Ok(policies);
        }
        
        let mut entries = fs::read_dir(&policy_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_file() {
                let file_path = entry.path();
                if let Some(ext) = file_path.extension() {
                    if ext == "json" {
                        let data = fs::read(&file_path).await?;
                        let policy: StoragePolicy = serde_json::from_slice(&data)?;
                        if policy.active {
                            policies.push(policy);
                        }
                    }
                }
            }
        }
        
        Ok(policies)
    }
    
    /// Create a storage policy proposal
    pub async fn propose_storage_policy(
        &mut self,
        proposer: &str,
        title: &str,
        description: &str,
        policy_type: StoragePolicyType,
        policy_content: serde_json::Value,
    ) -> Result<String> {
        // Create proposal content
        let content = serde_json::json!({
            "policy_type": policy_type,
            "policy_content": policy_content,
        });
        
        // Create governance proposal
        let proposal_id = self.governance_service.create_proposal(
            title,
            description,
            ProposalType::ResourceAllocation,
            proposer,
            content,
            51, // Default quorum percentage
            51, // Default approval percentage
        ).await?;
        
        info!("Created storage policy proposal: {}", proposal_id);
        
        Ok(proposal_id)
    }
    
    /// Check if a member has permission to access a file
    pub fn check_permission(&self, member_id: &str, key: &str, write_access: bool) -> bool {
        // Find access control policies
        let access_policies = self.policies.iter()
            .filter(|p| matches!(p.policy_type, StoragePolicyType::AccessControl) && p.active);
        
        for policy in access_policies {
            if let Ok(permissions) = serde_json::from_value::<Vec<AccessPermission>>(policy.content.clone()) {
                for permission in permissions {
                    if permission.member_id == member_id {
                        // Check if the path pattern matches
                        if self.pattern_matches(&permission.path_pattern, key) {
                            // Check the required permission
                            if write_access {
                                return permission.can_write;
                            } else {
                                return permission.can_read;
                            }
                        }
                    }
                }
            }
        }
        
        // Default: no access
        false
    }
    
    /// Check if a pattern matches a key
    fn pattern_matches(&self, pattern: &str, key: &str) -> bool {
        // Simple wildcard matching
        if pattern == "*" {
            return true;
        }
        
        // Exact match
        if pattern == key {
            return true;
        }
        
        // Prefix match with wildcard
        if pattern.ends_with('*') {
            let prefix = &pattern[0..pattern.len()-1];
            return key.starts_with(prefix);
        }
        
        false
    }
    
    /// Check if a member has exceeded their storage quota
    pub async fn check_quota(&self, member_id: &str) -> Result<(bool, Option<StorageQuota>)> {
        // Find quota policies
        let quota_policies = self.policies.iter()
            .filter(|p| matches!(p.policy_type, StoragePolicyType::MemberQuota));
        
        for policy in quota_policies {
            if let Ok(quotas) = serde_json::from_value::<Vec<StorageQuota>>(policy.content.clone()) {
                for quota in quotas {
                    if quota.target_id == member_id {
                        // Calculate current usage
                        let usage = self.calculate_member_usage(member_id).await?;
                        
                        // Check if usage exceeds quota
                        let exceeded = usage >= quota.max_bytes;
                        
                        return Ok((exceeded, Some(quota)));
                    }
                }
            }
        }
        
        // No specific quota found for this member, check federation quota
        let federation_policies = self.policies.iter()
            .filter(|p| matches!(p.policy_type, StoragePolicyType::FederationQuota));
        
        for policy in federation_policies {
            if let Ok(quota) = serde_json::from_value::<StorageQuota>(policy.content.clone()) {
                if quota.target_id == "federation" {
                    // Calculate total federation usage
                    let usage = self.calculate_federation_usage().await?;
                    
                    // Check if usage exceeds quota
                    let exceeded = usage >= quota.max_bytes;
                    
                    return Ok((exceeded, Some(quota)));
                }
            }
        }
        
        // No quota found
        Ok((false, None))
    }
    
    /// Calculate storage usage for a member
    async fn calculate_member_usage(&self, member_id: &str) -> Result<u64> {
        // This would require member-file associations
        // For now, return a placeholder value
        Ok(1024 * 1024) // 1 MB
    }
    
    /// Calculate total storage usage for the federation
    async fn calculate_federation_usage(&self) -> Result<u64> {
        // Get all files in the federation
        let files = self.storage_service.list_files(&self.federation, None).await?;
        
        // Sum up the sizes
        let mut total_bytes = 0;
        for file in files {
            if let Some(version) = file.versions.last() {
                total_bytes += version.size as u64;
            }
        }
        
        Ok(total_bytes)
    }
    
    /// Apply a policy once it's been approved
    pub async fn apply_approved_policy(&mut self, proposal_id: &str) -> Result<()> {
        // Get the proposal
        let proposal = self.governance_service.get_proposal(proposal_id)
            .ok_or_else(|| anyhow!("Proposal not found"))?;
        
        // Check if the proposal is approved
        if !matches!(proposal.status, crate::governance::ProposalStatus::Approved) {
            return Err(anyhow!("Proposal is not approved"));
        }
        
        // Check if it's a storage policy proposal
        let policy_type: StoragePolicyType = serde_json::from_value(
            proposal.content.get("policy_type")
                .ok_or_else(|| anyhow!("Missing policy_type"))?
                .clone()
        )?;
        
        let policy_content = proposal.content.get("policy_content")
            .ok_or_else(|| anyhow!("Missing policy_content"))?
            .clone();
        
        // Create the policy
        let policy = StoragePolicy {
            id: uuid::Uuid::new_v4().to_string(),
            policy_type: policy_type.clone(),
            content: policy_content,
            federation: self.federation.clone(),
            created_at: chrono::Utc::now().timestamp() as u64,
            updated_at: chrono::Utc::now().timestamp() as u64,
            active: true,
        };
        
        // Save the policy
        self.save_policy(&policy).await?;
        
        // Add to in-memory policies
        self.policies.push(policy);
        
        info!("Applied approved storage policy: {}", proposal_id);
        
        Ok(())
    }
    
    /// Save a policy to disk
    async fn save_policy(&self, policy: &StoragePolicy) -> Result<()> {
        let policy_path = self.data_path
            .join("policies")
            .join(&self.federation)
            .join(format!("{}.json", policy.id));
        
        // Create parent directories if they don't exist
        if let Some(parent) = policy_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        // Serialize and save
        let data = serde_json::to_vec_pretty(policy)?;
        fs::write(&policy_path, &data).await?;
        
        Ok(())
    }
    
    /// Store a file with governance checks
    pub async fn store_file(
        &self,
        member_id: &str,
        file_path: impl AsRef<std::path::Path>,
        key: &str,
        encrypted: bool,
    ) -> Result<()> {
        // Check if member has permission to write
        if !self.check_permission(member_id, key, true) {
            return Err(anyhow!("Member does not have write permission for this key"));
        }
        
        // Check if member has exceeded their quota
        let (quota_exceeded, quota) = self.check_quota(member_id).await?;
        if quota_exceeded {
            return Err(anyhow!("Storage quota exceeded"));
        }
        
        // Store the file
        self.storage_service.store_file(file_path, key, &self.federation, encrypted).await?;
        
        Ok(())
    }
    
    /// Retrieve a file with governance checks
    pub async fn retrieve_file(
        &self,
        member_id: &str,
        key: &str,
        output_path: impl AsRef<std::path::Path>,
        version: Option<&str>,
    ) -> Result<()> {
        // Check if member has permission to read
        if !self.check_permission(member_id, key, false) {
            return Err(anyhow!("Member does not have read permission for this key"));
        }
        
        // Retrieve the file
        self.storage_service.retrieve_file(key, output_path, &self.federation, version).await?;
        
        Ok(())
    }
    
    /// List files with governance checks
    pub async fn list_files(&self, member_id: &str, prefix: Option<&str>) -> Result<Vec<VersionedFileMetadata>> {
        // Get all files
        let all_files = self.storage_service.list_files(&self.federation, prefix).await?;
        
        // Filter by access permission
        let accessible_files = all_files.into_iter()
            .filter(|file| self.check_permission(member_id, &file.filename, false))
            .collect();
        
        Ok(accessible_files)
    }
}

/// JSON schema for storage policy content
pub mod schema {
    use super::*;
    
    /// Schema for federation quota policy
    pub fn federation_quota_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["target_id", "max_bytes"],
            "properties": {
                "target_id": {
                    "type": "string",
                    "const": "federation",
                    "description": "Must be 'federation' for federation-wide quota"
                },
                "max_bytes": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Maximum storage in bytes"
                },
                "max_files": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional maximum number of files"
                },
                "max_file_size": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Optional maximum file size in bytes"
                }
            }
        })
    }
    
    /// Schema for member quota policy
    pub fn member_quota_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "array",
            "items": {
                "type": "object",
                "required": ["target_id", "max_bytes"],
                "properties": {
                    "target_id": {
                        "type": "string",
                        "description": "Member ID this quota applies to"
                    },
                    "max_bytes": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Maximum storage in bytes"
                    },
                    "max_files": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional maximum number of files"
                    },
                    "max_file_size": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Optional maximum file size in bytes"
                    }
                }
            }
        })
    }
    
    /// Schema for access control policy
    pub fn access_control_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "array",
            "items": {
                "type": "object",
                "required": ["member_id", "path_pattern", "can_read", "can_write"],
                "properties": {
                    "member_id": {
                        "type": "string",
                        "description": "Member ID this permission applies to"
                    },
                    "path_pattern": {
                        "type": "string",
                        "description": "Path pattern (can include * wildcard)"
                    },
                    "can_read": {
                        "type": "boolean",
                        "description": "Whether read access is granted"
                    },
                    "can_write": {
                        "type": "boolean",
                        "description": "Whether write access is granted"
                    },
                    "can_grant": {
                        "type": "boolean",
                        "description": "Whether the member can grant access to others"
                    }
                }
            }
        })
    }
    
    /// Schema for retention policy
    pub fn retention_policy_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "array",
            "items": {
                "type": "object",
                "required": ["path_pattern"],
                "properties": {
                    "path_pattern": {
                        "type": "string",
                        "description": "Path pattern this policy applies to"
                    },
                    "max_age_seconds": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Maximum retention period in seconds"
                    },
                    "min_versions": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Minimum number of versions to keep"
                    },
                    "max_versions": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Maximum number of versions to keep"
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_pattern_matching() -> Result<()> {
        let temp_dir = tempdir()?;
        let service = GovernanceStorageService::new("test", temp_dir.path()).await?;
        
        // Test exact match
        assert!(service.pattern_matches("test.txt", "test.txt"));
        
        // Test wildcard match
        assert!(service.pattern_matches("*", "anything.txt"));
        
        // Test prefix match
        assert!(service.pattern_matches("documents/*", "documents/report.pdf"));
        assert!(!service.pattern_matches("documents/*", "images/logo.png"));
        
        Ok(())
    }
} 