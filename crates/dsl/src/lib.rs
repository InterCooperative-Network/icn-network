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
    Percentage(f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub title: String,
    pub description: String,
    pub quorum: f64, // As a percentage (0-100)
    pub threshold: Option<f64>, // As a percentage (0-100)
    pub voting_method: VotingMethod,
    pub required_role: Option<String>,
    pub voting_period: Option<u64>, // In seconds
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub execution: Vec<ExecutionStep>,
    pub rejection: Option<Vec<ExecutionStep>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VotingMethod {
    Majority,
    Consensus,
    RankedChoice,
    Quadratic,
    SingleChoice,
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
    pub description: Option<String>,
    pub initial_supply: f64,
    pub unit: Option<String>,
    pub divisible: Option<bool>,
    pub permissions: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub parent_role: Option<String>,
    pub max_members: Option<u64>,
    pub assignable_by: Option<Vec<String>>,
    pub attributes: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OnboardingMethod {
    InviteOnly,
    ApprovalVote,
    Open,
    CredentialBased,
    Custom(HashMap<String, Value>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Membership {
    pub name: String,
    pub onboarding: OnboardingMethod,
    pub default_role: Option<String>,
    pub max_members: Option<u64>,
    pub voting_rights: Option<bool>,
    pub credentials: Option<Vec<String>>,
    pub attributes: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Federation {
    pub name: String,
    pub description: Option<String>,
    pub governance_model: Option<String>,
    pub members: Option<Vec<String>>,
    pub joined_date: Option<String>,
    pub resources: Option<Vec<String>>,
    pub attributes: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditSystem {
    pub name: String,
    pub system_type: String,
    pub default_limit: Option<f64>,
    pub global_limit: Option<f64>,
    pub limit_calculation: Option<String>,
    pub trust_metric: Option<String>,
    pub attributes: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ASTNode {
    Proposal(Proposal),
    Asset(Asset),
    Role(Role),
    Membership(Membership),
    Federation(Federation),
    CreditSystem(CreditSystem),
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
                Rule::membership => {
                    nodes.push(Self::parse_membership(pair)?);
                }
                Rule::federation => {
                    nodes.push(Self::parse_federation(pair)?);
                }
                Rule::credit_system => {
                    nodes.push(Self::parse_credit_system(pair)?);
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
        let mut threshold = None;
        let mut voting_method = VotingMethod::Majority;
        let mut required_role = None;
        let mut voting_period = None;
        let mut category = None;
        let mut tags = None;
        let mut execution = Vec::new();
        let mut rejection = None;

        let mut inner_pairs = pair.into_inner();
        let id = inner_pairs.next().unwrap().as_str().to_string();

        for field in inner_pairs {
            match field.as_rule() {
                Rule::proposal_field => {
                    let mut inner = field.into_inner();
                    let field_name = inner.next().unwrap().as_str();
                    let field_value = inner.next().unwrap();

                    match field_name {
                        "title" => title = field_value.as_str().trim_matches('"').to_string(),
                        "description" => description = field_value.as_str().trim_matches('"').to_string(),
                        "quorum" => {
                            if let Ok(q) = field_value.as_str().trim_end_matches('%').parse::<f64>() {
                                quorum = q;
                            }
                        },
                        "threshold" => {
                            if let Ok(t) = field_value.as_str().trim_end_matches('%').parse::<f64>() {
                                threshold = Some(t);
                            }
                        },
                        "voting" => voting_method = Self::parse_voting_method(field_value)?,
                        "required_role" => required_role = Some(field_value.as_str().to_string()),
                        "voting_period" => {
                            if let Ok(p) = field_value.as_str().parse::<u64>() {
                                voting_period = Some(p);
                            }
                        },
                        "category" => category = Some(field_value.as_str().trim_matches('"').to_string()),
                        "tags" => {
                            // Parse the array of tags
                            let tag_values = Self::parse_value(field_value)?;
                            if let Value::Array(arr) = tag_values {
                                let mut tag_strings = Vec::new();
                                for item in arr {
                                    if let Value::String(s) = item {
                                        tag_strings.push(s);
                                    }
                                }
                                tags = Some(tag_strings);
                            }
                        },
                        _ => {}
                    }
                }
                Rule::execution_block => {
                    execution = Self::parse_execution_block(field)?;
                }
                Rule::rejection_block => {
                    rejection = Some(Self::parse_execution_block(field)?);
                }
                _ => {}
            }
        }

        Ok(ASTNode::Proposal(Proposal {
            title,
            description,
            quorum,
            threshold,
            voting_method,
            required_role,
            voting_period,
            category,
            tags,
            execution,
            rejection,
        }))
    }

    fn parse_voting_method(pair: pest::iterators::Pair<Rule>) -> Result<VotingMethod, DSLError> {
        match pair.as_str() {
            "majority" => Ok(VotingMethod::Majority),
            "consensus" => Ok(VotingMethod::Consensus),
            "ranked_choice" => Ok(VotingMethod::RankedChoice),
            "quadratic" => Ok(VotingMethod::Quadratic),
            "single_choice" => Ok(VotingMethod::SingleChoice),
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
            if let Rule::execution_statement = statement.as_rule() {
                for func_call in statement.into_inner() {
                    if let Rule::function_call = func_call.as_rule() {
                        let mut inner = func_call.into_inner();
                        let function = inner.next().unwrap().as_str().to_string();
                        let mut args = Vec::new();
                        
                        for arg in inner {
                            args.push(Self::parse_value(arg)?);
                        }
                        
                        steps.push(ExecutionStep { function, args });
                    }
                }
            }
        }
        
        Ok(steps)
    }

    fn parse_value(pair: pest::iterators::Pair<Rule>) -> Result<Value, DSLError> {
        match pair.as_rule() {
            Rule::string => Ok(Value::String(pair.as_str().trim_matches('"').to_string())),
            Rule::number => Ok(Value::Number(pair.as_str().parse().unwrap())),
            Rule::boolean => Ok(Value::Boolean(pair.as_str() == "true")),
            Rule::percentage => {
                let value = pair.as_str().trim_end_matches('%').parse::<f64>().unwrap();
                Ok(Value::Percentage(value))
            },
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

    fn parse_asset(pair: pest::iterators::Pair<Rule>) -> Result<ASTNode, DSLError> {
        let mut name = String::new();
        let mut asset_type = String::new();
        let mut description = None;
        let mut initial_supply = 0.0;
        let mut unit = None;
        let mut divisible = None;
        let mut permissions = HashMap::new();

        let mut inner_pairs = pair.into_inner();
        name = inner_pairs.next().unwrap().as_str().to_string();

        for field in inner_pairs {
            match field.as_rule() {
                Rule::asset_field => {
                    let mut inner = field.into_inner();
                    let field_name = inner.next().unwrap().as_str();
                    let field_value = inner.next().unwrap();

                    match field_name {
                        "type" => asset_type = field_value.as_str().trim_matches('"').to_string(),
                        "description" => description = Some(field_value.as_str().trim_matches('"').to_string()),
                        "initial_supply" => {
                            if let Ok(s) = field_value.as_str().parse::<f64>() {
                                initial_supply = s;
                            }
                        },
                        "unit" => unit = Some(field_value.as_str().trim_matches('"').to_string()),
                        "divisible" => divisible = Some(field_value.as_str() == "true"),
                        "permissions" => {
                            if let Rule::permissions_block = field_value.as_rule() {
                                for perm in field_value.into_inner() {
                                    let mut perm_inner = perm.into_inner();
                                    let perm_name = perm_inner.next().unwrap().as_str().to_string();
                                    let perm_value = Self::parse_value(perm_inner.next().unwrap())?;
                                    permissions.insert(perm_name, perm_value);
                                }
                            }
                        },
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        Ok(ASTNode::Asset(Asset {
            name,
            asset_type,
            description,
            initial_supply,
            unit,
            divisible,
            permissions,
        }))
    }

    fn parse_role(pair: pest::iterators::Pair<Rule>) -> Result<ASTNode, DSLError> {
        let mut name = String::new();
        let mut description = None;
        let mut permissions = Vec::new();
        let mut parent_role = None;
        let mut max_members = None;
        let mut assignable_by = None;
        let mut attributes = HashMap::new();

        let mut inner_pairs = pair.into_inner();
        name = inner_pairs.next().unwrap().as_str().to_string();

        for field in inner_pairs {
            match field.as_rule() {
                Rule::role_field => {
                    let mut inner = field.into_inner();
                    let field_name = inner.next().unwrap().as_str();
                    let field_value = inner.next().unwrap();

                    match field_name {
                        "permissions" => {
                            if let Value::Array(values) = Self::parse_value(field_value)? {
                                for val in values {
                                    if let Value::String(s) = val {
                                        permissions.push(s);
                                    }
                                }
                            }
                        },
                        "description" => description = Some(field_value.as_str().trim_matches('"').to_string()),
                        "parent_role" => parent_role = Some(field_value.as_str().to_string()),
                        "max_members" => {
                            if let Ok(m) = field_value.as_str().parse::<u64>() {
                                max_members = Some(m);
                            }
                        },
                        "assignable_by" => {
                            if let Value::Array(values) = Self::parse_value(field_value)? {
                                let mut roles = Vec::new();
                                for val in values {
                                    if let Value::String(s) = val {
                                        roles.push(s);
                                    }
                                }
                                assignable_by = Some(roles);
                            }
                        },
                        _ => {
                            let value = Self::parse_value(field_value)?;
                            attributes.insert(field_name.to_string(), value);
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(ASTNode::Role(Role {
            name,
            description,
            permissions,
            parent_role,
            max_members,
            assignable_by,
            attributes,
        }))
    }

    fn parse_membership(pair: pest::iterators::Pair<Rule>) -> Result<ASTNode, DSLError> {
        let mut name = String::new();
        let mut onboarding = OnboardingMethod::Open;
        let mut default_role = None;
        let mut max_members = None;
        let mut voting_rights = None;
        let mut credentials = None;
        let mut attributes = HashMap::new();

        let mut inner_pairs = pair.into_inner();
        name = inner_pairs.next().unwrap().as_str().to_string();

        for field in inner_pairs {
            match field.as_rule() {
                Rule::membership_field => {
                    let mut inner = field.into_inner();
                    let field_name = inner.next().unwrap().as_str();
                    let field_value = inner.next().unwrap();

                    match field_name {
                        "onboarding" => {
                            onboarding = match field_value.as_str() {
                                "invite_only" => OnboardingMethod::InviteOnly,
                                "approval_vote" => OnboardingMethod::ApprovalVote,
                                "open" => OnboardingMethod::Open,
                                "credential_based" => OnboardingMethod::CredentialBased,
                                _ => {
                                    let mut custom = HashMap::new();
                                    for f in field_value.into_inner() {
                                        let mut f_inner = f.into_inner();
                                        let f_name = f_inner.next().unwrap().as_str().to_string();
                                        let f_value = Self::parse_value(f_inner.next().unwrap())?;
                                        custom.insert(f_name, f_value);
                                    }
                                    OnboardingMethod::Custom(custom)
                                }
                            };
                        },
                        "default_role" => default_role = Some(field_value.as_str().to_string()),
                        "max_members" => {
                            if let Ok(m) = field_value.as_str().parse::<u64>() {
                                max_members = Some(m);
                            }
                        },
                        "voting_rights" => voting_rights = Some(field_value.as_str() == "true"),
                        "credentials" => {
                            if let Value::Array(values) = Self::parse_value(field_value)? {
                                let mut creds = Vec::new();
                                for val in values {
                                    if let Value::String(s) = val {
                                        creds.push(s);
                                    }
                                }
                                credentials = Some(creds);
                            }
                        },
                        _ => {
                            let value = Self::parse_value(field_value)?;
                            attributes.insert(field_name.to_string(), value);
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(ASTNode::Membership(Membership {
            name,
            onboarding,
            default_role,
            max_members,
            voting_rights,
            credentials,
            attributes,
        }))
    }

    fn parse_federation(pair: pest::iterators::Pair<Rule>) -> Result<ASTNode, DSLError> {
        let mut name = String::new();
        let mut description = None;
        let mut governance_model = None;
        let mut members = None;
        let mut joined_date = None;
        let mut resources = None;
        let mut attributes = HashMap::new();

        let mut inner_pairs = pair.into_inner();
        name = inner_pairs.next().unwrap().as_str().to_string();

        for field in inner_pairs {
            match field.as_rule() {
                Rule::federation_field => {
                    let mut inner = field.into_inner();
                    let field_name = inner.next().unwrap().as_str();
                    let field_value = inner.next().unwrap();

                    match field_name {
                        "name" => name = field_value.as_str().trim_matches('"').to_string(),
                        "description" => description = Some(field_value.as_str().trim_matches('"').to_string()),
                        "governance_model" => governance_model = Some(field_value.as_str().trim_matches('"').to_string()),
                        "members" => {
                            if let Value::Array(values) = Self::parse_value(field_value)? {
                                let mut member_ids = Vec::new();
                                for val in values {
                                    if let Value::String(s) = val {
                                        member_ids.push(s);
                                    }
                                }
                                members = Some(member_ids);
                            }
                        },
                        "joined_date" => joined_date = Some(field_value.as_str().trim_matches('"').to_string()),
                        "resources" => {
                            if let Value::Array(values) = Self::parse_value(field_value)? {
                                let mut resource_ids = Vec::new();
                                for val in values {
                                    if let Value::String(s) = val {
                                        resource_ids.push(s);
                                    }
                                }
                                resources = Some(resource_ids);
                            }
                        },
                        _ => {
                            let value = Self::parse_value(field_value)?;
                            attributes.insert(field_name.to_string(), value);
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(ASTNode::Federation(Federation {
            name,
            description,
            governance_model,
            members,
            joined_date,
            resources,
            attributes,
        }))
    }

    fn parse_credit_system(pair: pest::iterators::Pair<Rule>) -> Result<ASTNode, DSLError> {
        let mut name = String::new();
        let mut system_type = String::new();
        let mut default_limit = None;
        let mut global_limit = None;
        let mut limit_calculation = None;
        let mut trust_metric = None;
        let mut attributes = HashMap::new();

        let mut inner_pairs = pair.into_inner();
        name = inner_pairs.next().unwrap().as_str().to_string();

        for field in inner_pairs {
            match field.as_rule() {
                Rule::credit_field => {
                    let mut inner = field.into_inner();
                    let field_name = inner.next().unwrap().as_str();
                    let field_value = inner.next().unwrap();

                    match field_name {
                        "type" => system_type = field_value.as_str().trim_matches('"').to_string(),
                        "default_limit" => {
                            if let Ok(l) = field_value.as_str().parse::<f64>() {
                                default_limit = Some(l);
                            }
                        },
                        "global_limit" => {
                            if let Ok(l) = field_value.as_str().parse::<f64>() {
                                global_limit = Some(l);
                            }
                        },
                        "limit_calculation" => limit_calculation = Some(field_value.as_str().trim_matches('"').to_string()),
                        "trust_metric" => trust_metric = Some(field_value.as_str().trim_matches('"').to_string()),
                        _ => {
                            let value = Self::parse_value(field_value)?;
                            attributes.insert(field_name.to_string(), value);
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(ASTNode::CreditSystem(CreditSystem {
            name,
            system_type,
            default_limit,
            global_limit,
            limit_calculation,
            trust_metric,
            attributes,
        }))
    }
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
    
    #[test]
    fn test_parse_role() {
        let input = r#"
            role Admin {
                description = "Administrator role";
                permissions = ["create_proposal", "manage_members", "configure_system"];
                max_members = 5;
            }
        "#;
        
        let result = ICNParser::parse_file(input).unwrap();
        assert_eq!(result.len(), 1);
        
        match &result[0] {
            ASTNode::Role(role) => {
                assert_eq!(role.name, "Admin");
                assert_eq!(role.description, Some("Administrator role".to_string()));
                assert_eq!(role.permissions, vec!["create_proposal", "manage_members", "configure_system"]);
                assert_eq!(role.max_members, Some(5));
            }
            _ => panic!("Expected Role"),
        }
    }
    
    #[test]
    fn test_parse_membership() {
        let input = r#"
            membership DefaultMembership {
                onboarding = approval_vote;
                default_role = "Member";
                max_members = 100;
                voting_rights = true;
            }
        "#;
        
        let result = ICNParser::parse_file(input).unwrap();
        assert_eq!(result.len(), 1);
        
        match &result[0] {
            ASTNode::Membership(membership) => {
                assert_eq!(membership.name, "DefaultMembership");
                matches!(membership.onboarding, OnboardingMethod::ApprovalVote);
                assert_eq!(membership.default_role, Some("Member".to_string()));
                assert_eq!(membership.max_members, Some(100));
                assert_eq!(membership.voting_rights, Some(true));
            }
            _ => panic!("Expected Membership"),
        }
    }
} 