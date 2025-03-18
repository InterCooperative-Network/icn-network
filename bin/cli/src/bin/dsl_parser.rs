/// Command-line tool to parse DSL files
///
/// This binary provides a simple interface to parse DSL files
/// and validate their structure.

use anyhow::{Result, Context};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use cli::dsl;

/// DSL Parser CLI
#[derive(Parser)]
#[clap(name = "dsl_parser")]
#[clap(about = "Command-line tool for parsing DSL files")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse a DSL file
    Parse {
        /// Path to the DSL file
        #[clap(value_parser)]
        path: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse { path } => {
            parse_dsl_file(&path)?;
        }
    }

    Ok(())
}

/// Parse a DSL file and print the parsed structure
fn parse_dsl_file(path: &PathBuf) -> Result<()> {
    let input = fs::read_to_string(path)
        .with_context(|| format!("Failed to read DSL file: {}", path.display()))?;
    
    println!("Parsing DSL file: {}", path.display());
    
    let program = dsl::parse(&input)
        .with_context(|| format!("Failed to parse DSL file: {}", path.display()))?;
    
    println!("Successfully parsed DSL file!");
    println!("Number of statements: {}", program.statements.len());
    
    // Print a simplified view of the parsed structure
    print_program_structure(&program);
    
    Ok(())
}

/// Print a simplified view of the program structure
fn print_program_structure(program: &dsl::parser::ast::Program) {
    use dsl::parser::ast::Statement;
    
    println!("\nProgram Structure:");
    
    for (i, statement) in program.statements.iter().enumerate() {
        match statement {
            Statement::Proposal(proposal) => {
                println!("  {}. Proposal: {}", i + 1, proposal.identifier);
                
                // Print nested statements
                for (j, nested) in proposal.body.iter().enumerate() {
                    print_nested_statement(j + 1, nested, 4);
                }
            },
            Statement::Asset(asset) => {
                println!("  {}. Asset: {}", i + 1, asset.identifier);
                
                // Print nested statements if any
                if let Some(body) = &asset.body {
                    for (j, nested) in body.iter().enumerate() {
                        print_nested_statement(j + 1, nested, 4);
                    }
                }
            },
            Statement::Transaction(transaction) => {
                println!("  {}. Transaction: {}", i + 1, transaction.identifier);
                
                // Print nested statements if any
                if let Some(body) = &transaction.body {
                    for (j, nested) in body.iter().enumerate() {
                        print_nested_statement(j + 1, nested, 4);
                    }
                }
            },
            Statement::Federation(federation) => {
                println!("  {}. Federation: {}", i + 1, federation.identifier);
                
                // Print nested statements
                for (j, nested) in federation.body.iter().enumerate() {
                    print_nested_statement(j + 1, nested, 4);
                }
            },
            Statement::Vote(vote) => {
                println!("  {}. Vote: {}", i + 1, vote.identifier);
                
                // Print nested statements if any
                if let Some(body) = &vote.body {
                    for (j, nested) in body.iter().enumerate() {
                        print_nested_statement(j + 1, nested, 4);
                    }
                }
            },
            Statement::Role(role) => {
                println!("  {}. Role: {}", i + 1, role.identifier);
                
                // Print nested statements
                for (j, nested) in role.body.iter().enumerate() {
                    print_nested_statement(j + 1, nested, 4);
                }
            },
            Statement::Permission(permission) => {
                println!("  {}. Permission: {}", i + 1, permission.identifier);
                
                // Print nested statements if any
                if let Some(body) = &permission.body {
                    for (j, nested) in body.iter().enumerate() {
                        print_nested_statement(j + 1, nested, 4);
                    }
                }
            },
            Statement::Log(log) => {
                println!("  {}. Log", i + 1);
            },
        }
    }
}

/// Print a nested statement with proper indentation
fn print_nested_statement(index: usize, statement: &dsl::parser::ast::Statement, indent: usize) {
    use dsl::parser::ast::Statement;
    
    let indent_str = " ".repeat(indent);
    
    match statement {
        Statement::Proposal(proposal) => {
            println!("{}{}. Proposal: {}", indent_str, index, proposal.identifier);
        },
        Statement::Asset(asset) => {
            println!("{}{}. Asset: {}", indent_str, index, asset.identifier);
        },
        Statement::Transaction(transaction) => {
            println!("{}{}. Transaction: {}", indent_str, index, transaction.identifier);
        },
        Statement::Federation(federation) => {
            println!("{}{}. Federation: {}", indent_str, index, federation.identifier);
        },
        Statement::Vote(vote) => {
            println!("{}{}. Vote: {}", indent_str, index, vote.identifier);
        },
        Statement::Role(role) => {
            println!("{}{}. Role: {}", indent_str, index, role.identifier);
        },
        Statement::Permission(permission) => {
            println!("{}{}. Permission: {}", indent_str, index, permission.identifier);
        },
        Statement::Log(log) => {
            println!("{}{}. Log", indent_str, index);
        },
    }
} 