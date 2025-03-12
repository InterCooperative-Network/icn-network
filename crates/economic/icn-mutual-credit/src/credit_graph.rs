//! Credit graph implementation for the mutual credit system.

use crate::account::{Account, AccountStatus};
use crate::credit_line::CreditLine;
use crate::error::CreditError;
use crate::transaction::{Transaction, TransactionStatus, TransactionType};
use crate::types::{Amount, DID, Timestamp};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Unique identifier for a credit line
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CreditLineId {
    /// The account extending credit
    pub from: DID,
    /// The account receiving credit
    pub to: DID,
}

impl CreditLineId {
    /// Create a new credit line ID
    pub fn new(from: &DID, to: &DID) -> Self {
        Self {
            from: from.clone(),
            to: to.clone(),
        }
    }
}

impl fmt::Display for CreditLineId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}→{}", self.from, self.to)
    }
}

/// A step in a transaction path
#[derive(Debug, Clone)]
pub struct CreditLineStep {
    /// The sender in this step
    pub from: DID,
    /// The receiver in this step
    pub to: DID,
    /// The amount transferred in this step
    pub amount: Amount,
    /// The credit line ID for this step
    pub credit_line_id: CreditLineId,
}

/// The credit graph for the mutual credit system
#[derive(Debug)]
pub struct CreditGraph {
    /// All accounts in the system
    accounts: HashMap<DID, Account>,
    /// All credit lines in the system
    credit_lines: HashMap<CreditLineId, CreditLine>,
    /// All transactions in the system
    transactions: Vec<Transaction>,
}

impl CreditGraph {
    /// Create a new credit graph
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            credit_lines: HashMap::new(),
            transactions: Vec::new(),
        }
    }

    /// Add an account to the graph
    pub async fn add_account(&mut self, account: Account) -> Result<(), CreditError> {
        if self.accounts.contains_key(&account.did) {
            return Err(CreditError::AccountAlreadyExists(account.did.to_string()));
        }
        
        self.accounts.insert(account.did.clone(), account);
        Ok(())
    }

    /// Add a credit line to the graph
    pub async fn add_credit_line(&mut self, credit_line: CreditLine) -> Result<(), CreditError> {
        let id = CreditLineId::new(&credit_line.from_account, &credit_line.to_account);
        
        if self.credit_lines.contains_key(&id) {
            return Err(CreditError::CreditLineAlreadyExists(id.to_string()));
        }
        
        // Verify both accounts exist
        self.verify_account_exists(&credit_line.from_account).await?;
        self.verify_account_exists(&credit_line.to_account).await?;
        
        self.credit_lines.insert(id, credit_line);
        Ok(())
    }

    /// Add a transaction to the graph
    pub async fn add_transaction(&mut self, transaction: Transaction) -> Result<(), CreditError> {
        // Verify accounts exist and are active
        self.verify_account_active(&transaction.from).await?;
        self.verify_account_active(&transaction.to).await?;
        
        // For direct transfers, verify credit line exists
        if transaction.transaction_type == TransactionType::DirectTransfer {
            let credit_line_id = CreditLineId::new(&transaction.from, &transaction.to);
            
            if !self.credit_lines.contains_key(&credit_line_id) {
                return Err(CreditError::CreditLineNotFound(format!(
                    "Credit line from {} to {} not found",
                    transaction.from, transaction.to
                )));
            }
        }
        
        self.transactions.push(transaction);
        Ok(())
    }

    /// Process a transaction
    pub async fn process_transaction(&mut self, transaction: &mut Transaction) -> Result<(), CreditError> {
        // Verify accounts exist and are active
        self.verify_account_active(&transaction.from).await?;
        self.verify_account_active(&transaction.to).await?;
        
        match transaction.transaction_type {
            TransactionType::DirectTransfer => {
                // Get the credit line
                let credit_line_id = CreditLineId::new(&transaction.from, &transaction.to);
                let credit_line = self.credit_lines.get_mut(&credit_line_id)
                    .ok_or_else(|| CreditError::CreditLineNotFound(format!(
                        "Credit line from {} to {} not found",
                        transaction.from, transaction.to
                    )))?;
                
                // Check if the credit line is active
                if !credit_line.is_active() {
                    return Err(CreditError::InactiveCredit(format!(
                        "Credit line from {} to {} is inactive",
                        transaction.from, transaction.to
                    )));
                }
                
                // Update the credit line balance
                credit_line.update_balance(transaction.amount.clone())?;
                
                // Update account balances
                if let Some(from_account) = self.accounts.get_mut(&transaction.from) {
                    from_account.update_balance(-transaction.amount.clone());
                }
                
                if let Some(to_account) = self.accounts.get_mut(&transaction.to) {
                    to_account.update_balance(transaction.amount.clone());
                }
                
                // Mark transaction as completed
                transaction.complete();
                
                Ok(())
            },
            TransactionType::PathTransfer => {
                // Implement path transfer logic
                // This would involve finding a path and executing multiple transfers
                Err(CreditError::NotImplemented("Path transfers not yet implemented".to_string()))
            },
            TransactionType::CreditLineAdjustment => {
                // Implement credit line adjustment logic
                Err(CreditError::NotImplemented("Credit line adjustments not yet implemented".to_string()))
            },
            TransactionType::SystemOperation => {
                // Implement system operation logic
                Err(CreditError::NotImplemented("System operations not yet implemented".to_string()))
            },
        }
    }

    /// Find a transaction path between accounts
    pub async fn find_transaction_path(
        &self,
        from: &DID,
        to: &DID,
        amount: &Amount,
    ) -> Result<Vec<CreditLineStep>, CreditError> {
        // Verify accounts exist
        self.verify_account_exists(from).await?;
        self.verify_account_exists(to).await?;
        
        // Simple implementation - just check for direct path
        let credit_line_id = CreditLineId::new(from, to);
        
        if let Some(credit_line) = self.credit_lines.get(&credit_line_id) {
            if credit_line.is_active() && !credit_line.would_exceed_limit(amount) {
                let step = CreditLineStep {
                    from: from.clone(),
                    to: to.clone(),
                    amount: amount.clone(),
                    credit_line_id,
                };
                
                return Ok(vec![step]);
            }
        }
        
        // In a real implementation, we would use a pathfinding algorithm here
        // to find a path through the credit network
        
        Err(CreditError::NoPathFound(format!(
            "No path found from {} to {} for amount {}",
            from, to, amount
        )))
    }

    /// Get the balance for an account
    pub async fn get_account_balance(&self, account: &DID) -> Result<Amount, CreditError> {
        let account = self.accounts.get(account)
            .ok_or_else(|| CreditError::AccountNotFound(account.to_string()))?;
        
        Ok(account.balance.clone())
    }

    /// Get the transaction history for an account
    pub async fn get_transaction_history(&self, account: &DID) -> Result<Vec<&Transaction>, CreditError> {
        self.verify_account_exists(account).await?;
        
        let transactions = self.transactions.iter()
            .filter(|tx| &tx.from == account || &tx.to == account)
            .collect();
        
        Ok(transactions)
    }

    /// Verify an account exists
    pub async fn verify_account_exists(&self, account: &DID) -> Result<(), CreditError> {
        if !self.accounts.contains_key(account) {
            return Err(CreditError::AccountNotFound(account.to_string()));
        }
        
        Ok(())
    }

    /// Verify an account is active
    pub async fn verify_account_active(&self, account: &DID) -> Result<(), CreditError> {
        let account = self.accounts.get(account)
            .ok_or_else(|| CreditError::AccountNotFound(account.to_string()))?;
        
        if account.status != AccountStatus::Active {
            return Err(CreditError::InactiveAccount(format!(
                "Account {} is not active", account.did
            )));
        }
        
        Ok(())
    }

    /// Get a read-only account by DID
    pub async fn get_account(&self, account: &DID) -> Result<Option<&Account>, CreditError> {
        Ok(self.accounts.get(account))
    }

    /// Get a mutable account by DID
    pub async fn get_account_mut(&mut self, account: &DID) -> Result<Option<&mut Account>, CreditError> {
        Ok(self.accounts.get_mut(account))
    }

    /// Get all accounts in the system
    pub async fn get_all_accounts(&self) -> Result<Vec<&Account>, CreditError> {
        Ok(self.accounts.values().collect())
    }

    /// Get a read-only credit line by ID
    pub async fn get_credit_line(&self, id: &CreditLineId) -> Result<Option<&CreditLine>, CreditError> {
        Ok(self.credit_lines.get(id))
    }

    /// Get a mutable credit line by ID
    pub async fn get_credit_line_mut(&mut self, id: &CreditLineId) -> Result<Option<&mut CreditLine>, CreditError> {
        Ok(self.credit_lines.get_mut(id))
    }

    /// Get all credit lines in the system
    pub async fn get_all_credit_lines(&self) -> Result<Vec<&CreditLine>, CreditError> {
        Ok(self.credit_lines.values().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_add_account() {
        let mut graph = CreditGraph::new();
        let account = Account::new(DID::new("test"), "Test Account".to_string());
        
        // Add account
        assert!(graph.add_account(account.clone()).await.is_ok());
        
        // Try to add the same account again
        assert!(graph.add_account(account).await.is_err());
    }
} 