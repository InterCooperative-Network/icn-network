//! Transaction processing for the mutual credit system.
//!
//! This module implements the functionality for processing different types of
//! transactions, including direct transfers, path transfers, credit line adjustments,
//! and system operations. It also includes a credit clearing algorithm to optimize
//! the settlement of transactions across the network.

use crate::account::{Account, AccountStatus};
use crate::credit_graph::{CreditGraph, CreditLineId, CreditLineStep};
use crate::credit_line::CreditLine;
use crate::error::CreditError;
use crate::transaction::{Transaction, TransactionStatus, TransactionType, TransactionId};
use crate::types::{Amount, DID, Timestamp};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Parameters for the credit clearing algorithm
#[derive(Debug, Clone)]
pub struct CreditClearingParams {
    /// Minimum amount to consider for clearing
    pub min_clearing_amount: Amount,
    /// Maximum number of hops in a clearing path
    pub max_path_length: usize,
    /// Whether to prioritize high-value transactions
    pub prioritize_high_value: bool,
}

impl Default for CreditClearingParams {
    fn default() -> Self {
        Self {
            min_clearing_amount: Amount::new(1),
            max_path_length: 5,
            prioritize_high_value: true,
        }
    }
}

/// A result from transaction processing
#[derive(Debug, Clone)]
pub struct TransactionResult {
    /// The processed transaction
    pub transaction: Transaction,
    /// Updated balances for affected accounts
    pub updated_balances: HashMap<DID, Amount>,
    /// Any fees or costs associated with the transaction
    pub fees: Option<Amount>,
    /// Timestamp when processing completed
    pub timestamp: Timestamp,
}

/// Transaction processor handles the execution of transactions in the mutual credit system
#[derive(Debug)]
pub struct TransactionProcessor {
    /// The credit graph for the system
    credit_graph: Arc<Mutex<CreditGraph>>,
    /// Parameters for credit clearing
    clearing_params: CreditClearingParams,
    /// Pending transactions to be processed
    pending_transactions: VecDeque<Transaction>,
    /// Processed transaction history
    transaction_history: Vec<TransactionResult>,
}

impl TransactionProcessor {
    /// Create a new transaction processor
    pub fn new(credit_graph: Arc<Mutex<CreditGraph>>, clearing_params: Option<CreditClearingParams>) -> Self {
        Self {
            credit_graph,
            clearing_params: clearing_params.unwrap_or_default(),
            pending_transactions: VecDeque::new(),
            transaction_history: Vec::new(),
        }
    }

    /// Submit a transaction for processing
    pub async fn submit_transaction(&mut self, transaction: Transaction) -> Result<(), CreditError> {
        // Verify the transaction is valid
        self.verify_transaction(&transaction).await?;
        
        // Add to pending queue
        self.pending_transactions.push_back(transaction);
        
        Ok(())
    }

    /// Verify that a transaction is valid
    async fn verify_transaction(&self, transaction: &Transaction) -> Result<(), CreditError> {
        // Lock the credit graph to perform validations
        let graph = self.credit_graph.lock().await;
        
        // Verify accounts exist and are active
        graph.verify_account_active(&transaction.from).await?;
        graph.verify_account_active(&transaction.to).await?;
        
        // Verify the transaction hasn't already been processed
        if self.transaction_history.iter().any(|result| result.transaction.id == transaction.id) {
            return Err(CreditError::InvalidTransaction(format!(
                "Transaction {} has already been processed",
                transaction.id
            )));
        }
        
        // Verify the transaction amount is positive
        if transaction.amount.is_zero() || transaction.amount.is_negative() {
            return Err(CreditError::InvalidTransaction(
                "Transaction amount must be positive".to_string()
            ));
        }
        
        // Additional verification based on transaction type
        match transaction.transaction_type {
            TransactionType::DirectTransfer => {
                // Check if credit line exists and has sufficient credit
                let credit_line_id = CreditLineId::new(&transaction.from, &transaction.to);
                
                if let Some(credit_line) = graph.get_credit_line(&credit_line_id).await? {
                    if !credit_line.is_active() {
                        return Err(CreditError::InactiveCredit(format!(
                            "Credit line from {} to {} is inactive",
                            transaction.from, transaction.to
                        )));
                    }
                    
                    if credit_line.would_exceed_limit(&transaction.amount) {
                        return Err(CreditError::CreditLimitExceeded(format!(
                            "Transaction would exceed credit limit of {}",
                            credit_line.limit
                        )));
                    }
                } else {
                    return Err(CreditError::CreditLineNotFound(format!(
                        "Credit line from {} to {} not found",
                        transaction.from, transaction.to
                    )));
                }
            },
            TransactionType::PathTransfer => {
                // Verify that a valid path exists
                if transaction.path.is_none() || transaction.path.as_ref().unwrap().is_empty() {
                    return Err(CreditError::InvalidTransaction(
                        "Path transfer requires a non-empty path".to_string()
                    ));
                }
                
                // In a full implementation, we would validate each step of the path
                // For now, we'll just ensure the path starts with the sender and ends with the receiver
                let path = transaction.path.as_ref().unwrap();
                if path.first().map(|d| d != &transaction.from).unwrap_or(true) 
                   || path.last().map(|d| d != &transaction.to).unwrap_or(true) {
                    return Err(CreditError::InvalidTransaction(
                        "Path must start with sender and end with receiver".to_string()
                    ));
                }
            },
            TransactionType::CreditLineAdjustment => {
                // Verify that the credit line exists
                let credit_line_id = CreditLineId::new(&transaction.from, &transaction.to);
                if graph.get_credit_line(&credit_line_id).await?.is_none() {
                    return Err(CreditError::CreditLineNotFound(format!(
                        "Credit line from {} to {} not found",
                        transaction.from, transaction.to
                    )));
                }
                
                // Additional verification for credit line adjustments would go here
            },
            TransactionType::SystemOperation => {
                // System operations might have special authorization requirements
                // For now, we'll accept all system operations
            },
        }
        
        Ok(())
    }

    /// Process all pending transactions
    pub async fn process_pending_transactions(&mut self) -> Vec<Result<TransactionResult, CreditError>> {
        let mut results = Vec::new();
        
        while let Some(mut transaction) = self.pending_transactions.pop_front() {
            match self.process_transaction(&mut transaction).await {
                Ok(result) => {
                    self.transaction_history.push(result.clone());
                    results.push(Ok(result));
                },
                Err(error) => {
                    // Mark the transaction as rejected
                    transaction.reject();
                    
                    // Store the failed transaction in history
                    let failed_result = TransactionResult {
                        transaction,
                        updated_balances: HashMap::new(),
                        fees: None,
                        timestamp: chrono::Utc::now(),
                    };
                    self.transaction_history.push(failed_result);
                    
                    results.push(Err(error));
                }
            }
        }
        
        results
    }

    /// Process a single transaction
    async fn process_transaction(&mut self, transaction: &mut Transaction) -> Result<TransactionResult, CreditError> {
        // Verify the transaction is still valid
        self.verify_transaction(transaction).await?;
        
        // Lock the credit graph for updating
        let mut graph = self.credit_graph.lock().await;
        
        let mut updated_balances = HashMap::new();
        
        match transaction.transaction_type {
            TransactionType::DirectTransfer => {
                // Get the credit line
                let credit_line_id = CreditLineId::new(&transaction.from, &transaction.to);
                let credit_line = graph.get_credit_line_mut(&credit_line_id).await?
                    .ok_or_else(|| CreditError::CreditLineNotFound(format!(
                        "Credit line from {} to {} not found",
                        transaction.from, transaction.to
                    )))?;
                
                // Update the credit line balance
                credit_line.update_balance(-transaction.amount.clone())?;
                
                // Update account balances
                if let Some(from_account) = graph.get_account_mut(&transaction.from).await? {
                    from_account.update_balance(-transaction.amount.clone());
                    updated_balances.insert(transaction.from.clone(), from_account.balance.clone());
                }
                
                if let Some(to_account) = graph.get_account_mut(&transaction.to).await? {
                    to_account.update_balance(transaction.amount.clone());
                    updated_balances.insert(transaction.to.clone(), to_account.balance.clone());
                }
            },
            TransactionType::PathTransfer => {
                // Process a path transfer
                self.process_path_transfer(transaction, &mut graph, &mut updated_balances).await?;
            },
            TransactionType::CreditLineAdjustment => {
                // Process a credit line adjustment
                self.process_credit_line_adjustment(transaction, &mut graph, &mut updated_balances).await?;
            },
            TransactionType::SystemOperation => {
                // Process a system operation
                self.process_system_operation(transaction, &mut graph, &mut updated_balances).await?;
            },
        }
        
        // Mark transaction as completed
        transaction.complete();
        
        Ok(TransactionResult {
            transaction: transaction.clone(),
            updated_balances,
            fees: None,
            timestamp: chrono::Utc::now(),
        })
    }

    /// Process a path transfer transaction
    async fn process_path_transfer(
        &self,
        transaction: &Transaction,
        graph: &mut CreditGraph,
        updated_balances: &mut HashMap<DID, Amount>,
    ) -> Result<(), CreditError> {
        // Get the path from the transaction
        let path = transaction.path.as_ref()
            .ok_or_else(|| CreditError::InvalidTransaction("Path transfer requires a path".to_string()))?;
        
        if path.len() < 2 {
            return Err(CreditError::InvalidTransaction("Path must have at least two accounts".to_string()));
        }
        
        // Process each step in the path
        for i in 0..path.len() - 1 {
            let from = &path[i];
            let to = &path[i + 1];
            let credit_line_id = CreditLineId::new(from, to);
            
            // Get and update the credit line
            let credit_line = graph.get_credit_line_mut(&credit_line_id).await?
                .ok_or_else(|| CreditError::CreditLineNotFound(format!(
                    "Credit line from {} to {} not found in path transfer",
                    from, to
                )))?;
            
            // Check if the credit line is active and has enough credit
            if !credit_line.is_active() {
                return Err(CreditError::InactiveCredit(format!(
                    "Credit line from {} to {} is inactive", from, to
                )));
            }
            
            if credit_line.would_exceed_limit(&transaction.amount) {
                return Err(CreditError::CreditLimitExceeded(format!(
                    "Transaction would exceed credit limit of {} for step {} to {}",
                    credit_line.limit, from, to
                )));
            }
            
            // Update the credit line balance
            credit_line.update_balance(-transaction.amount.clone())?;
            
            // Update account balances
            if let Some(from_account) = graph.get_account_mut(from).await? {
                from_account.update_balance(-transaction.amount.clone());
                updated_balances.insert(from.clone(), from_account.balance.clone());
            }
            
            if let Some(to_account) = graph.get_account_mut(to).await? {
                to_account.update_balance(transaction.amount.clone());
                updated_balances.insert(to.clone(), to_account.balance.clone());
            }
        }
        
        Ok(())
    }

    /// Process a credit line adjustment transaction
    async fn process_credit_line_adjustment(
        &self,
        transaction: &Transaction,
        graph: &mut CreditGraph,
        updated_balances: &mut HashMap<DID, Amount>,
    ) -> Result<(), CreditError> {
        // Get the credit line
        let credit_line_id = CreditLineId::new(&transaction.from, &transaction.to);
        let credit_line = graph.get_credit_line_mut(&credit_line_id).await?
            .ok_or_else(|| CreditError::CreditLineNotFound(format!(
                "Credit line from {} to {} not found",
                transaction.from, transaction.to
            )))?;
        
        // Update the credit limit
        credit_line.update_limit(transaction.amount.clone());
        
        // No account balance updates for credit line adjustments
        
        Ok(())
    }

    /// Process a system operation transaction
    async fn process_system_operation(
        &self,
        transaction: &Transaction,
        graph: &mut CreditGraph,
        updated_balances: &mut HashMap<DID, Amount>,
    ) -> Result<(), CreditError> {
        // System operations are highly dependent on the specific operation being performed
        // For now, we'll implement a simple system operation that updates an account's status
        
        // We'll use the metadata to determine the operation
        let operation = transaction.metadata.get("operation")
            .ok_or_else(|| CreditError::InvalidTransaction(
                "System operation requires an 'operation' field in metadata".to_string()
            ))?;
        
        match operation.as_str() {
            Some("update_account_status") => {
                // Get the status from metadata
                let status_str = transaction.metadata.get("status")
                    .ok_or_else(|| CreditError::InvalidTransaction(
                        "Operation 'update_account_status' requires a 'status' field in metadata".to_string()
                    ))?
                    .as_str()
                    .ok_or_else(|| CreditError::InvalidTransaction(
                        "Status field must be a string".to_string()
                    ))?;
                
                // Convert string to AccountStatus
                let status = match status_str {
                    "active" => AccountStatus::Active,
                    "inactive" => AccountStatus::Inactive,
                    "suspended" => AccountStatus::Suspended,
                    "closed" => AccountStatus::Closed,
                    _ => return Err(CreditError::InvalidTransaction(format!(
                        "Invalid account status: {}", status_str
                    ))),
                };
                
                // Update the account status
                let account = graph.get_account_mut(&transaction.to).await?
                    .ok_or_else(|| CreditError::AccountNotFound(
                        transaction.to.to_string()
                    ))?;
                
                account.update_status(status);
                
                // No balance updates for status changes
            },
            Some(op) => {
                return Err(CreditError::NotImplemented(format!(
                    "System operation '{}' is not implemented", op
                )));
            },
            None => {
                return Err(CreditError::InvalidTransaction(
                    "Operation field must be a string".to_string()
                ));
            }
        }
        
        Ok(())
    }

    /// Run the credit clearing algorithm to optimize the network
    pub async fn run_credit_clearing(&mut self) -> Result<Vec<Transaction>, CreditError> {
        // Lock the credit graph
        let mut graph = self.credit_graph.lock().await;
        
        // Credit clearing via cycle detection and resolution
        // This is a simplified version of the credit clearing algorithm
        // A full implementation would use more sophisticated graph algorithms
        
        // Map accounts to their outstanding balances
        let mut account_balances = HashMap::new();
        
        // Get all accounts and their balances
        let accounts = graph.get_all_accounts().await?;
        for account in &accounts {
            account_balances.insert(account.did.clone(), account.balance.clone());
        }
        
        // Find cycles in the credit graph
        let cycles = self.find_credit_cycles(&graph).await?;
        
        // Create transactions for each cycle
        let mut clearing_transactions = Vec::new();
        
        for cycle in cycles {
            if cycle.len() < 3 {
                // Need at least 3 nodes to form a cycle
                continue;
            }
            
            // Find the minimum balance in the cycle
            let min_amount = self.find_minimum_credit_in_cycle(&cycle, &graph).await?;
            
            if min_amount < self.clearing_params.min_clearing_amount {
                // Skip cycles with small amounts
                continue;
            }
            
            // Create transactions to clear the cycle
            for i in 0..cycle.len() {
                let from = cycle[i].clone();
                let to = cycle[(i + 1) % cycle.len()].clone();
                
                // Create a transaction for this step in the cycle
                let transaction = Transaction::new(
                    format!("clearing-{}-{}", from, to),
                    from.clone(),
                    to.clone(),
                    min_amount.clone(),
                    TransactionType::DirectTransfer,
                    Some("Credit clearing transaction".to_string()),
                );
                
                clearing_transactions.push(transaction);
                
                // Update account balances
                if let Some(balance) = account_balances.get_mut(&from) {
                    *balance = balance.clone() - min_amount.clone();
                }
                
                if let Some(balance) = account_balances.get_mut(&to) {
                    *balance = balance.clone() + min_amount.clone();
                }
            }
        }
        
        // Apply the new balances to accounts
        for (did, balance) in account_balances {
            if let Some(account) = graph.get_account_mut(&did).await? {
                account.balance = balance;
                account.updated_at = chrono::Utc::now();
            }
        }
        
        Ok(clearing_transactions)
    }

    /// Find cycles in the credit graph
    async fn find_credit_cycles(&self, graph: &CreditGraph) -> Result<Vec<Vec<DID>>, CreditError> {
        // This is a simplified implementation
        // A full implementation would use a more efficient algorithm
        
        // Get all credit lines
        let credit_lines = graph.get_all_credit_lines().await?;
        
        // Build an adjacency list representation of the graph
        let mut adjacency_list: HashMap<DID, Vec<DID>> = HashMap::new();
        
        for credit_line in &credit_lines {
            if credit_line.is_active() {
                adjacency_list
                    .entry(credit_line.from_account.clone())
                    .or_default()
                    .push(credit_line.to_account.clone());
            }
        }
        
        // Find cycles using DFS
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut path: Vec<DID> = Vec::new();
        
        for account in adjacency_list.keys() {
            if !visited.contains(account) {
                self.dfs_find_cycles(
                    account,
                    &adjacency_list,
                    &mut visited,
                    &mut path,
                    &mut cycles,
                    self.clearing_params.max_path_length,
                );
            }
        }
        
        Ok(cycles)
    }

    /// Depth-first search to find cycles
    fn dfs_find_cycles(
        &self,
        node: &DID,
        adjacency_list: &HashMap<DID, Vec<DID>>,
        visited: &mut HashSet<DID>,
        path: &mut Vec<DID>,
        cycles: &mut Vec<Vec<DID>>,
        max_depth: usize,
    ) {
        if path.len() >= max_depth {
            return;
        }
        
        // Check if we've found a cycle
        if path.contains(node) {
            // Extract the cycle
            let start_idx = path.iter().position(|x| x == node).unwrap();
            let cycle: Vec<DID> = path[start_idx..].iter().cloned().collect();
            cycles.push(cycle);
            return;
        }
        
        visited.insert(node.clone());
        path.push(node.clone());
        
        if let Some(neighbors) = adjacency_list.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    self.dfs_find_cycles(
                        neighbor,
                        adjacency_list,
                        visited,
                        path,
                        cycles,
                        max_depth,
                    );
                } else if path.contains(neighbor) {
                    // We've found a cycle
                    let start_idx = path.iter().position(|x| x == neighbor).unwrap();
                    let cycle: Vec<DID> = path[start_idx..].iter().cloned().collect();
                    cycles.push(cycle);
                }
            }
        }
        
        path.pop();
        visited.remove(node);
    }

    /// Find the minimum credit available in a cycle
    async fn find_minimum_credit_in_cycle(
        &self,
        cycle: &[DID],
        graph: &CreditGraph,
    ) -> Result<Amount, CreditError> {
        let mut min_amount = Amount::new(i64::MAX);
        
        for i in 0..cycle.len() {
            let from = &cycle[i];
            let to = &cycle[(i + 1) % cycle.len()];
            
            let credit_line_id = CreditLineId::new(from, to);
            
            if let Some(credit_line) = graph.get_credit_line(&credit_line_id).await? {
                if credit_line.is_active() {
                    let available_credit = credit_line.available_credit();
                    if available_credit < min_amount {
                        min_amount = available_credit;
                    }
                }
            } else {
                return Err(CreditError::CreditLineNotFound(format!(
                    "Credit line from {} to {} not found in cycle",
                    from, to
                )));
            }
        }
        
        Ok(min_amount)
    }

    /// Get transaction history
    pub fn get_transaction_history(&self) -> &[TransactionResult] {
        &self.transaction_history
    }

    /// Get transaction history for a specific account
    pub fn get_account_transaction_history(&self, account: &DID) -> Vec<&TransactionResult> {
        self.transaction_history
            .iter()
            .filter(|result| {
                result.transaction.from == *account || result.transaction.to == *account
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test::block_on;
    
    fn create_test_environment() -> (Arc<Mutex<CreditGraph>>, TransactionProcessor) {
        let graph = CreditGraph::new();
        let graph = Arc::new(Mutex::new(graph));
        
        let params = CreditClearingParams {
            min_clearing_amount: Amount::new(1),
            max_path_length: 5,
            prioritize_high_value: true,
        };
        
        let processor = TransactionProcessor::new(Arc::clone(&graph), Some(params));
        
        (graph, processor)
    }
    
    #[test]
    fn test_submit_transaction() {
        block_on(async {
            let (graph, mut processor) = create_test_environment();
            
            let mut graph_lock = graph.lock().await;
            
            // Create accounts
            let from_account = Account::new(
                DID::new("from"),
                "From Account".to_string(),
            );
            let to_account = Account::new(
                DID::new("to"),
                "To Account".to_string(),
            );
            
            graph_lock.add_account(from_account).await.unwrap();
            graph_lock.add_account(to_account).await.unwrap();
            
            // Create credit line
            let credit_line = CreditLine::new(
                DID::new("from"),
                DID::new("to"),
                Amount::new(100),
                Default::default(),
            );
            
            graph_lock.add_credit_line(credit_line).await.unwrap();
            
            drop(graph_lock);
            
            // Create transaction
            let transaction = Transaction::new(
                "test-tx".to_string(),
                DID::new("from"),
                DID::new("to"),
                Amount::new(50),
                TransactionType::DirectTransfer,
                Some("Test transaction".to_string()),
            );
            
            // Submit the transaction
            let result = processor.submit_transaction(transaction).await;
            assert!(result.is_ok());
            
            // Check that it's in the pending queue
            assert_eq!(processor.pending_transactions.len(), 1);
        });
    }
    
    #[test]
    fn test_process_transaction() {
        block_on(async {
            let (graph, mut processor) = create_test_environment();
            
            let mut graph_lock = graph.lock().await;
            
            // Create accounts
            let from_account = Account::new(
                DID::new("from"),
                "From Account".to_string(),
            );
            let to_account = Account::new(
                DID::new("to"),
                "To Account".to_string(),
            );
            
            graph_lock.add_account(from_account).await.unwrap();
            graph_lock.add_account(to_account).await.unwrap();
            
            // Create credit line
            let credit_line = CreditLine::new(
                DID::new("from"),
                DID::new("to"),
                Amount::new(100),
                Default::default(),
            );
            
            graph_lock.add_credit_line(credit_line).await.unwrap();
            
            drop(graph_lock);
            
            // Create transaction
            let transaction = Transaction::new(
                "test-tx".to_string(),
                DID::new("from"),
                DID::new("to"),
                Amount::new(50),
                TransactionType::DirectTransfer,
                Some("Test transaction".to_string()),
            );
            
            // Submit and process the transaction
            processor.submit_transaction(transaction).await.unwrap();
            let results = processor.process_pending_transactions().await;
            
            // Check that processing was successful
            assert_eq!(results.len(), 1);
            assert!(results[0].is_ok());
            
            // Check that account balances were updated
            let graph_lock = graph.lock().await;
            
            let from_account = graph_lock.get_account(&DID::new("from")).await.unwrap().unwrap();
            let to_account = graph_lock.get_account(&DID::new("to")).await.unwrap().unwrap();
            
            assert_eq!(from_account.balance, Amount::new(-50));
            assert_eq!(to_account.balance, Amount::new(50));
            
            // Check that the credit line was updated
            let credit_line_id = CreditLineId::new(&DID::new("from"), &DID::new("to"));
            let credit_line = graph_lock.get_credit_line(&credit_line_id).await.unwrap().unwrap();
            
            assert_eq!(credit_line.balance, Amount::new(-50));
        });
    }
} 