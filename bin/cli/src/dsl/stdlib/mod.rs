/// Standard library for the DSL
///
/// This module contains the standard library of functions and types
/// that can be used in DSL scripts.

pub mod governance;
pub mod economic;
pub mod network;

/// Function registration for the standard library
#[derive(Debug, Clone)]
pub struct StdlibRegistry {
    /// Governance functions
    pub governance_functions: Vec<StdlibFunction>,
    /// Economic functions
    pub economic_functions: Vec<StdlibFunction>,
    /// Network functions
    pub network_functions: Vec<StdlibFunction>,
}

/// A function in the standard library
#[derive(Debug, Clone)]
pub struct StdlibFunction {
    /// Name of the function
    pub name: String,
    /// Handler for the function
    pub handler: fn(args: Vec<StdlibValue>) -> Result<StdlibValue, String>,
}

/// Values that can be returned from stdlib functions
#[derive(Debug, Clone)]
pub enum StdlibValue {
    /// String value
    String(String),
    /// Number value
    Number(f64),
    /// Boolean value
    Boolean(bool),
    /// Array of values
    Array(Vec<StdlibValue>),
    /// No value (void)
    Void,
}

impl StdlibRegistry {
    /// Create a new stdlib registry with default functions
    pub fn new() -> Self {
        Self {
            governance_functions: governance::register_functions(),
            economic_functions: economic::register_functions(),
            network_functions: network::register_functions(),
        }
    }

    /// Get a function by name
    pub fn get_function(&self, name: &str) -> Option<&StdlibFunction> {
        self.governance_functions.iter().find(|f| f.name == name)
            .or_else(|| self.economic_functions.iter().find(|f| f.name == name))
            .or_else(|| self.network_functions.iter().find(|f| f.name == name))
    }
}

/// Governance standard library
pub mod governance {
    use anyhow::{Result, anyhow};
    
    /// Create a proposal
    pub fn create_proposal(args: &[&str]) -> Result<String> {
        if args.len() < 3 {
            return Err(anyhow!("create_proposal requires at least 3 arguments: title, description, proposer"));
        }
        
        let title = args[0];
        let description = args[1];
        let proposer = args[2];
        
        // In a real implementation, this would create a proposal in the governance system
        
        Ok(format!("Proposal created: {}, by {}", title, proposer))
    }
    
    /// Cast a vote on a proposal
    pub fn cast_vote(args: &[&str]) -> Result<String> {
        if args.len() < 3 {
            return Err(anyhow!("cast_vote requires at least 3 arguments: proposal_id, voter, vote"));
        }
        
        let proposal_id = args[0];
        let voter = args[1];
        let vote = args[2];
        
        // In a real implementation, this would cast a vote in the governance system
        
        Ok(format!("Vote cast: {} by {} on proposal {}", vote, voter, proposal_id))
    }
    
    /// Execute a proposal
    pub fn execute_proposal(args: &[&str]) -> Result<String> {
        if args.len() < 1 {
            return Err(anyhow!("execute_proposal requires at least 1 argument: proposal_id"));
        }
        
        let proposal_id = args[0];
        
        // In a real implementation, this would execute a proposal in the governance system
        
        Ok(format!("Proposal executed: {}", proposal_id))
    }
}

/// Economic standard library
pub mod economic {
    use anyhow::{Result, anyhow};
    
    /// Transfer assets
    pub fn transfer(args: &[&str]) -> Result<String> {
        if args.len() < 4 {
            return Err(anyhow!("transfer requires at least 4 arguments: from, to, amount, asset_type"));
        }
        
        let from = args[0];
        let to = args[1];
        let amount = args[2];
        let asset_type = args[3];
        
        // In a real implementation, this would transfer assets in the economic system
        
        Ok(format!("Transferred {} {} from {} to {}", amount, asset_type, from, to))
    }
    
    /// Create an asset
    pub fn create_asset(args: &[&str]) -> Result<String> {
        if args.len() < 3 {
            return Err(anyhow!("create_asset requires at least 3 arguments: id, asset_type, initial_supply"));
        }
        
        let id = args[0];
        let asset_type = args[1];
        let initial_supply = args[2];
        
        // In a real implementation, this would create an asset in the economic system
        
        Ok(format!("Asset created: {} of type {} with initial supply {}", id, asset_type, initial_supply))
    }
    
    /// Get balance
    pub fn get_balance(args: &[&str]) -> Result<String> {
        if args.len() < 2 {
            return Err(anyhow!("get_balance requires at least 2 arguments: account, asset_type"));
        }
        
        let account = args[0];
        let asset_type = args[1];
        
        // In a real implementation, this would get the balance from the economic system
        
        Ok(format!("Balance for {} of asset {}: 0", account, asset_type))
    }
}

/// Networking standard library
pub mod network {
    use anyhow::{Result, anyhow};
    
    /// Connect to a peer
    pub fn connect_peer(args: &[&str]) -> Result<String> {
        if args.len() < 1 {
            return Err(anyhow!("connect_peer requires at least 1 argument: peer_id"));
        }
        
        let peer_id = args[0];
        
        // In a real implementation, this would connect to a peer in the networking system
        
        Ok(format!("Connected to peer: {}", peer_id))
    }
    
    /// Create a federation
    pub fn create_federation(args: &[&str]) -> Result<String> {
        if args.len() < 2 {
            return Err(anyhow!("create_federation requires at least 2 arguments: id, name"));
        }
        
        let id = args[0];
        let name = args[1];
        
        // In a real implementation, this would create a federation in the networking system
        
        Ok(format!("Federation created: {} ({})", name, id))
    }
    
    /// Join a federation
    pub fn join_federation(args: &[&str]) -> Result<String> {
        if args.len() < 1 {
            return Err(anyhow!("join_federation requires at least 1 argument: federation_id"));
        }
        
        let federation_id = args[0];
        
        // In a real implementation, this would join a federation in the networking system
        
        Ok(format!("Joined federation: {}", federation_id))
    }
}
