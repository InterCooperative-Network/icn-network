/// Token module for the DSL parser
///
/// This module defines the token types used by the lexical analyzer (lexer)
/// to categorize parts of the DSL input.

/// Token types for the DSL lexer
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// A keyword such as 'proposal', 'asset', etc.
    Keyword(String),
    /// An identifier (variable or entity name)
    Identifier(String),
    /// A string literal enclosed in double quotes
    String(String),
    /// A numeric literal
    Number(String),
    /// An opening brace '{'
    OpenBrace,
    /// A closing brace '}'
    CloseBrace,
    /// An opening parenthesis '('
    OpenParen,
    /// A closing parenthesis ')'
    CloseParen,
    /// An opening square bracket '['
    OpenBracket,
    /// A closing square bracket ']'
    CloseBracket,
    /// A colon ':'
    Colon,
    /// A comma ','
    Comma,
    /// A comment (line starting with '//')
    Comment,
    /// Any other symbol
    Symbol(char),
} 