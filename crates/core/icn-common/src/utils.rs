//! Utility functions used throughout the ICN project
use std::time::{SystemTime, UNIX_EPOCH};
use rand::Rng;
use crate::error::{Error, Result};

/// Generate a random identifier with a specified prefix
pub fn generate_id(prefix: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis();
    
    let random = rand::thread_rng().gen::<u32>();
    format!("{}-{:x}-{:x}", prefix, timestamp, random)
}

/// Validates that a string matches a specified format using regex
pub fn validate_format(input: &str, regex: &str) -> Result<()> {
    let re = regex::Regex::new(regex).map_err(|e| {
        Error::validation(format!("Invalid regex pattern: {}", e))
    })?;
    
    if !re.is_match(input) {
        return Err(Error::validation(format!(
            "Input '{}' does not match required format", input
        )));
    }
    
    Ok(())
}

/// Convert a byte slice to a hexadecimal string
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

/// Convert a hexadecimal string to a byte vector
pub fn hex_to_bytes(hex: &str) -> Result<Vec<u8>> {
    // Check for valid hex string (must be even length and only hex chars)
    if hex.len() % 2 != 0 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(Error::validation("Invalid hex string"));
    }
    
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|_| Error::validation("Invalid hex string"))
        })
        .collect()
}

/// Helper function to check if a value is valid according to predicate
pub fn validate<T, F>(value: T, predicate: F, message: &str) -> Result<T>
where
    F: FnOnce(&T) -> bool,
{
    if predicate(&value) {
        Ok(value)
    } else {
        Err(Error::validation(message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_id() {
        let id = generate_id("test");
        assert!(id.starts_with("test-"));
        assert!(id.split('-').count() == 3);
    }
    
    #[test]
    fn test_validate_format() {
        // Valid email
        assert!(validate_format("test@example.com", r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").is_ok());
        
        // Invalid email
        assert!(validate_format("invalid-email", r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").is_err());
    }
    
    #[test]
    fn test_hex_conversion() {
        let bytes = vec![0x12, 0x34, 0xAB, 0xCD];
        let hex = bytes_to_hex(&bytes);
        assert_eq!(hex, "1234abcd");
        
        let converted_bytes = hex_to_bytes(&hex).unwrap();
        assert_eq!(converted_bytes, bytes);
        
        // Test invalid hex
        assert!(hex_to_bytes("invalid").is_err());
        assert!(hex_to_bytes("123").is_err()); // Odd length
    }
    
    #[test]
    fn test_validate() {
        // Test valid
        let result = validate(5, |x| *x > 0, "Value must be positive");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 5);
        
        // Test invalid
        let result = validate(-5, |x| *x > 0, "Value must be positive");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Validation error: Value must be positive"
        );
    }
}