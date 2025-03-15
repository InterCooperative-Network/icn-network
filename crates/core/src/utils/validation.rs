//! Validation utilities
//!
//! This module provides common validation functions for various types of data.

use std::net::{IpAddr, SocketAddr};
use super::UtilError;

/// Validate an IP address
pub fn validate_ip(ip: &str) -> Result<IpAddr, UtilError> {
    ip.parse::<IpAddr>()
        .map_err(|e| UtilError::InvalidValue(format!("Invalid IP address: {}, error: {}", ip, e)))
}

/// Validate a port number (0-65535)
pub fn validate_port(port: u16) -> Result<u16, UtilError> {
    // Technically all u16 values are valid ports, but we might want to check for reserved ports
    // or other constraints in the future
    Ok(port)
}

/// Validate a socket address (IP:Port)
pub fn validate_socket_addr(addr: &str) -> Result<SocketAddr, UtilError> {
    addr.parse::<SocketAddr>()
        .map_err(|e| UtilError::InvalidValue(format!("Invalid socket address: {}, error: {}", addr, e)))
}

/// Validate a string is not empty
pub fn validate_non_empty(s: &str, field_name: &str) -> Result<(), UtilError> {
    if s.trim().is_empty() {
        return Err(UtilError::InvalidValue(format!("{} cannot be empty", field_name)));
    }
    Ok(())
}

/// Validate a string length is between min and max (inclusive)
pub fn validate_string_length(s: &str, min: usize, max: usize, field_name: &str) -> Result<(), UtilError> {
    let len = s.len();
    if len < min || len > max {
        return Err(UtilError::InvalidValue(
            format!("{} must be between {} and {} characters, got {}", field_name, min, max, len)
        ));
    }
    Ok(())
}

/// Validate a numeric value is between min and max (inclusive)
pub fn validate_range<T: PartialOrd + std::fmt::Display>(
    value: T,
    min: T,
    max: T,
    field_name: &str,
) -> Result<(), UtilError> {
    if value < min || value > max {
        return Err(UtilError::InvalidValue(
            format!("{} must be between {} and {}, got {}", field_name, min, max, value)
        ));
    }
    Ok(())
}

/// Validate an email address format
pub fn validate_email(email: &str) -> Result<(), UtilError> {
    // Simple email validation - contains @ and at least one . after @
    if !email.contains('@') {
        return Err(UtilError::InvalidValue(format!("Invalid email address: {}, missing @", email)));
    }
    
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(UtilError::InvalidValue(format!("Invalid email address: {}", email)));
    }
    
    if !parts[1].contains('.') {
        return Err(UtilError::InvalidValue(format!("Invalid email domain: {}", parts[1])));
    }
    
    Ok(())
}

/// Validate a URL format
pub fn validate_url(url: &str) -> Result<(), UtilError> {
    // Simple URL validation - starts with http:// or https:// and contains at least one .
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(UtilError::InvalidValue(
            format!("Invalid URL: {}, must start with http:// or https://", url)
        ));
    }
    
    if !url.contains('.') {
        return Err(UtilError::InvalidValue(format!("Invalid URL: {}, missing domain", url)));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_ip() {
        assert!(validate_ip("127.0.0.1").is_ok());
        assert!(validate_ip("::1").is_ok());
        assert!(validate_ip("not-an-ip").is_err());
    }
    
    #[test]
    fn test_validate_socket_addr() {
        assert!(validate_socket_addr("127.0.0.1:8000").is_ok());
        assert!(validate_socket_addr("[::1]:8000").is_ok());
        assert!(validate_socket_addr("localhost:8000").is_err());
        assert!(validate_socket_addr("127.0.0.1").is_err());
    }
    
    #[test]
    fn test_validate_non_empty() {
        assert!(validate_non_empty("hello", "test").is_ok());
        assert!(validate_non_empty("", "test").is_err());
        assert!(validate_non_empty("   ", "test").is_err());
    }
    
    #[test]
    fn test_validate_string_length() {
        assert!(validate_string_length("hello", 1, 10, "test").is_ok());
        assert!(validate_string_length("", 1, 10, "test").is_err());
        assert!(validate_string_length("hello world!", 1, 10, "test").is_err());
    }
    
    #[test]
    fn test_validate_range() {
        assert!(validate_range(5, 1, 10, "test").is_ok());
        assert!(validate_range(1, 1, 10, "test").is_ok());
        assert!(validate_range(10, 1, 10, "test").is_ok());
        assert!(validate_range(0, 1, 10, "test").is_err());
        assert!(validate_range(11, 1, 10, "test").is_err());
    }
    
    #[test]
    fn test_validate_email() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("user.name+tag@example.co.uk").is_ok());
        assert!(validate_email("user@example").is_err());
        assert!(validate_email("user@.com").is_err());
        assert!(validate_email("@example.com").is_err());
        assert!(validate_email("user@").is_err());
        assert!(validate_email("userexample.com").is_err());
    }
    
    #[test]
    fn test_validate_url() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("http://example.co.uk/path").is_ok());
        assert!(validate_url("example.com").is_err());
        assert!(validate_url("ftp://example.com").is_err());
    }
} 