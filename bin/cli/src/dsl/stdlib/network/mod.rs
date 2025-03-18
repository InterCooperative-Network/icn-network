/// Network standard library for DSL
///
/// This module provides functions for network operations including
/// federation management, peer connections, and communication.

use crate::dsl::stdlib::{StdlibFunction, StdlibValue};

/// Register all network functions
pub fn register_functions() -> Vec<StdlibFunction> {
    vec![
        StdlibFunction {
            name: "create_federation".to_string(),
            handler: create_federation,
        },
        StdlibFunction {
            name: "connect_peer".to_string(),
            handler: connect_peer,
        },
        StdlibFunction {
            name: "join_federation".to_string(),
            handler: join_federation,
        },
        StdlibFunction {
            name: "list_peers".to_string(),
            handler: list_peers,
        },
    ]
}

/// Create a new federation
fn create_federation(args: Vec<StdlibValue>) -> Result<StdlibValue, String> {
    if args.len() < 1 {
        return Err("create_federation requires at least 1 argument: name".to_string());
    }
    
    let name = match &args[0] {
        StdlibValue::String(s) => s,
        _ => return Err("name must be a string".to_string()),
    };
    
    // Optional arguments
    let encrypt = if args.len() > 1 {
        match &args[1] {
            StdlibValue::Boolean(b) => *b,
            _ => true, // Default to true
        }
    } else {
        true
    };
    
    let allow_cross_federation = if args.len() > 2 {
        match &args[2] {
            StdlibValue::Boolean(b) => *b,
            _ => false, // Default to false
        }
    } else {
        false
    };
    
    // In a real implementation, we would call into the network system here
    Ok(StdlibValue::String(format!(
        "Created federation '{}' (encrypted: {}, cross-federation: {})",
        name,
        encrypt,
        allow_cross_federation
    )))
}

/// Connect to a peer
fn connect_peer(args: Vec<StdlibValue>) -> Result<StdlibValue, String> {
    if args.len() < 1 {
        return Err("connect_peer requires at least 1 argument: address".to_string());
    }
    
    let address = match &args[0] {
        StdlibValue::String(s) => s,
        _ => return Err("address must be a string".to_string()),
    };
    
    // In a real implementation, we would call into the network system here
    Ok(StdlibValue::String(format!(
        "Connected to peer at '{}'",
        address
    )))
}

/// Join an existing federation
fn join_federation(args: Vec<StdlibValue>) -> Result<StdlibValue, String> {
    if args.len() < 2 {
        return Err("join_federation requires at least 2 arguments: federation_id and bootstrap_peer".to_string());
    }
    
    let federation_id = match &args[0] {
        StdlibValue::String(s) => s,
        _ => return Err("federation_id must be a string".to_string()),
    };
    
    let bootstrap_peer = match &args[1] {
        StdlibValue::String(s) => s,
        _ => return Err("bootstrap_peer must be a string".to_string()),
    };
    
    // In a real implementation, we would call into the network system here
    Ok(StdlibValue::String(format!(
        "Joined federation '{}' via bootstrap peer '{}'",
        federation_id,
        bootstrap_peer
    )))
}

/// List peers in the network
fn list_peers(args: Vec<StdlibValue>) -> Result<StdlibValue, String> {
    let federation_id = if !args.is_empty() {
        match &args[0] {
            StdlibValue::String(s) => Some(s.as_str()),
            _ => return Err("federation_id must be a string".to_string()),
        }
    } else {
        None
    };
    
    // In a real implementation, we would call into the network system here
    // For now, we just return a mock list of peers
    let peers = vec![
        StdlibValue::String("peer1.example.com:8000".to_string()),
        StdlibValue::String("peer2.example.com:8000".to_string()),
        StdlibValue::String("peer3.example.com:8000".to_string()),
    ];
    
    let federation_msg = federation_id
        .map(|id| format!(" in federation '{}'", id))
        .unwrap_or_else(|| "".to_string());
    
    println!("Listed peers{}", federation_msg);
    
    Ok(StdlibValue::Array(peers))
} 