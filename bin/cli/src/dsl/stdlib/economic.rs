use anyhow::Result;
use super::StdlibValue;

/// Transfer assets from one account to another
pub fn transfer(args: Vec<StdlibValue>) -> Result<StdlibValue> {
    // Ensure the correct number of arguments
    if args.len() < 4 {
        return Err(anyhow::anyhow!("transfer requires at least 4 arguments: from, to, amount, asset_type"));
    }

    // Extract arguments
    let from = match &args[0] {
        StdlibValue::String(s) => s,
        _ => return Err(anyhow::anyhow!("from must be a string")),
    };

    let to = match &args[1] {
        StdlibValue::String(s) => s,
        _ => return Err(anyhow::anyhow!("to must be a string")),
    };

    let amount = match &args[2] {
        StdlibValue::Integer(n) => *n,
        StdlibValue::Float(n) => *n as i64,
        _ => return Err(anyhow::anyhow!("amount must be a number")),
    };

    let asset_type = match &args[3] {
        StdlibValue::String(s) => s,
        _ => return Err(anyhow::anyhow!("asset_type must be a string")),
    };

    // In a real implementation, this would transfer assets in the economic system
    // For now, we'll just log it and return success
    println!("Transferred {} {} from {} to {}", amount, asset_type, from, to);

    // Return success with the transaction ID
    Ok(StdlibValue::String(format!("tx_{}_{}_{}", from, to, amount)))
}

/// Create a new asset in the system
pub fn create_asset(args: Vec<StdlibValue>) -> Result<StdlibValue> {
    // Ensure the correct number of arguments
    if args.len() < 3 {
        return Err(anyhow::anyhow!("create_asset requires at least 3 arguments: id, asset_type, initial_supply"));
    }

    // Extract arguments
    let id = match &args[0] {
        StdlibValue::String(s) => s,
        _ => return Err(anyhow::anyhow!("id must be a string")),
    };

    let asset_type = match &args[1] {
        StdlibValue::String(s) => s,
        _ => return Err(anyhow::anyhow!("asset_type must be a string")),
    };

    let initial_supply = match &args[2] {
        StdlibValue::Integer(n) => *n,
        StdlibValue::Float(n) => *n as i64,
        _ => return Err(anyhow::anyhow!("initial_supply must be a number")),
    };

    // In a real implementation, this would create a new asset in the economic system
    // For now, we'll just log it and return success
    println!("Created asset {} of type {} with initial supply {}", id, asset_type, initial_supply);

    // Return success with the asset ID
    Ok(StdlibValue::String(id.clone()))
}

/// Get the balance of an asset for an account
pub fn get_balance(args: Vec<StdlibValue>) -> Result<StdlibValue> {
    // Ensure the correct number of arguments
    if args.len() < 2 {
        return Err(anyhow::anyhow!("get_balance requires at least 2 arguments: account, asset_type"));
    }

    // Extract arguments
    let account = match &args[0] {
        StdlibValue::String(s) => s,
        _ => return Err(anyhow::anyhow!("account must be a string")),
    };

    let asset_type = match &args[1] {
        StdlibValue::String(s) => s,
        _ => return Err(anyhow::anyhow!("asset_type must be a string")),
    };

    // In a real implementation, this would retrieve the balance from the economic system
    // For now, we'll return a mock balance
    let balance = 100; // Mock balance

    println!("Retrieved balance for account {} of asset type {}: {}", account, asset_type, balance);

    // Return the balance as an integer
    Ok(StdlibValue::Integer(balance))
}

/// Register all economic functions in the standard library
pub fn register_functions() -> Vec<(String, fn(Vec<StdlibValue>) -> Result<StdlibValue>)> {
    vec![
        ("economic.transfer".to_string(), transfer as fn(Vec<StdlibValue>) -> Result<StdlibValue>),
        ("economic.create_asset".to_string(), create_asset as fn(Vec<StdlibValue>) -> Result<StdlibValue>),
        ("economic.get_balance".to_string(), get_balance as fn(Vec<StdlibValue>) -> Result<StdlibValue>),
    ]
} 