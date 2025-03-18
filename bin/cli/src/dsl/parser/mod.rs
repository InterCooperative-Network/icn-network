/// Parser module for the DSL
///
/// This module handles the parsing of DSL scripts into an Abstract Syntax Tree (AST)
/// which can then be executed by the Virtual Machine (VM).

pub mod ast;
pub mod lexer;
pub mod token;

use anyhow::{Result, anyhow};
use self::ast::{Program, Statement, ProposalStatement, AssetStatement, TransactionStatement, 
    FederationStatement, VoteStatement, RoleStatement, PermissionStatement, LogStatement, Expression};
use self::lexer::Lexer;
use self::token::Token;
use std::collections::HashMap;

/// Parser for converting DSL scripts into an AST
pub struct Parser<'a> {
    /// The lexer that tokenizes the input
    lexer: Lexer<'a>,
    /// The current tokens being processed
    tokens: Vec<Token>,
    /// The current position in the token stream
    position: usize,
}

impl<'a> Parser<'a> {
    /// Create a new parser from an input string
    pub fn new(input: &'a str) -> Result<Self> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize()?;
        
        Ok(Self {
            lexer,
            tokens,
            position: 0,
        })
    }
    
    /// Parse the input script into an AST program
    pub fn parse_script(&mut self) -> Result<Program> {
        let mut statements = Vec::new();
        
        while self.position < self.tokens.len() {
            let statement = self.parse_statement()?;
            statements.push(statement);
        }
        
        Ok(Program { statements })
    }
    
    /// Parse a statement from the token stream
    fn parse_statement(&mut self) -> Result<Statement> {
        let token = self.current_token()?;
        
        match token {
            Token::Keyword(keyword) => {
                match keyword.as_str() {
                    "proposal" => self.parse_proposal_statement(),
                    "asset" => self.parse_asset_statement(),
                    "transaction" => self.parse_transaction_statement(),
                    "federation" => self.parse_federation_statement(),
                    "vote" => self.parse_vote_statement(),
                    "role" => self.parse_role_statement(),
                    "permission" => self.parse_permission_statement(),
                    "log" => self.parse_log_statement(),
                    _ => Err(anyhow!("Unexpected keyword: {}", keyword)),
                }
            },
            _ => Err(anyhow!("Expected a statement keyword, found: {:?}", token)),
        }
    }
    
    /// Parse a proposal statement
    fn parse_proposal_statement(&mut self) -> Result<Statement> {
        self.advance_token(); // Consume 'proposal'
        
        let identifier = self.parse_identifier()?;
        let properties = self.parse_properties()?;
        
        self.expect_token(Token::OpenBrace)?;
        self.advance_token(); // Consume '{'
        
        let mut body = Vec::new();
        
        while !self.matches_token(&Token::CloseBrace) && self.position < self.tokens.len() {
            let statement = self.parse_statement()?;
            body.push(statement);
        }
        
        self.expect_token(Token::CloseBrace)?;
        self.advance_token(); // Consume '}'
        
        Ok(Statement::Proposal(ProposalStatement {
            identifier,
            properties,
            body,
        }))
    }
    
    /// Parse an asset statement
    fn parse_asset_statement(&mut self) -> Result<Statement> {
        self.advance_token(); // Consume 'asset'
        
        let identifier = self.parse_identifier()?;
        let properties = self.parse_properties()?;
        
        let body = if self.matches_token(&Token::OpenBrace) {
            self.advance_token(); // Consume '{'
            
            let mut statements = Vec::new();
            
            while !self.matches_token(&Token::CloseBrace) && self.position < self.tokens.len() {
                let statement = self.parse_statement()?;
                statements.push(statement);
            }
            
            self.expect_token(Token::CloseBrace)?;
            self.advance_token(); // Consume '}'
            
            Some(statements)
        } else {
            None
        };
        
        Ok(Statement::Asset(AssetStatement {
            identifier,
            properties,
            body,
        }))
    }
    
    /// Parse a transaction statement
    fn parse_transaction_statement(&mut self) -> Result<Statement> {
        self.advance_token(); // Consume 'transaction'
        
        let identifier = self.parse_identifier()?;
        let properties = self.parse_properties()?;
        
        let body = if self.matches_token(&Token::OpenBrace) {
            self.advance_token(); // Consume '{'
            
            let mut statements = Vec::new();
            
            while !self.matches_token(&Token::CloseBrace) && self.position < self.tokens.len() {
                let statement = self.parse_statement()?;
                statements.push(statement);
            }
            
            self.expect_token(Token::CloseBrace)?;
            self.advance_token(); // Consume '}'
            
            Some(statements)
        } else {
            None
        };
        
        Ok(Statement::Transaction(TransactionStatement {
            identifier,
            properties,
            body,
        }))
    }
    
    /// Parse a federation statement
    fn parse_federation_statement(&mut self) -> Result<Statement> {
        self.advance_token(); // Consume 'federation'
        
        let identifier = self.parse_identifier()?;
        let properties = self.parse_properties()?;
        
        self.expect_token(Token::OpenBrace)?;
        self.advance_token(); // Consume '{'
        
        let mut body = Vec::new();
        
        while !self.matches_token(&Token::CloseBrace) && self.position < self.tokens.len() {
            let statement = self.parse_statement()?;
            body.push(statement);
        }
        
        self.expect_token(Token::CloseBrace)?;
        self.advance_token(); // Consume '}'
        
        Ok(Statement::Federation(FederationStatement {
            identifier,
            properties,
            body,
        }))
    }
    
    /// Parse a vote statement
    fn parse_vote_statement(&mut self) -> Result<Statement> {
        self.advance_token(); // Consume 'vote'
        
        let identifier = self.parse_identifier()?;
        let properties = self.parse_properties()?;
        
        let body = if self.matches_token(&Token::OpenBrace) {
            self.advance_token(); // Consume '{'
            
            let mut statements = Vec::new();
            
            while !self.matches_token(&Token::CloseBrace) && self.position < self.tokens.len() {
                let statement = self.parse_statement()?;
                statements.push(statement);
            }
            
            self.expect_token(Token::CloseBrace)?;
            self.advance_token(); // Consume '}'
            
            Some(statements)
        } else {
            None
        };
        
        Ok(Statement::Vote(VoteStatement {
            identifier,
            properties,
            body,
        }))
    }
    
    /// Parse a role statement
    fn parse_role_statement(&mut self) -> Result<Statement> {
        self.advance_token(); // Consume 'role'
        
        let identifier = self.parse_identifier()?;
        let properties = self.parse_properties()?;
        
        self.expect_token(Token::OpenBrace)?;
        self.advance_token(); // Consume '{'
        
        let mut body = Vec::new();
        
        while !self.matches_token(&Token::CloseBrace) && self.position < self.tokens.len() {
            let statement = self.parse_statement()?;
            body.push(statement);
        }
        
        self.expect_token(Token::CloseBrace)?;
        self.advance_token(); // Consume '}'
        
        Ok(Statement::Role(RoleStatement {
            identifier,
            properties,
            body,
        }))
    }
    
    /// Parse a permission statement
    fn parse_permission_statement(&mut self) -> Result<Statement> {
        self.advance_token(); // Consume 'permission'
        
        let identifier = self.parse_identifier()?;
        let properties = self.parse_properties()?;
        
        let body = if self.matches_token(&Token::OpenBrace) {
            self.advance_token(); // Consume '{'
            
            let mut statements = Vec::new();
            
            while !self.matches_token(&Token::CloseBrace) && self.position < self.tokens.len() {
                let statement = self.parse_statement()?;
                statements.push(statement);
            }
            
            self.expect_token(Token::CloseBrace)?;
            self.advance_token(); // Consume '}'
            
            Some(statements)
        } else {
            None
        };
        
        Ok(Statement::Permission(PermissionStatement {
            identifier,
            properties,
            body,
        }))
    }
    
    /// Parse a log statement
    fn parse_log_statement(&mut self) -> Result<Statement> {
        self.advance_token(); // Consume 'log'
        
        let message = self.parse_expression()?;
        
        Ok(Statement::Log(LogStatement { message }))
    }
    
    /// Parse an identifier
    fn parse_identifier(&mut self) -> Result<String> {
        let token = self.current_token()?;
        
        match token {
            Token::Identifier(identifier) => {
                self.advance_token();
                Ok(identifier)
            },
            _ => Err(anyhow!("Expected an identifier, found: {:?}", token)),
        }
    }
    
    /// Parse properties (key-value pairs)
    fn parse_properties(&mut self) -> Result<HashMap<String, Expression>> {
        let mut properties = HashMap::new();
        
        if self.matches_token(&Token::OpenParen) {
            self.advance_token(); // Consume '('
            
            while !self.matches_token(&Token::CloseParen) && self.position < self.tokens.len() {
                let key = self.parse_identifier()?;
                
                self.expect_token(Token::Colon)?;
                self.advance_token(); // Consume ':'
                
                let value = self.parse_expression()?;
                
                properties.insert(key, value);
                
                if self.matches_token(&Token::Comma) {
                    self.advance_token(); // Consume ','
                } else {
                    break;
                }
            }
            
            self.expect_token(Token::CloseParen)?;
            self.advance_token(); // Consume ')'
        }
        
        Ok(properties)
    }
    
    /// Parse an expression
    fn parse_expression(&mut self) -> Result<Expression> {
        let token = self.current_token()?;
        
        match token {
            Token::String(s) => {
                self.advance_token();
                Ok(Expression::String(s))
            },
            Token::Number(n) => {
                self.advance_token();
                // Convert string number to f64
                match n.parse::<f64>() {
                    Ok(num) => Ok(Expression::Number(num)),
                    Err(_) => Err(anyhow!("Invalid number: {}", n)),
                }
            },
            Token::Identifier(id) => {
                self.advance_token();
                
                // Check if it's a function call
                if self.matches_token(&Token::OpenParen) {
                    self.advance_token(); // Consume '('
                    
                    let mut arguments = Vec::new();
                    
                    while !self.matches_token(&Token::CloseParen) && self.position < self.tokens.len() {
                        let arg = self.parse_expression()?;
                        arguments.push(arg);
                        
                        if self.matches_token(&Token::Comma) {
                            self.advance_token(); // Consume ','
                        } else {
                            break;
                        }
                    }
                    
                    self.expect_token(Token::CloseParen)?;
                    self.advance_token(); // Consume ')'
                    
                    Ok(Expression::FunctionCall {
                        name: id,
                        arguments,
                    })
                } else {
                    Ok(Expression::Identifier(id))
                }
            },
            Token::OpenBracket => {
                self.advance_token(); // Consume '['
                
                let mut elements = Vec::new();
                
                while !self.matches_token(&Token::CloseBracket) && self.position < self.tokens.len() {
                    let element = self.parse_expression()?;
                    elements.push(element);
                    
                    if self.matches_token(&Token::Comma) {
                        self.advance_token(); // Consume ','
                    } else {
                        break;
                    }
                }
                
                self.expect_token(Token::CloseBracket)?;
                self.advance_token(); // Consume ']'
                
                Ok(Expression::Array(elements))
            },
            Token::OpenBrace => {
                self.advance_token(); // Consume '{'
                
                let mut object = HashMap::new();
                
                while !self.matches_token(&Token::CloseBrace) && self.position < self.tokens.len() {
                    let key = self.parse_identifier()?;
                    
                    self.expect_token(Token::Colon)?;
                    self.advance_token(); // Consume ':'
                    
                    let value = self.parse_expression()?;
                    
                    object.insert(key, value);
                    
                    if self.matches_token(&Token::Comma) {
                        self.advance_token(); // Consume ','
                    } else {
                        break;
                    }
                }
                
                self.expect_token(Token::CloseBrace)?;
                self.advance_token(); // Consume '}'
                
                Ok(Expression::Object(object))
            },
            _ => Err(anyhow!("Expected an expression, found: {:?}", token)),
        }
    }
    
    /// Get the current token
    fn current_token(&self) -> Result<&Token> {
        if self.position < self.tokens.len() {
            Ok(&self.tokens[self.position])
        } else {
            Err(anyhow!("Unexpected end of input"))
        }
    }
    
    /// Advance to the next token
    fn advance_token(&mut self) {
        self.position += 1;
    }
    
    /// Check if the current token matches the expected token
    fn matches_token(&self, expected: &Token) -> bool {
        if self.position < self.tokens.len() {
            matches_token_type(&self.tokens[self.position], expected)
        } else {
            false
        }
    }
    
    /// Expect a specific token, returning an error if it doesn't match
    fn expect_token(&self, expected: Token) -> Result<()> {
        let token = self.current_token()?;
        
        if matches_token_type(token, &expected) {
            Ok(())
        } else {
            Err(anyhow!("Expected {:?}, found {:?}", expected, token))
        }
    }
}

/// Check if two tokens match in type (ignoring their values)
fn matches_token_type(a: &Token, b: &Token) -> bool {
    match (a, b) {
        (Token::Keyword(_), Token::Keyword(_)) => true,
        (Token::Identifier(_), Token::Identifier(_)) => true,
        (Token::String(_), Token::String(_)) => true,
        (Token::Number(_), Token::Number(_)) => true,
        (Token::OpenBrace, Token::OpenBrace) => true,
        (Token::CloseBrace, Token::CloseBrace) => true,
        (Token::OpenParen, Token::OpenParen) => true,
        (Token::CloseParen, Token::CloseParen) => true,
        (Token::OpenBracket, Token::OpenBracket) => true,
        (Token::CloseBracket, Token::CloseBracket) => true,
        (Token::Colon, Token::Colon) => true,
        (Token::Comma, Token::Comma) => true,
        (Token::Comment, Token::Comment) => true,
        (Token::Symbol(a), Token::Symbol(b)) => a == b,
        _ => a == b,
    }
}
