/// Economic standard library for DSL
///
/// This module provides functions for economic operations including
/// asset creation, transactions, and mutual credit management.

use crate::dsl::stdlib::{StdlibFunction, StdlibValue};

/// Register all economic functions
pub fn register_functions() -> Vec<StdlibFunction> {
    vec![
        StdlibFunction {
            name: "create_asset".to_string(),
            handler: create_asset,
        },
        StdlibFunction {
            name: "transfer".to_string(),
            handler: transfer,
        },
        StdlibFunction {
            name: "get_balance".to_string(),
            handler: get_balance,
        },
        StdlibFunction {
            name: "create_mutual_credit".to_string(),
            handler: create_mutual_credit,
        },
    ]
}

/// Create a new asset
fn create_asset(args: Vec<StdlibValue>) -> Result<StdlibValue, String> {
    if args.len() < 3 {
        return Err("create_asset requires at least 3 arguments: name, symbol, and initial_supply".to_string());
    }
    
    let name = match &args[0] {
        StdlibValue::String(s) => s,
        _ => return Err("name must be a string".to_string()),
    };
    
    let symbol = match &args[1] {
        StdlibValue::String(s) => s,
        _ => return Err("symbol must be a string".to_string()),
    };
    
    let initial_supply = match &args[2] {
        StdlibValue::Number(n) => n,
        _ => return Err("initial_supply must be a number".to_string()),
    };
    
    // In a real implementation, we would call into the economic system here
    Ok(StdlibValue::String(format!(
        "Created asset '{}' ({}) with initial supply {}", 
        name, 
        symbol,
        initial_supply
    )))
}

/// Transfer assets between accounts
fn transfer(args: Vec<StdlibValue>) -> Result<StdlibValue, String> {
    if args.len() < 4 {
        return Err("transfer requires at least 4 arguments: from, to, amount, and asset".to_string());
    }
    
    let from = match &args[0] {
        StdlibValue::String(s) => s,
        _ => return Err("from must be a string".to_string()),
    };
    
    let to = match &args[1] {
        StdlibValue::String(s) => s,
        _ => return Err("to must be a string".to_string()),
    };
    
    let amount = match &args[2] {
        StdlibValue::Number(n) => n,
        _ => return Err("amount must be a number".to_string()),
    };
    
    let asset = match &args[3] {
        StdlibValue::String(s) => s,
        _ => return Err("asset must be a string".to_string()),
    };
    
    // In a real implementation, we would call into the economic system here
    Ok(StdlibValue::String(format!(
        "Transferred {} {} from {} to {}", 
        amount, 
        asset,
        from,
        to
    )))
}

/// Get the balance of an account
fn get_balance(args: Vec<StdlibValue>) -> Result<StdlibValue, String> {
    if args.len() < 2 {
        return Err("get_balance requires at least 2 arguments: account and asset".to_string());
    }
    
    let account = match &args[0] {
        StdlibValue::String(s) => s,
        _ => return Err("account must be a string".to_string()),
    };
    
    let asset = match &args[1] {
        StdlibValue::String(s) => s,
        _ => return Err("asset must be a string".to_string()),
    };
    
    // In a real implementation, we would call into the economic system here
    // For now, we just return a mock balance
    Ok(StdlibValue::Number(1000.0))
}

/// Create a mutual credit system
fn create_mutual_credit(args: Vec<StdlibValue>) -> Result<StdlibValue, String> {
    if args.len() < 2 {
        return Err("create_mutual_credit requires at least 2 arguments: name and credit_limit".to_string());
    }
    
    let name = match &args[0] {
        StdlibValue::String(s) => s,
        _ => return Err("name must be a string".to_string()),
    };
    
    let credit_limit = match &args[1] {
        StdlibValue::Number(n) => n,
        _ => return Err("credit_limit must be a number".to_string()),
    };
    
    // In a real implementation, we would call into the economic system here
    Ok(StdlibValue::String(format!(
        "Created mutual credit system '{}' with credit limit {}", 
        name, 
        credit_limit
    )))
} 