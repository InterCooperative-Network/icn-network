# ICN Domain-Specific Language (DSL) Implementation

This document provides detailed technical documentation on the implementation of the Domain-Specific Language (DSL) for the InterCooperative Network.

## Overview

The ICN Domain-Specific Language (DSL) is a specialized programming language designed for expressing cooperative agreements, governance rules, and economic relationships. The DSL implementation enables cooperative organizations to create smart contracts that are both human-readable and machine-executable, focusing specifically on cooperative patterns and needs.

## Core Architecture

The DSL implementation consists of several key components:

```
┌─────────────────────────────────────────────────────────────────┐
│                    DSL Implementation                           │
├───────────────┬─────────────────┬───────────────┬───────────────┤
│               │                 │               │               │
│    Parser     │   Interpreter   │   Compiler    │   Template    │
│               │                 │               │    Engine     │
│               │                 │               │               │
├───────────────┴─────────────────┴───────────────┴───────────────┤
│                                                                 │
│                       DSL Manager                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Language Structure

### Expressions

Expressions are the fundamental building blocks of computation in the DSL:

```rust
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
```

### Operators

The DSL supports two types of operators:

```rust
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

pub enum UnaryOperator {
    /// Negation (numeric)
    Negate,
    /// Logical NOT
    Not,
}
```

### Values

The DSL uses a flexible value system to represent different data types:

```rust
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
```

### Statements and Scripts

Statements are expressions with source location information, and scripts are collections of statements:

```rust
pub struct Statement {
    /// Expression in the statement
    pub expression: Expression,
    /// Location information
    pub location: Option<SourceLocation>,
}

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
```

## Parser Implementation

The parser converts source code text into an abstract syntax tree (AST):

```rust
pub struct Parser {
    /// Source code
    source: String,
    /// Current position
    position: usize,
    /// Current line
    line: usize,
    /// Current column
    column: usize,
}
```

The parser implements methods for:
- Tokenizing the source code
- Parsing expressions, statements, and full scripts
- Handling syntax errors with precise location information
- Building the AST for later execution or compilation

Example parsing workflow:

```
┌────────────┐     ┌────────────┐     ┌────────────┐     ┌────────────┐
│   Source   │     │   Tokens   │     │  Abstract  │     │  Validated │
│    Code    │────▶│            │────▶│  Syntax    │────▶│    AST     │
│            │     │            │     │   Tree     │     │            │
└────────────┘     └────────────┘     └────────────┘     └────────────┘
```

## Interpreter Implementation

The interpreter executes DSL code by evaluating the AST within an environment:

```rust
pub struct Interpreter {
    /// Global environment
    global_env: Environment,
    /// Standard library functions
    stdlib: HashMap<String, Box<dyn Fn(Vec<Value>) -> Result<Value, Error> + Send + Sync>>,
    /// Virtual machine for execution
    vm: Option<Arc<VirtualMachine>>,
}
```

### Environment

The environment manages variable scopes and lookups:

```rust
pub struct Environment {
    /// Variables in scope
    variables: HashMap<String, Value>,
    /// Parent environment
    parent: Option<Box<Environment>>,
}
```

The environment implements methods for:
- Creating new environments (global and child scopes)
- Getting variable values, with lookup in parent scopes
- Setting variable values in the current scope
- Defining new variables in the current scope

### Evaluation

The interpreter evaluates expressions recursively:

```rust
pub fn evaluate(&mut self, expr: &Expression, env: &mut Environment) -> Result<Value, Error> {
    match expr {
        Expression::Literal(value) => Ok(value.clone()),
        Expression::Variable(name) => env.get(name).ok_or_else(|| Error::NotFound),
        Expression::BinaryOp { left, op, right } => {
            let left_val = self.evaluate(left, env)?;
            let right_val = self.evaluate(right, env)?;
            
            match op {
                BinaryOperator::Add => self.eval_add(&left_val, &right_val),
                // Other operators...
            }
        },
        // Other expression types...
    }
}
```

The interpreter provides specialized methods for evaluating each type of operation:

```rust
fn eval_add(&self, left: &Value, right: &Value) -> Result<Value, Error> {
    match (left, right) {
        (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l + r)),
        (Value::Integer(l), Value::Integer(r)) => Ok(Value::Integer(l + r)),
        (Value::String(l), Value::String(r)) => Ok(Value::String(format!("{}{}", l, r))),
        _ => Err(Error::InvalidInput("Invalid operands for addition".into())),
    }
}
```

### Standard Library

The interpreter includes a standard library of built-in functions:

```rust
fn init_stdlib() -> HashMap<String, Box<dyn Fn(Vec<Value>) -> Result<Value, Error> + Send + Sync>> {
    let mut stdlib = HashMap::new();
    
    // Add standard library functions
    stdlib.insert("print".to_string(), Box::new(|args| {
        // Implementation...
        Ok(Value::Null)
    }));
    
    stdlib.insert("len".to_string(), Box::new(|args| {
        // Implementation...
        Ok(Value::Integer(length))
    }));
    
    stdlib.insert("now".to_string(), Box::new(|_args| {
        let now = chrono::Utc::now();
        Ok(Value::String(now.to_rfc3339()))
    }));
    
    // Additional standard library functions...
    
    stdlib
}
```

## Compiler Implementation

For performance-critical contracts, the compiler transforms DSL code into bytecode:

```rust
pub struct Compiler {
    /// Optimization level
    optimization_level: usize,
}
```

The compiler implements methods for:
- Setting optimization levels
- Compiling scripts to bytecode
- Performing static analysis and optimizations
- Generating executable bytecode for the VM

## Template Engine Implementation

The template engine allows creating contracts from parameterized templates:

```rust
pub struct TemplateEngine {
    /// Templates
    templates: RwLock<HashMap<String, Template>>,
}

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
```

The template engine implements methods for:
- Registering templates
- Retrieving templates by name
- Listing available templates
- Instantiating templates with parameters

## DSL Manager

The DSL Manager provides a unified interface for working with all aspects of the DSL:

```rust
pub struct DslManager {
    /// Interpreter
    interpreter: Interpreter,
    /// Parser
    parser: Arc<RwLock<Parser>>,
    /// Compiler
    compiler: Compiler,
    /// Template engine
    template_engine: Arc<TemplateEngine>,
}
```

The DSL Manager implements methods for:
- Parsing scripts from source
- Executing scripts directly
- Registering templates
- Instantiating templates
- Executing scripts from templates
- Compiling scripts to bytecode

## Integration with ICN Components

The DSL integrates with other ICN components through specialized APIs:

### VM Integration

```rust
pub fn set_vm(&mut self, vm: Arc<VirtualMachine>) {
    self.interpreter.set_vm(vm);
}
```

This connection allows the DSL to interact with the virtual machine for:
- Accessing blockchain state
- Executing system operations
- Interacting with other contracts
- Accessing protected resources

### Component APIs

The DSL provides access to ICN components through specialized APIs:

- **Identity API**: Access to the DID system
- **Economic API**: Access to accounts and transactions
- **Governance API**: Access to voting and proposals
- **Network API**: Access to messaging and peer information
- **ZKP API**: Access to zero-knowledge proof operations
- **DAO API**: Access to DAO management operations
- **Incentive API**: Access to contribution tracking and rewards
- **Sharding API**: Access to cross-shard operations

## Example Usage

### Basic Script Parsing and Execution

```rust
let source = r#"
    x = 10
    y = 20
    z = x + y
    print(z)
"#;

let mut dsl_manager = DslManager::new();
let script = dsl_manager.parse_script(source.to_string()).await?;
let result = dsl_manager.execute_script(&script)?;
```

### Template Instantiation and Execution

```rust
let template_name = "MutualCreditAgreement";
let parameters = {
    let mut params = HashMap::new();
    params.insert("coop1_did".to_string(), Value::String("did:icn:coop1".to_string()));
    params.insert("coop2_did".to_string(), Value::String("did:icn:coop2".to_string()));
    params.insert("credit_limit".to_string(), Value::Number(10000.0));
    params.insert("duration_days".to_string(), Value::Integer(365));
    params
};

let mut dsl_manager = DslManager::new();
let result = dsl_manager.execute_from_template(template_name, parameters).await?;
```

## Security Considerations

The DSL implementation incorporates several security features:

1. **Sandboxed Execution**: Scripts run in a restricted environment
2. **Resource Limits**: Prevents excessive computation or memory usage
3. **Input Validation**: All inputs are validated before processing
4. **Error Isolation**: Errors in scripts don't affect the wider system
5. **Permissioned APIs**: Access to system functions is controlled

## Conclusion

The ICN DSL implementation provides a powerful, secure, and flexible foundation for expressing cooperative agreements as executable code. With its focus on cooperative patterns and integration with ICN components, the DSL enables cooperatives to automate their relationships while maintaining democratic control and cooperative principles.

The combination of an easy-to-understand syntax, a comprehensive standard library, and a template system makes the DSL accessible to cooperatives regardless of their technical expertise, while the advanced interpreter and compiler ensure efficient execution of contracts on the ICN network. 