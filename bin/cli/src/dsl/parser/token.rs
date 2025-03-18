/// Token module for the DSL parser
///
/// This module defines the tokens used by the DSL lexer.

/// Token types for the DSL
#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    /// Keyword (e.g., "proposal", "asset", "transaction")
    Keyword(String),
    /// Identifier (e.g., variable names)
    Identifier(String),
    /// String literal
    String(String),
    /// Number literal
    Number(String),
    /// Single character symbol
    Symbol(char),
    /// Opening brace {
    OpenBrace,
    /// Closing brace }
    CloseBrace,
    /// Opening parenthesis (
    OpenParen,
    /// Closing parenthesis )
    CloseParen,
    /// Opening bracket [
    OpenBracket,
    /// Closing bracket ]
    CloseBracket,
    /// Colon :
    Colon,
    /// Comma ,
    Comma,
    /// Whitespace
    Whitespace,
    /// Comment
    Comment,
} 