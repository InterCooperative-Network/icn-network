use pest::Parser;
use pest_derive::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct ICNParser;

#[derive(Debug, Error)]
pub enum DSLError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub title: String,
    pub description: String,
    pub quorum: f64,
    pub voting_method: VotingMethod,
    pub execution: Vec<ExecutionStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VotingMethod {
    Majority,
    Consensus,
    RankedChoice,
    Custom(HashMap<String, Value>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub function: String,
    pub args: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    pub asset_type: String,
    pub initial_supply: f64,
    pub permissions: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub permissions: Vec<String>,
    pub attributes: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ASTNode {
    Proposal(Proposal),
    Asset(Asset),
    Role(Role),
}

impl ICNParser {
    pub fn parse_file(input: &str) -> Result<Vec<ASTNode>, DSLError> {
        let file = Self::parse(Rule::file, input)
            .map_err(|e| DSLError::ParseError(e.to_string()))?
            .next()
            .unwrap();

        let mut nodes = Vec::new();

        for pair in file.into_inner() {
            match pair.as_rule() {
                Rule::proposal => {
                    nodes.push(Self::parse_proposal(pair)?);
                }
                Rule::asset => {
                    nodes.push(Self::parse_asset(pair)?);
                }
                Rule::role => {
                    nodes.push(Self::parse_role(pair)?);
                }
                Rule::EOI => break,
                _ => continue,
            }
        }

        Ok(nodes)
    }

    fn parse_proposal(pair: pest::iterators::Pair<Rule>) -> Result<ASTNode, DSLError> {
        let mut title = String::new();
        let mut description = String::new();
        let mut quorum = 0.0;
        let mut voting_method = VotingMethod::Majority;
        let mut execution = Vec::new();

        for field in pair.into_inner() {
            match field.as_rule() {
                Rule::proposal_field => {
                    let mut inner = field.into_inner();
                    let field_name = inner.next().unwrap().as_str();
                    let field_value = inner.next().unwrap();

                    match field_name {
                        "title" => title = field_value.as_str().trim_matches('"').to_string(),
                        "description" => description = field_value.as_str().trim_matches('"').to_string(),
                        "quorum" => quorum = field_value.as_str().trim_matches('%').parse().unwrap(),
                        "voting" => voting_method = Self::parse_voting_method(field_value)?,
                        _ => {}
                    }
                }
                Rule::execution_block => {
                    execution = Self::parse_execution_block(field)?;
                }
                _ => {}
            }
        }

        Ok(ASTNode::Proposal(Proposal {
            title,
            description,
            quorum,
            voting_method,
            execution,
        }))
    }

    fn parse_voting_method(pair: pest::iterators::Pair<Rule>) -> Result<VotingMethod, DSLError> {
        match pair.as_str() {
            "majority" => Ok(VotingMethod::Majority),
            "consensus" => Ok(VotingMethod::Consensus),
            "ranked_choice" => Ok(VotingMethod::RankedChoice),
            _ => {
                let mut custom = HashMap::new();
                for field in pair.into_inner() {
                    if let Rule::voting_method_field = field.as_rule() {
                        let mut inner = field.into_inner();
                        let name = inner.next().unwrap().as_str().to_string();
                        let value = Self::parse_value(inner.next().unwrap())?;
                        custom.insert(name, value);
                    }
                }
                Ok(VotingMethod::Custom(custom))
            }
        }
    }

    fn parse_execution_block(pair: pest::iterators::Pair<Rule>) -> Result<Vec<ExecutionStep>, DSLError> {
        let mut steps = Vec::new();
        
        for statement in pair.into_inner() {
            if let Rule::function_call = statement.as_rule() {
                let mut inner = statement.into_inner();
                let function = inner.next().unwrap().as_str().to_string();
                let mut args = Vec::new();
                
                for arg in inner {
                    args.push(Self::parse_value(arg)?);
                }
                
                steps.push(ExecutionStep { function, args });
            }
        }
        
        Ok(steps)
    }

    fn parse_value(pair: pest::iterators::Pair<Rule>) -> Result<Value, DSLError> {
        match pair.as_rule() {
            Rule::string => Ok(Value::String(pair.as_str().trim_matches('"').to_string())),
            Rule::number => Ok(Value::Number(pair.as_str().parse().unwrap())),
            Rule::boolean => Ok(Value::Boolean(pair.as_str() == "true")),
            Rule::array => {
                let values: Result<Vec<Value>, _> = pair
                    .into_inner()
                    .map(|p| Self::parse_value(p))
                    .collect();
                Ok(Value::Array(values?))
            }
            Rule::object => {
                let mut map = HashMap::new();
                for field in pair.into_inner() {
                    let mut inner = field.into_inner();
                    let key = inner.next().unwrap().as_str().to_string();
                    let value = Self::parse_value(inner.next().unwrap())?;
                    map.insert(key, value);
                }
                Ok(Value::Object(map))
            }
            _ => Err(DSLError::ParseError(format!("Unexpected value type: {:?}", pair.as_rule())))
        }
    }

    // Add parse_asset and parse_role methods similarly...
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_proposal() {
        let input = r#"
            proposal TestProposal {
                title = "Test Proposal";
                description = "A test proposal";
                quorum = 60%;
                voting = majority;
                execution = {
                    allocateFunds("Education", 500);
                    notifyMembers("Proposal executed");
                }
            }
        "#;

        let result = ICNParser::parse_file(input).unwrap();
        assert_eq!(result.len(), 1);
        
        match &result[0] {
            ASTNode::Proposal(proposal) => {
                assert_eq!(proposal.title, "Test Proposal");
                assert_eq!(proposal.quorum, 60.0);
                matches!(proposal.voting_method, VotingMethod::Majority);
                assert_eq!(proposal.execution.len(), 2);
            }
            _ => panic!("Expected Proposal"),
        }
    }
} 