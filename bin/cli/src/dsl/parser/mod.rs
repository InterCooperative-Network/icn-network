/// DSL Parser Module
///
/// This module is responsible for parsing DSL scripts into an Abstract Syntax Tree (AST)
/// that can be executed by the VM.

use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use std::str::FromStr;

/// Parse a DSL script into an AST
pub fn parse_script(script: &str) -> Result<Ast> {
    let mut parser = Parser::new(script);
    parser.parse()
}

/// Parser for DSL scripts
struct Parser {
    input: String,
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    /// Create a new parser
    fn new(input: &str) -> Self {
        Self {
            input: input.to_string(),
            tokens: Vec::new(),
            position: 0,
        }
    }
    
    /// Parse the input into an AST
    fn parse(&mut self) -> Result<Ast> {
        // Tokenize the input
        self.tokenize()?;
        
        // Parse the tokens into an AST
        let mut ast = Ast::new();
        
        while self.position < self.tokens.len() {
            match self.tokens[self.position] {
                Token::Keyword(ref keyword) => {
                    match keyword.as_str() {
                        "proposal" => {
                            let proposal = self.parse_proposal()?;
                            ast.nodes.push(AstNode::Proposal(proposal));
                        },
                        "asset" => {
                            let asset = self.parse_asset()?;
                            ast.nodes.push(AstNode::Asset(asset));
                        },
                        "transaction" => {
                            let transaction = self.parse_transaction()?;
                            ast.nodes.push(AstNode::Transaction(transaction));
                        },
                        "federation" => {
                            let federation = self.parse_federation()?;
                            ast.nodes.push(AstNode::Federation(federation));
                        },
                        "vote" => {
                            let vote = self.parse_vote()?;
                            ast.nodes.push(AstNode::Vote(vote));
                        },
                        "role" => {
                            let role = self.parse_role()?;
                            ast.nodes.push(AstNode::Role(role));
                        },
                        "permission" => {
                            let permission = self.parse_permission()?;
                            ast.nodes.push(AstNode::Permission(permission));
                        },
                        "log" => {
                            let log = self.parse_log()?;
                            ast.nodes.push(AstNode::Log(log));
                        },
                        _ => {
                            return Err(anyhow!("Unknown keyword: {}", keyword));
                        }
                    }
                },
                Token::Comment => {
                    // Skip comments
                    self.position += 1;
                },
                Token::Whitespace => {
                    // Skip whitespace
                    self.position += 1;
                },
                _ => {
                    return Err(anyhow!("Unexpected token: {:?}", self.tokens[self.position]));
                }
            }
        }
        
        Ok(ast)
    }
    
    /// Tokenize the input
    fn tokenize(&mut self) -> Result<()> {
        // Simple tokenization logic for now
        let mut chars = self.input.chars().peekable();
        let mut tokens = Vec::new();
        
        while let Some(c) = chars.next() {
            match c {
                ' ' | '\t' | '\n' | '\r' => {
                    tokens.push(Token::Whitespace);
                },
                '/' => {
                    if let Some('/') = chars.peek() {
                        // Comment
                        chars.next(); // Consume the second '/'
                        while let Some(c) = chars.peek() {
                            if *c == '\n' {
                                break;
                            }
                            chars.next();
                        }
                        tokens.push(Token::Comment);
                    } else {
                        tokens.push(Token::Symbol('/'));
                    }
                },
                '{' => tokens.push(Token::OpenBrace),
                '}' => tokens.push(Token::CloseBrace),
                '(' => tokens.push(Token::OpenParen),
                ')' => tokens.push(Token::CloseParen),
                '[' => tokens.push(Token::OpenBracket),
                ']' => tokens.push(Token::CloseBracket),
                ':' => tokens.push(Token::Colon),
                ',' => tokens.push(Token::Comma),
                '"' => {
                    // String literal
                    let mut string = String::new();
                    while let Some(c) = chars.next() {
                        if c == '"' {
                            break;
                        }
                        string.push(c);
                    }
                    tokens.push(Token::String(string));
                },
                c if c.is_alphabetic() => {
                    // Identifier or keyword
                    let mut identifier = String::new();
                    identifier.push(c);
                    
                    while let Some(&c) = chars.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            identifier.push(c);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    
                    // Check if it's a keyword
                    let keywords = ["proposal", "asset", "transaction", "federation", "vote", "role", "permission", "log"];
                    if keywords.contains(&identifier.as_str()) {
                        tokens.push(Token::Keyword(identifier));
                    } else {
                        tokens.push(Token::Identifier(identifier));
                    }
                },
                c if c.is_digit(10) => {
                    // Number
                    let mut number = String::new();
                    number.push(c);
                    
                    while let Some(&c) = chars.peek() {
                        if c.is_digit(10) || c == '.' {
                            number.push(c);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    
                    tokens.push(Token::Number(number));
                },
                _ => {
                    tokens.push(Token::Symbol(c));
                }
            }
        }
        
        self.tokens = tokens;
        Ok(())
    }
    
    /// Parse a proposal
    fn parse_proposal(&mut self) -> Result<ProposalNode> {
        // Expect "proposal" keyword
        self.expect_token(Token::Keyword("proposal".to_string()))?;
        self.position += 1;
        
        // Expect identifier or string
        let id = match &self.tokens[self.position] {
            Token::Identifier(id) => id.clone(),
            Token::String(id) => id.clone(),
            _ => return Err(anyhow!("Expected identifier or string after 'proposal'")),
        };
        self.position += 1;
        
        // Expect open brace
        self.expect_token(Token::OpenBrace)?;
        self.position += 1;
        
        // Parse proposal fields
        let mut title = id.clone();
        let mut description = String::new();
        let mut voting_method = VotingMethodNode::Majority;
        let mut execution = Vec::new();
        
        while self.position < self.tokens.len() {
            match &self.tokens[self.position] {
                Token::Identifier(field) => {
                    self.position += 1;
                    
                    // Expect colon
                    self.expect_token(Token::Colon)?;
                    self.position += 1;
                    
                    match field.as_str() {
                        "title" => {
                            // Expect string
                            match &self.tokens[self.position] {
                                Token::String(value) => {
                                    title = value.clone();
                                    self.position += 1;
                                },
                                _ => return Err(anyhow!("Expected string after 'title:'")),
                            }
                        },
                        "description" => {
                            // Expect string
                            match &self.tokens[self.position] {
                                Token::String(value) => {
                                    description = value.clone();
                                    self.position += 1;
                                },
                                _ => return Err(anyhow!("Expected string after 'description:'")),
                            }
                        },
                        "voting_method" => {
                            // Expect identifier
                            match &self.tokens[self.position] {
                                Token::Identifier(value) => {
                                    voting_method = match value.as_str() {
                                        "majority" => VotingMethodNode::Majority,
                                        "ranked_choice" => VotingMethodNode::RankedChoice,
                                        "quadratic" => VotingMethodNode::Quadratic,
                                        _ => return Err(anyhow!("Unknown voting method: {}", value)),
                                    };
                                    self.position += 1;
                                },
                                _ => return Err(anyhow!("Expected identifier after 'voting_method:'")),
                            }
                        },
                        "execution" => {
                            // Expect open brace
                            self.expect_token(Token::OpenBrace)?;
                            self.position += 1;
                            
                            // Parse execution steps
                            while self.position < self.tokens.len() {
                                match &self.tokens[self.position] {
                                    Token::Identifier(action) => {
                                        let action_name = action.clone();
                                        self.position += 1;
                                        
                                        // Expect open paren
                                        self.expect_token(Token::OpenParen)?;
                                        self.position += 1;
                                        
                                        // Parse parameters
                                        let mut params = HashMap::new();
                                        let mut param_index = 0;
                                        
                                        while self.position < self.tokens.len() {
                                            match &self.tokens[self.position] {
                                                Token::String(value) => {
                                                    params.insert(param_index.to_string(), value.clone());
                                                    param_index += 1;
                                                    self.position += 1;
                                                    
                                                    // Check for comma or closing paren
                                                    if self.tokens[self.position] == Token::Comma {
                                                        self.position += 1;
                                                    } else if self.tokens[self.position] == Token::CloseParen {
                                                        self.position += 1;
                                                        break;
                                                    } else {
                                                        return Err(anyhow!("Expected comma or closing paren"));
                                                    }
                                                },
                                                Token::CloseParen => {
                                                    self.position += 1;
                                                    break;
                                                },
                                                _ => return Err(anyhow!("Expected string or closing paren")),
                                            }
                                        }
                                        
                                        execution.push(ExecutionStepNode {
                                            action: action_name,
                                            params,
                                        });
                                    },
                                    Token::CloseBrace => {
                                        self.position += 1;
                                        break;
                                    },
                                    Token::Whitespace | Token::Comment => {
                                        self.position += 1;
                                    },
                                    _ => return Err(anyhow!("Expected identifier or closing brace")),
                                }
                            }
                        },
                        _ => return Err(anyhow!("Unknown field: {}", field)),
                    }
                },
                Token::CloseBrace => {
                    self.position += 1;
                    break;
                },
                Token::Whitespace | Token::Comment => {
                    self.position += 1;
                },
                _ => return Err(anyhow!("Expected identifier or closing brace")),
            }
        }
        
        Ok(ProposalNode {
            id,
            title,
            description,
            voting_method,
            execution,
        })
    }
    
    /// Parse an asset
    fn parse_asset(&mut self) -> Result<AssetNode> {
        // Expect "asset" keyword
        self.expect_token(Token::Keyword("asset".to_string()))?;
        self.position += 1;
        
        // Expect identifier or string
        let id = match &self.tokens[self.position] {
            Token::Identifier(id) => id.clone(),
            Token::String(id) => id.clone(),
            _ => return Err(anyhow!("Expected identifier or string after 'asset'")),
        };
        self.position += 1;
        
        // Expect open brace
        self.expect_token(Token::OpenBrace)?;
        self.position += 1;
        
        // Parse asset fields
        let mut asset_type = AssetType::Token;
        let mut initial_supply = 0;
        
        while self.position < self.tokens.len() {
            match &self.tokens[self.position] {
                Token::Identifier(field) => {
                    self.position += 1;
                    
                    // Expect colon
                    self.expect_token(Token::Colon)?;
                    self.position += 1;
                    
                    match field.as_str() {
                        "type" => {
                            // Expect string
                            match &self.tokens[self.position] {
                                Token::String(value) => {
                                    asset_type = match value.as_str() {
                                        "mutual_credit" => AssetType::MutualCredit,
                                        "token" => AssetType::Token,
                                        "resource" => AssetType::Resource,
                                        _ => return Err(anyhow!("Unknown asset type: {}", value)),
                                    };
                                    self.position += 1;
                                },
                                Token::Identifier(value) => {
                                    asset_type = match value.as_str() {
                                        "mutual_credit" => AssetType::MutualCredit,
                                        "token" => AssetType::Token,
                                        "resource" => AssetType::Resource,
                                        _ => return Err(anyhow!("Unknown asset type: {}", value)),
                                    };
                                    self.position += 1;
                                },
                                _ => return Err(anyhow!("Expected string after 'type:'")),
                            }
                        },
                        "initial_supply" => {
                            // Expect number
                            match &self.tokens[self.position] {
                                Token::Number(value) => {
                                    initial_supply = value.parse::<u64>().context("Invalid initial supply")?;
                                    self.position += 1;
                                },
                                _ => return Err(anyhow!("Expected number after 'initial_supply:'")),
                            }
                        },
                        _ => return Err(anyhow!("Unknown field: {}", field)),
                    }
                },
                Token::CloseBrace => {
                    self.position += 1;
                    break;
                },
                Token::Whitespace | Token::Comment => {
                    self.position += 1;
                },
                _ => return Err(anyhow!("Expected identifier or closing brace")),
            }
        }
        
        Ok(AssetNode {
            id,
            asset_type,
            initial_supply,
        })
    }
    
    /// Parse a transaction
    fn parse_transaction(&mut self) -> Result<TransactionNode> {
        // Expect "transaction" keyword
        self.expect_token(Token::Keyword("transaction".to_string()))?;
        self.position += 1;
        
        // Expect open brace
        self.expect_token(Token::OpenBrace)?;
        self.position += 1;
        
        // Parse transaction fields
        let mut from = String::new();
        let mut to = String::new();
        let mut amount = 0;
        let mut asset_id = String::new();
        
        while self.position < self.tokens.len() {
            match &self.tokens[self.position] {
                Token::Identifier(field) => {
                    self.position += 1;
                    
                    // Expect colon
                    self.expect_token(Token::Colon)?;
                    self.position += 1;
                    
                    match field.as_str() {
                        "from" => {
                            // Expect string
                            match &self.tokens[self.position] {
                                Token::String(value) => {
                                    from = value.clone();
                                    self.position += 1;
                                },
                                _ => return Err(anyhow!("Expected string after 'from:'")),
                            }
                        },
                        "to" => {
                            // Expect string
                            match &self.tokens[self.position] {
                                Token::String(value) => {
                                    to = value.clone();
                                    self.position += 1;
                                },
                                _ => return Err(anyhow!("Expected string after 'to:'")),
                            }
                        },
                        "amount" => {
                            // Expect number
                            match &self.tokens[self.position] {
                                Token::Number(value) => {
                                    amount = value.parse::<u64>().context("Invalid amount")?;
                                    self.position += 1;
                                },
                                _ => return Err(anyhow!("Expected number after 'amount:'")),
                            }
                        },
                        "asset" => {
                            // Expect string
                            match &self.tokens[self.position] {
                                Token::String(value) => {
                                    asset_id = value.clone();
                                    self.position += 1;
                                },
                                Token::Identifier(value) => {
                                    asset_id = value.clone();
                                    self.position += 1;
                                },
                                _ => return Err(anyhow!("Expected string after 'asset:'")),
                            }
                        },
                        _ => return Err(anyhow!("Unknown field: {}", field)),
                    }
                },
                Token::CloseBrace => {
                    self.position += 1;
                    break;
                },
                Token::Whitespace | Token::Comment => {
                    self.position += 1;
                },
                _ => return Err(anyhow!("Expected identifier or closing brace")),
            }
        }
        
        Ok(TransactionNode {
            from,
            to,
            amount,
            asset_id,
        })
    }
    
    /// Parse a federation
    fn parse_federation(&mut self) -> Result<FederationNode> {
        // Placeholder implementation
        // In a real implementation, this would parse a federation declaration
        self.position += 1; // Skip "federation" keyword
        
        // Skip until we find a closing brace
        while self.position < self.tokens.len() && self.tokens[self.position] != Token::CloseBrace {
            self.position += 1;
        }
        
        if self.position < self.tokens.len() {
            self.position += 1; // Skip closing brace
        }
        
        Ok(FederationNode {
            id: "federation".to_string(),
            name: "Default Federation".to_string(),
            bootstrap_peers: Vec::new(),
            allow_cross_federation: false,
            encrypt: true,
        })
    }
    
    /// Parse a vote
    fn parse_vote(&mut self) -> Result<VoteNode> {
        // Placeholder implementation
        // In a real implementation, this would parse a vote declaration
        self.position += 1; // Skip "vote" keyword
        
        // Skip until we find a closing brace
        while self.position < self.tokens.len() && self.tokens[self.position] != Token::CloseBrace {
            self.position += 1;
        }
        
        if self.position < self.tokens.len() {
            self.position += 1; // Skip closing brace
        }
        
        Ok(VoteNode {
            proposal_id: "proposal".to_string(),
            voter_id: "voter".to_string(),
            vote: VoteType::Yes,
        })
    }
    
    /// Parse a role
    fn parse_role(&mut self) -> Result<RoleNode> {
        // Placeholder implementation
        // In a real implementation, this would parse a role declaration
        self.position += 1; // Skip "role" keyword
        
        // Skip until we find a closing brace
        while self.position < self.tokens.len() && self.tokens[self.position] != Token::CloseBrace {
            self.position += 1;
        }
        
        if self.position < self.tokens.len() {
            self.position += 1; // Skip closing brace
        }
        
        Ok(RoleNode {
            id: "role".to_string(),
            name: "Default Role".to_string(),
            permissions: Vec::new(),
        })
    }
    
    /// Parse a permission
    fn parse_permission(&mut self) -> Result<PermissionNode> {
        // Placeholder implementation
        // In a real implementation, this would parse a permission declaration
        self.position += 1; // Skip "permission" keyword
        
        // Skip until we find a closing brace
        while self.position < self.tokens.len() && self.tokens[self.position] != Token::CloseBrace {
            self.position += 1;
        }
        
        if self.position < self.tokens.len() {
            self.position += 1; // Skip closing brace
        }
        
        Ok(PermissionNode {
            id: "permission".to_string(),
            name: "Default Permission".to_string(),
            description: "Default permission description".to_string(),
        })
    }
    
    /// Parse a log
    fn parse_log(&mut self) -> Result<LogNode> {
        // Expect "log" keyword
        self.expect_token(Token::Keyword("log".to_string()))?;
        self.position += 1;
        
        // Expect open paren
        self.expect_token(Token::OpenParen)?;
        self.position += 1;
        
        // Expect string
        let message = match &self.tokens[self.position] {
            Token::String(message) => message.clone(),
            _ => return Err(anyhow!("Expected string in log statement")),
        };
        self.position += 1;
        
        // Expect close paren
        self.expect_token(Token::CloseParen)?;
        self.position += 1;
        
        Ok(LogNode {
            message,
        })
    }
    
    /// Expect a specific token
    fn expect_token(&self, expected: Token) -> Result<()> {
        if self.position >= self.tokens.len() {
            return Err(anyhow!("Unexpected end of input"));
        }
        
        if self.tokens[self.position] != expected {
            return Err(anyhow!("Expected {:?}, found {:?}", expected, self.tokens[self.position]));
        }
        
        Ok(())
    }
}

/// Token types
#[derive(Debug, PartialEq, Clone)]
enum Token {
    Keyword(String),
    Identifier(String),
    String(String),
    Number(String),
    Symbol(char),
    OpenBrace,
    CloseBrace,
    OpenParen,
    CloseParen,
    OpenBracket,
    CloseBracket,
    Colon,
    Comma,
    Whitespace,
    Comment,
}

/// The Abstract Syntax Tree representing a parsed DSL script
#[derive(Debug, Clone)]
pub struct Ast {
    pub nodes: Vec<AstNode>,
}

impl Ast {
    /// Create a new empty AST
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }
}

/// A node in the AST
#[derive(Debug, Clone)]
pub enum AstNode {
    /// A proposal definition
    Proposal(ProposalNode),
    /// An asset definition
    Asset(AssetNode),
    /// A transaction
    Transaction(TransactionNode),
    /// A federation
    Federation(FederationNode),
    /// A vote
    Vote(VoteNode),
    /// A role definition
    Role(RoleNode),
    /// A permission definition
    Permission(PermissionNode),
    /// A voting method definition
    VotingMethod(VotingMethodNode),
    /// An execution step
    ExecutionStep(ExecutionStepNode),
    /// A governance module
    GovernanceModule(GovernanceModuleNode),
    /// A log statement
    Log(LogNode),
}

/// A proposal definition node
#[derive(Debug, Clone)]
pub struct ProposalNode {
    /// Proposal ID
    pub id: String,
    /// Proposal title
    pub title: String,
    /// Proposal description
    pub description: String,
    /// Voting method to use
    pub voting_method: VotingMethodNode,
    /// Execution steps to perform if approved
    pub execution: Vec<ExecutionStepNode>,
}

/// An asset definition node
#[derive(Debug, Clone)]
pub struct AssetNode {
    /// Asset ID
    pub id: String,
    /// Asset type
    pub asset_type: AssetType,
    /// Initial supply
    pub initial_supply: u64,
}

/// A transaction node
#[derive(Debug, Clone)]
pub struct TransactionNode {
    /// From account
    pub from: String,
    /// To account
    pub to: String,
    /// Amount
    pub amount: u64,
    /// Asset ID
    pub asset_id: String,
}

/// A federation node
#[derive(Debug, Clone)]
pub struct FederationNode {
    /// Federation ID
    pub id: String,
    /// Federation name
    pub name: String,
    /// Bootstrap peers
    pub bootstrap_peers: Vec<String>,
    /// Whether to allow cross-federation communication
    pub allow_cross_federation: bool,
    /// Whether to encrypt federation traffic
    pub encrypt: bool,
}

/// A vote node
#[derive(Debug, Clone)]
pub struct VoteNode {
    /// Proposal ID
    pub proposal_id: String,
    /// Voter ID
    pub voter_id: String,
    /// Vote type
    pub vote: VoteType,
}

/// Vote type
#[derive(Debug, Clone)]
pub enum VoteType {
    /// Yes vote
    Yes,
    /// No vote
    No,
    /// Abstain vote
    Abstain,
    /// Ranked choice vote
    RankedChoice(Vec<String>),
}

/// Asset types
#[derive(Debug, Clone)]
pub enum AssetType {
    /// Mutual credit asset
    MutualCredit,
    /// Token asset
    Token,
    /// Resource asset (e.g., CPU, storage)
    Resource,
}

/// A role definition node
#[derive(Debug, Clone)]
pub struct RoleNode {
    /// Role ID
    pub id: String,
    /// Role name
    pub name: String,
    /// Role permissions
    pub permissions: Vec<String>,
}

/// A permission definition node
#[derive(Debug, Clone)]
pub struct PermissionNode {
    /// Permission ID
    pub id: String,
    /// Permission name
    pub name: String,
    /// Permission description
    pub description: String,
}

/// A voting method node
#[derive(Debug, Clone)]
pub enum VotingMethodNode {
    /// Simple majority vote
    Majority,
    /// Ranked choice vote
    RankedChoice,
    /// Quadratic vote
    Quadratic,
    /// Custom vote with threshold
    Custom {
        /// Voting method name
        name: String,
        /// Threshold percentage
        threshold: f64,
    },
}

/// An execution step node
#[derive(Debug, Clone)]
pub struct ExecutionStepNode {
    /// Action to perform
    pub action: String,
    /// Parameters for the action
    pub params: HashMap<String, String>,
}

/// A governance module node
#[derive(Debug, Clone)]
pub struct GovernanceModuleNode {
    /// Module ID
    pub id: String,
    /// Module name
    pub name: String,
    /// Module components
    pub components: Vec<AstNode>,
}

/// A log node
#[derive(Debug, Clone)]
pub struct LogNode {
    /// Log message
    pub message: String,
}
