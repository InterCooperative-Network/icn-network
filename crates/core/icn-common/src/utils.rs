//! Utility functions for the Intercooperative Network

use uuid::Uuid;

/// Generate a random UUID
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}

/// Generate a random UUID with a specific prefix
pub fn generate_prefixed_uuid(prefix: &str) -> String {
    format!("{}:{}", prefix, generate_uuid())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_uuid() {
        let uuid = generate_uuid();
        assert!(!uuid.is_empty());
        assert_eq!(uuid.len(), 36); // Standard UUID length
    }

    #[test]
    fn test_generate_prefixed_uuid() {
        let prefix = "test";
        let uuid = generate_prefixed_uuid(prefix);
        assert!(uuid.starts_with("test:"));
        assert_eq!(uuid.len(), 41); // prefix + ":" + UUID
    }
}