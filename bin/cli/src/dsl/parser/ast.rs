/// Abstract Syntax Tree (AST) module for the DSL parser
///
/// This module defines the data structures that represent the parsed DSL script
/// as an Abstract Syntax Tree (AST). The AST can then be interpreted or compiled
/// by the Virtual Machine (VM).

use std::collections::HashMap;

/// The root node of the AST, representing a complete DSL script
#[derive(Debug, Clone)]
pub struct Program {
    /// The statements that make up the program
    pub statements: Vec<Statement>,
}

/// A statement in the DSL
#[derive(Debug, Clone)]
pub enum Statement {
    /// A proposal definition
    Proposal(ProposalStatement),
    /// An asset definition or operation
    Asset(AssetStatement),
    /// A transaction definition or operation
    Transaction(TransactionStatement),
    /// A federation definition or operation
    Federation(FederationStatement),
    /// A vote definition or operation
    Vote(VoteStatement),
    /// A role definition or operation
    Role(RoleStatement),
    /// A permission definition or operation
    Permission(PermissionStatement),
    /// A log statement
    Log(LogStatement),
}

/// A proposal statement defining or referencing a governance proposal
#[derive(Debug, Clone)]
pub struct ProposalStatement {
    /// Name or identifier of the proposal
    pub identifier: String,
    /// Properties of the proposal
    pub properties: HashMap<String, Expression>,
    /// Nested statements within the proposal block
    pub body: Vec<Statement>,
}

/// An asset statement defining or manipulating an asset
#[derive(Debug, Clone)]
pub struct AssetStatement {
    /// Name or identifier of the asset
    pub identifier: String,
    /// Properties of the asset
    pub properties: HashMap<String, Expression>,
    /// Nested statements within the asset block (if any)
    pub body: Option<Vec<Statement>>,
}

/// A transaction statement defining or executing a transaction
#[derive(Debug, Clone)]
pub struct TransactionStatement {
    /// Name or identifier of the transaction
    pub identifier: String,
    /// Properties of the transaction
    pub properties: HashMap<String, Expression>,
    /// Nested statements within the transaction block (if any)
    pub body: Option<Vec<Statement>>,
}

/// A federation statement defining or referencing a federation
#[derive(Debug, Clone)]
pub struct FederationStatement {
    /// Name or identifier of the federation
    pub identifier: String,
    /// Properties of the federation
    pub properties: HashMap<String, Expression>,
    /// Nested statements within the federation block
    pub body: Vec<Statement>,
}

/// A vote statement defining or casting a vote
#[derive(Debug, Clone)]
pub struct VoteStatement {
    /// Name or identifier of the vote
    pub identifier: String,
    /// Properties of the vote
    pub properties: HashMap<String, Expression>,
    /// Nested statements within the vote block (if any)
    pub body: Option<Vec<Statement>>,
}

/// A role statement defining or modifying a role
#[derive(Debug, Clone)]
pub struct RoleStatement {
    /// Name or identifier of the role
    pub identifier: String,
    /// Properties of the role
    pub properties: HashMap<String, Expression>,
    /// Nested statements within the role block
    pub body: Vec<Statement>,
}

/// A permission statement defining or checking permissions
#[derive(Debug, Clone)]
pub struct PermissionStatement {
    /// Name or identifier of the permission
    pub identifier: String,
    /// Properties of the permission
    pub properties: HashMap<String, Expression>,
    /// Nested statements within the permission block (if any)
    pub body: Option<Vec<Statement>>,
}

/// A log statement to output information
#[derive(Debug, Clone)]
pub struct LogStatement {
    /// Message to log
    pub message: Expression,
}

/// An expression that can be evaluated to a value
#[derive(Debug, Clone)]
pub enum Expression {
    /// A string literal
    String(String),
    /// A numeric literal
    Number(f64),
    /// A boolean literal
    Boolean(bool),
    /// An identifier referencing another entity
    Identifier(String),
    /// An array of expressions
    Array(Vec<Expression>),
    /// A key-value map of expressions
    Object(HashMap<String, Expression>),
    /// A function call with arguments
    FunctionCall {
        /// Name of the function
        name: String,
        /// Arguments to the function
        arguments: Vec<Expression>,
    },
} 