use std::collections::HashSet;
use serde::{Deserialize, Serialize};

/// Access policy for stored data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataAccessPolicy {
    /// Federations with read access
    pub read_federations: HashSet<String>,
    /// Federations with write access
    pub write_federations: HashSet<String>,
    /// Federations with admin access
    pub admin_federations: HashSet<String>,
    /// Whether encryption is required
    pub encryption_required: bool,
    /// Number of replicas to maintain
    pub redundancy_factor: u8,
    /// Optional expiration time (Unix timestamp)
    pub expiration_time: Option<u64>,
    /// Whether versioning is enabled
    pub versioning_enabled: bool,
    /// Maximum number of versions to keep
    pub max_versions: u32,
}

impl Default for DataAccessPolicy {
    fn default() -> Self {
        Self {
            read_federations: HashSet::new(),
            write_federations: HashSet::new(),
            admin_federations: HashSet::new(),
            encryption_required: true,
            redundancy_factor: 3,
            expiration_time: None,
            versioning_enabled: false,
            max_versions: 10,
        }
    }
}

impl DataAccessPolicy {
    /// Create a new policy with default values
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create a new policy with specified federations
    pub fn with_federations(
        read_federations: HashSet<String>,
        write_federations: HashSet<String>,
        admin_federations: HashSet<String>,
    ) -> Self {
        Self {
            read_federations,
            write_federations,
            admin_federations,
            ..Default::default()
        }
    }
    
    /// Add read access for a federation
    pub fn add_read_federation(&mut self, federation_id: String) {
        self.read_federations.insert(federation_id);
    }
    
    /// Add write access for a federation
    pub fn add_write_federation(&mut self, federation_id: String) {
        self.write_federations.insert(federation_id);
    }
    
    /// Add admin access for a federation
    pub fn add_admin_federation(&mut self, federation_id: String) {
        self.admin_federations.insert(federation_id);
    }
    
    /// Remove read access for a federation
    pub fn remove_read_federation(&mut self, federation_id: &str) {
        self.read_federations.remove(federation_id);
    }
    
    /// Remove write access for a federation
    pub fn remove_write_federation(&mut self, federation_id: &str) {
        self.write_federations.remove(federation_id);
    }
    
    /// Remove admin access for a federation
    pub fn remove_admin_federation(&mut self, federation_id: &str) {
        self.admin_federations.remove(federation_id);
    }
    
    /// Check if a federation has read access
    pub fn can_read(&self, federation_id: &str) -> bool {
        self.admin_federations.contains(federation_id) ||
        self.write_federations.contains(federation_id) ||
        self.read_federations.contains(federation_id)
    }
    
    /// Check if a federation has write access
    pub fn can_write(&self, federation_id: &str) -> bool {
        self.admin_federations.contains(federation_id) ||
        self.write_federations.contains(federation_id)
    }
    
    /// Check if a federation has admin access
    pub fn can_admin(&self, federation_id: &str) -> bool {
        self.admin_federations.contains(federation_id)
    }
    
    /// Set the redundancy factor
    pub fn set_redundancy_factor(&mut self, factor: u8) {
        self.redundancy_factor = factor;
    }
    
    /// Set whether encryption is required
    pub fn set_encryption_required(&mut self, required: bool) {
        self.encryption_required = required;
    }
    
    /// Set the expiration time
    pub fn set_expiration_time(&mut self, expiration_time: Option<u64>) {
        self.expiration_time = expiration_time;
    }
    
    /// Enable or disable versioning
    pub fn set_versioning(&mut self, enabled: bool, max_versions: Option<u32>) {
        self.versioning_enabled = enabled;
        if let Some(max) = max_versions {
            self.max_versions = max;
        }
    }
    
    /// Check if the policy has expired
    pub fn is_expired(&self, current_time: u64) -> bool {
        self.expiration_time
            .map(|expiry| current_time >= expiry)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_access_control() {
        let mut policy = DataAccessPolicy::new();
        
        policy.add_read_federation("fed1".to_string());
        policy.add_write_federation("fed2".to_string());
        policy.add_admin_federation("fed3".to_string());
        
        // Test read access
        assert!(policy.can_read("fed1"));
        assert!(policy.can_read("fed2")); // Write implies read
        assert!(policy.can_read("fed3")); // Admin implies read
        assert!(!policy.can_read("fed4"));
        
        // Test write access
        assert!(!policy.can_write("fed1"));
        assert!(policy.can_write("fed2"));
        assert!(policy.can_write("fed3")); // Admin implies write
        assert!(!policy.can_write("fed4"));
        
        // Test admin access
        assert!(!policy.can_admin("fed1"));
        assert!(!policy.can_admin("fed2"));
        assert!(policy.can_admin("fed3"));
        assert!(!policy.can_admin("fed4"));
    }
    
    #[test]
    fn test_expiration() {
        let mut policy = DataAccessPolicy::new();
        
        // Test no expiration
        assert!(!policy.is_expired(1000));
        
        // Test with expiration
        policy.set_expiration_time(Some(1000));
        assert!(!policy.is_expired(999));
        assert!(policy.is_expired(1000));
        assert!(policy.is_expired(1001));
    }
    
    #[test]
    fn test_versioning() {
        let mut policy = DataAccessPolicy::new();
        assert!(!policy.versioning_enabled);
        assert_eq!(policy.max_versions, 10); // Default value
        
        policy.set_versioning(true, Some(5));
        assert!(policy.versioning_enabled);
        assert_eq!(policy.max_versions, 5);
        
        policy.set_versioning(false, None);
        assert!(!policy.versioning_enabled);
        assert_eq!(policy.max_versions, 5); // Keeps previous max_versions
    }
} 