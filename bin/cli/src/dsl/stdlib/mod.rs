/// Standard library for the DSL
///
/// This module contains the standard library of functions and types
/// that can be used in DSL scripts.

// Import necessary libraries
use std::collections::HashMap;
use anyhow::Result;

// Define the modules but don't include inline implementations
mod governance;
mod economic;
mod network;

/// Values that can be returned from stdlib functions
#[derive(Debug, Clone)]
pub enum StdlibValue {
    /// String value
    String(String),
    /// Integer value
    Integer(i64),
    /// Float value
    Float(f64),
    /// Boolean value
    Boolean(bool),
    /// Array of values
    Array(Vec<StdlibValue>),
    /// Map of key-value pairs
    Map(HashMap<String, StdlibValue>),
}

/// Function registration for the standard library
pub type StdlibFunction = fn(Vec<StdlibValue>) -> Result<StdlibValue>;

/// Function registration for the standard library
pub struct StdlibRegistry {
    functions: HashMap<String, StdlibFunction>,
}

impl StdlibRegistry {
    /// Create a new stdlib registry with default functions
    pub fn new() -> Self {
        let mut functions = HashMap::new();
        
        // Register functions from each module
        for (name, func) in governance::register_functions() {
            functions.insert(name, func);
        }
        
        for (name, func) in economic::register_functions() {
            functions.insert(name, func);
        }
        
        for (name, func) in network::register_functions() {
            functions.insert(name, func);
        }
        
        Self { functions }
    }
    
    /// Call a function by name
    pub fn call(&self, name: &str, args: Vec<StdlibValue>) -> Result<StdlibValue> {
        if let Some(func) = self.functions.get(name) {
            func(args)
        } else {
            Err(anyhow::anyhow!("Function '{}' not found in stdlib", name))
        }
    }
}

