/// Lexer module for the DSL parser
///
/// This module implements the lexical analyzer (lexer) for the DSL,
/// which converts input text into a stream of tokens.

use anyhow::{Result, anyhow};
use super::token::Token;
use std::iter::Peekable;
use std::str::Chars;

/// Lexer for tokenizing DSL input
pub struct Lexer<'a> {
    /// Input as a peekable character iterator
    input: Peekable<Chars<'a>>,
    /// Current position in the input
    position: usize,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer from input text
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input.chars().peekable(),
            position: 0,
        }
    }
    
    /// Tokenize the entire input
    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        
        while let Some(token) = self.next_token()? {
            tokens.push(token);
        }
        
        Ok(tokens)
    }
    
    /// Get the next token from the input
    pub fn next_token(&mut self) -> Result<Option<Token>> {
        self.skip_whitespace();
        
        let next_char = match self.input.peek() {
            Some(c) => *c,
            None => return Ok(None), // End of input
        };
        
        match next_char {
            '{' => {
                self.input.next();
                self.position += 1;
                Ok(Some(Token::OpenBrace))
            },
            '}' => {
                self.input.next();
                self.position += 1;
                Ok(Some(Token::CloseBrace))
            },
            '(' => {
                self.input.next();
                self.position += 1;
                Ok(Some(Token::OpenParen))
            },
            ')' => {
                self.input.next();
                self.position += 1;
                Ok(Some(Token::CloseParen))
            },
            '[' => {
                self.input.next();
                self.position += 1;
                Ok(Some(Token::OpenBracket))
            },
            ']' => {
                self.input.next();
                self.position += 1;
                Ok(Some(Token::CloseBracket))
            },
            ':' => {
                self.input.next();
                self.position += 1;
                Ok(Some(Token::Colon))
            },
            ',' => {
                self.input.next();
                self.position += 1;
                Ok(Some(Token::Comma))
            },
            '/' => {
                self.input.next();
                self.position += 1;
                
                // Check for comment
                if let Some('/') = self.input.peek() {
                    self.input.next();
                    self.position += 1;
                    
                    // Consume the rest of the line
                    while let Some(c) = self.input.peek() {
                        if *c == '\n' {
                            break;
                        }
                        self.input.next();
                        self.position += 1;
                    }
                    
                    Ok(Some(Token::Comment))
                } else {
                    Ok(Some(Token::Symbol('/')))
                }
            },
            '"' => {
                self.input.next(); // Consume the opening quote
                self.position += 1;
                
                let mut string = String::new();
                
                while let Some(c) = self.input.peek() {
                    if *c == '"' {
                        self.input.next(); // Consume the closing quote
                        self.position += 1;
                        break;
                    }
                    
                    // Handle escape sequences
                    if *c == '\\' {
                        self.input.next(); // Consume the backslash
                        self.position += 1;
                        
                        match self.input.next() {
                            Some('n') => {
                                string.push('\n');
                                self.position += 1;
                            },
                            Some('t') => {
                                string.push('\t');
                                self.position += 1;
                            },
                            Some('r') => {
                                string.push('\r');
                                self.position += 1;
                            },
                            Some('"') => {
                                string.push('"');
                                self.position += 1;
                            },
                            Some('\\') => {
                                string.push('\\');
                                self.position += 1;
                            },
                            Some(c) => {
                                return Err(anyhow!("Invalid escape sequence: \\{}", c));
                            },
                            None => {
                                return Err(anyhow!("Unexpected end of input in escape sequence"));
                            },
                        }
                    } else {
                        string.push(*c);
                        self.input.next();
                        self.position += 1;
                    }
                }
                
                Ok(Some(Token::String(string)))
            },
            c if c.is_alphabetic() || c == '_' => {
                let identifier = self.read_identifier();
                
                // Check if it's a keyword
                let keywords = [
                    "proposal", "asset", "transaction", "federation",
                    "vote", "role", "permission", "log"
                ];
                
                if keywords.contains(&identifier.as_str()) {
                    Ok(Some(Token::Keyword(identifier)))
                } else {
                    Ok(Some(Token::Identifier(identifier)))
                }
            },
            c if c.is_digit(10) => {
                Ok(Some(Token::Number(self.read_number())))
            },
            c => {
                self.input.next();
                self.position += 1;
                Ok(Some(Token::Symbol(c)))
            },
        }
    }
    
    /// Skip whitespace characters
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.input.peek() {
            if c.is_whitespace() {
                self.input.next();
                self.position += 1;
            } else {
                break;
            }
        }
    }
    
    /// Read an identifier
    fn read_identifier(&mut self) -> String {
        let mut identifier = String::new();
        
        while let Some(c) = self.input.peek() {
            if c.is_alphanumeric() || *c == '_' {
                identifier.push(*c);
                self.input.next();
                self.position += 1;
            } else {
                break;
            }
        }
        
        identifier
    }
    
    /// Read a number
    fn read_number(&mut self) -> String {
        let mut number = String::new();
        
        while let Some(c) = self.input.peek() {
            if c.is_digit(10) || *c == '.' {
                number.push(*c);
                self.input.next();
                self.position += 1;
            } else {
                break;
            }
        }
        
        number
    }
} 