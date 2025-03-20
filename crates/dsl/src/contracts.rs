use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use thiserror::Error;
use serde::{Deserialize, Serialize};

// Error type for DSL contracts
#[derive(Debug, Error)]
pub enum ContractError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Evaluation error: {0}")]
    EvaluationError(String),
    #[error("Type error: {0}")]
    TypeError(String),
    #[error("Execution error: {0}")]
    ExecutionError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
}

/// Types of DSL expressions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Expression {
    /// Literal value (string, number, boolean)
    Literal(Value),
    /// Variable reference
    Variable(String),
    /// Binary operation (e.g., addition, comparison)
    BinaryOp {
        /// Left operand
        left: Box<Expression>,
        /// Operator
        op: BinaryOperator,
        /// Right operand
        right: Box<Expression>,
    },
    /// Unary operation (e.g., negation)
    UnaryOp {
        /// Operator
        op: UnaryOperator,
        /// Operand
        expr: Box<Expression>,
    },
    /// Function call
    FunctionCall {
        /// Function name
        name: String,
        /// Arguments
        args: Vec<Expression>,
    },
    /// Block of expressions
    Block(Vec<Expression>),
    /// Conditional expression
    If {
        /// Condition
        condition: Box<Expression>,
        /// Then branch
        then_branch: Box<Expression>,
        /// Else branch
        else_branch: Option<Box<Expression>>,
    },
    /// Loop expression
    Loop {
        /// Condition
        condition: Option<Box<Expression>>,
        /// Body
        body: Box<Expression>,
    },
    /// Assignment
    Assignment {
        /// Target
        target: String,
        /// Value
        value: Box<Expression>,
    },
    /// Object/Map construction
    Object(HashMap<String, Expression>),
    /// Array construction
    Array(Vec<Expression>),
    /// Object property access
    PropertyAccess {
        /// Object expression
        object: Box<Expression>,
        /// Property name
        property: String,
    },
    /// Array index access
    IndexAccess {
        /// Array expression
        array: Box<Expression>,
        /// Index expression
        index: Box<Expression>,
    },
}

/// Binary operators
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BinaryOperator {
    /// Addition
    Add,
    /// Subtraction
    Subtract,
    /// Multiplication
    Multiply,
    /// Division
    Divide,
    /// Modulo
    Modulo,
    /// Equal to
    Equal,
    /// Not equal to
    NotEqual,
    /// Less than
    LessThan,
    /// Less than or equal to
    LessThanOrEqual,
    /// Greater than
    GreaterThan,
    /// Greater than or equal to
    GreaterThanOrEqual,
    /// Logical AND
    And,
    /// Logical OR
    Or,
}

/// Unary operators
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum UnaryOperator {
    /// Negation (numeric)
    Negate,
    /// Logical NOT
    Not,
}

/// Value types in the DSL
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Value {
    /// String value
    String(String),
    /// Numeric value
    Number(f64),
    /// Integer value
    Integer(i64),
    /// Boolean value
    Boolean(bool),
    /// Object value
    Object(HashMap<String, Value>),
    /// Array value
    Array(Vec<Value>),
    /// Null value
    Null,
}

/// A DSL statement
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Statement {
    /// Expression in the statement
    pub expression: Expression,
    /// Location information
    pub location: Option<SourceLocation>,
}

/// Source location information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceLocation {
    /// Source file
    pub file: String,
    /// Start line
    pub start_line: usize,
    /// Start column
    pub start_column: usize,
    /// End line
    pub end_line: usize,
    /// End column
    pub end_column: usize,
}

/// A DSL script
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Script {
    /// Statements in the script
    pub statements: Vec<Statement>,
    /// Source information
    pub source: Option<String>,
    /// Script name
    pub name: Option<String>,
    /// Script metadata
    pub metadata: HashMap<String, String>,
}

/// A DSL function definition
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// Function name
    pub name: String,
    /// Parameters
    pub parameters: Vec<String>,
    /// Function body
    pub body: Box<Expression>,
    /// Documentation
    pub documentation: Option<String>,
}

/// A DSL template
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Template {
    /// Template name
    pub name: String,
    /// Description
    pub description: String,
    /// Parameters
    pub parameters: Vec<TemplateParameter>,
    /// Script template
    pub script_template: String,
    /// Documentation
    pub documentation: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last updated timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// A template parameter
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemplateParameter {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub param_type: TemplateParameterType,
    /// Description
    pub description: String,
    /// Default value
    pub default_value: Option<Value>,
    /// Whether the parameter is required
    pub required: bool,
}

/// Template parameter types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TemplateParameterType {
    /// String parameter
    String,
    /// Numeric parameter
    Number,
    /// Integer parameter
    Integer,
    /// Boolean parameter
    Boolean,
    /// Object parameter
    Object,
    /// Array parameter
    Array,
    /// DID/Address parameter
    Address,
    /// Date parameter
    Date,
    /// Selection from options
    Select(Vec<String>),
}

/// Evaluation result
#[derive(Debug)]
pub enum EvaluationResult {
    /// Value result
    Value(Value),
    /// Error result
    Error(ContractError),
}

/// Environment for evaluating expressions
#[derive(Debug, Clone)]
pub struct Environment {
    /// Variables in scope
    variables: HashMap<String, Value>,
    /// Parent environment
    parent: Option<Box<Environment>>,
}

impl Environment {
    /// Create a new environment
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            parent: None,
        }
    }

    /// Create a new child environment
    pub fn new_child(parent: Environment) -> Self {
        Self {
            variables: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    /// Get a variable's value
    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.variables.get(name) {
            Some(value.clone())
        } else if let Some(parent) = &self.parent {
            parent.get(name)
        } else {
            None
        }
    }

    /// Set a variable's value
    pub fn set(&mut self, name: &str, value: Value) {
        self.variables.insert(name.to_string(), value);
    }

    /// Define a new variable
    pub fn define(&mut self, name: &str, value: Value) {
        self.variables.insert(name.to_string(), value);
    }

    /// Check if a variable is defined in this environment
    pub fn has_own(&self, name: &str) -> bool {
        self.variables.contains_key(name)
    }
}

/// Template engine for DSL contracts
pub struct TemplateEngine {
    /// Templates
    templates: RwLock<HashMap<String, Template>>,
}

impl TemplateEngine {
    /// Create a new template engine
    pub fn new() -> Self {
        Self {
            templates: RwLock::new(HashMap::new()),
        }
    }

    /// Register a template
    pub async fn register_template(&self, template: Template) -> Result<(), ContractError> {
        let mut templates = self.templates.write().await;
        templates.insert(template.name.clone(), template);
        Ok(())
    }

    /// Get a template
    pub async fn get_template(&self, name: &str) -> Result<Template, ContractError> {
        let templates = self.templates.read().await;
        templates
            .get(name)
            .cloned()
            .ok_or_else(|| ContractError::ValidationError(format!("Template not found: {}", name)))
    }

    /// List all templates
    pub async fn list_templates(&self) -> Result<Vec<Template>, ContractError> {
        let templates = self.templates.read().await;
        Ok(templates.values().cloned().collect())
    }

    /// Instantiate a template
    pub async fn instantiate_template(
        &self,
        template_name: &str,
        parameters: HashMap<String, Value>,
    ) -> Result<Script, ContractError> {
        let template = self.get_template(template_name).await?;
        
        // Validate parameters
        for param in &template.parameters {
            if param.required && !parameters.contains_key(&param.name) {
                return Err(ContractError::ValidationError(
                    format!("Missing required parameter: {}", param.name)
                ));
            }
        }
        
        // Simple template instantiation - in a real system, you'd have template placeholders
        // and would replace them with the parameter values
        let statements = Vec::new();  // This would be populated from template_engine processing
        
        Ok(Script {
            statements,
            source: Some(template.script_template.clone()),
            name: Some(format!("{}:{}", template_name, chrono::Utc::now())),
            metadata: HashMap::new(),
        })
    }
}

/// Contract registry for managing smart contracts based on the DSL
pub struct ContractRegistry {
    /// Template engine
    template_engine: Arc<TemplateEngine>,
    /// Deployed contracts
    contracts: RwLock<HashMap<String, Script>>,
}

impl ContractRegistry {
    /// Create a new contract registry
    pub fn new(template_engine: Arc<TemplateEngine>) -> Self {
        Self {
            template_engine,
            contracts: RwLock::new(HashMap::new()),
        }
    }
    
    /// Register a contract template
    pub async fn register_contract_template(
        &self,
        name: &str,
        template: Template,
    ) -> Result<(), ContractError> {
        self.template_engine.register_template(template).await
    }
    
    /// Deploy a contract from a template
    pub async fn deploy_contract(
        &self,
        template_name: &str,
        contract_id: &str,
        parameters: HashMap<String, Value>,
    ) -> Result<String, ContractError> {
        let script = self.template_engine.instantiate_template(
            template_name,
            parameters,
        ).await?;
        
        let mut contracts = self.contracts.write().await;
        contracts.insert(contract_id.to_string(), script);
        
        Ok(contract_id.to_string())
    }
    
    /// Get a deployed contract
    pub async fn get_contract(&self, contract_id: &str) -> Result<Script, ContractError> {
        let contracts = self.contracts.read().await;
        contracts
            .get(contract_id)
            .cloned()
            .ok_or_else(|| ContractError::ValidationError(format!("Contract not found: {}", contract_id)))
    }
    
    /// List all deployed contracts
    pub async fn list_contracts(&self) -> Result<Vec<(String, Script)>, ContractError> {
        let contracts = self.contracts.read().await;
        Ok(contracts.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
    }
} 