//! Credit line management for the mutual credit system.

use crate::error::CreditError;
use crate::types::{Amount, DID, Timestamp};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// A credit line between two accounts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditLine {
    /// The account extending credit
    pub from_account: DID,
    /// The account receiving credit
    pub to_account: DID,
    /// The maximum amount of credit that can be extended
    pub limit: Amount,
    /// The current balance of the credit line
    pub balance: Amount,
    /// When the credit line was created
    pub created_at: Timestamp,
    /// When the credit line was last updated
    pub updated_at: Timestamp,
    /// Terms of the credit line
    pub terms: CreditTerms,
}

/// Terms of a credit line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditTerms {
    /// Interest rate (usually 0 in mutual credit systems)
    pub interest_rate: Decimal,
    /// Optional expiration date
    pub expiration: Option<Timestamp>,
    /// Whether the credit line auto-renews
    pub auto_renewal: bool,
    /// Additional conditions
    pub conditions: Vec<CreditCondition>,
}

/// Conditions for credit lines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CreditCondition {
    /// Minimum reputation score required
    MinimumReputation(f64),
    /// Minimum active participation time required
    ActiveParticipation(Duration),
    /// Requires governance approval
    GovernanceApproval,
    /// Requires collateral
    Collateral(CollateralRequirement),
    /// Requires reciprocal credit line
    ReciprocalCreditLine(Amount),
    /// Custom condition
    Custom(String),
}

/// Collateral requirement for a credit line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollateralRequirement {
    /// Type of collateral
    pub collateral_type: CollateralType,
    /// Amount of collateral required
    pub amount: Amount,
    /// Ratio of collateral to credit (e.g., 1.5 means 150% collateralization)
    pub ratio: Decimal,
}

/// Types of collateral
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollateralType {
    /// Credit from another account
    Credit(DID),
    /// Resource commitment
    Resource(ResourceCommitment),
    /// Governance token
    GovernanceToken,
    /// External asset
    ExternalAsset(String),
}

/// Resource commitment as collateral
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCommitment {
    /// Type of resource
    pub resource_type: String,
    /// Quantity of resource
    pub quantity: u64,
    /// Unit of measurement
    pub unit: String,
    /// Duration of commitment
    pub duration: Duration,
}

impl CreditLine {
    /// Create a new credit line
    pub fn new(
        from_account: DID,
        to_account: DID,
        limit: Amount,
        terms: CreditTerms,
    ) -> Self {
        Self {
            from_account,
            to_account,
            limit,
            balance: Amount::zero(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            terms,
        }
    }

    /// Check if the credit line is active
    pub fn is_active(&self) -> bool {
        if let Some(expiration) = self.terms.expiration {
            chrono::Utc::now() < expiration
        } else {
            true
        }
    }

    /// Check if a transaction would exceed the credit limit
    pub fn would_exceed_limit(&self, amount: &Amount) -> bool {
        let new_balance = self.balance.clone() + amount.clone();
        new_balance.abs() > self.limit
    }

    /// Update the balance of the credit line
    pub fn update_balance(&mut self, amount: Amount) -> Result<(), CreditError> {
        let new_balance = self.balance.clone() + amount;
        
        if new_balance.abs() > self.limit {
            return Err(CreditError::CreditLimitExceeded(format!(
                "Transaction would exceed credit limit of {}",
                self.limit
            )));
        }
        
        self.balance = new_balance;
        self.updated_at = chrono::Utc::now();
        
        Ok(())
    }

    /// Check if the credit line has available credit
    pub fn available_credit(&self) -> Amount {
        if self.balance.is_negative() {
            self.limit.clone() - self.balance.abs()
        } else {
            self.limit.clone()
        }
    }

    /// Update the credit limit
    pub fn update_limit(&mut self, new_limit: Amount) {
        self.limit = new_limit;
        self.updated_at = chrono::Utc::now();
    }

    /// Update the terms of the credit line
    pub fn update_terms(&mut self, new_terms: CreditTerms) {
        self.terms = new_terms;
        self.updated_at = chrono::Utc::now();
    }

    /// Extend the expiration date
    pub fn extend_expiration(&mut self, duration: Duration) {
        let new_expiration = if let Some(current_expiration) = self.terms.expiration {
            Some(current_expiration + chrono::Duration::from_std(duration).unwrap())
        } else {
            Some(chrono::Utc::now() + chrono::Duration::from_std(duration).unwrap())
        };
        
        self.terms.expiration = new_expiration;
        self.updated_at = chrono::Utc::now();
    }
}

impl CreditTerms {
    /// Create new default credit terms
    pub fn new() -> Self {
        Self {
            interest_rate: Decimal::new(0, 0),
            expiration: None,
            auto_renewal: true,
            conditions: Vec::new(),
        }
    }

    /// Create credit terms with an expiration date
    pub fn with_expiration(expiration: Timestamp) -> Self {
        Self {
            interest_rate: Decimal::new(0, 0),
            expiration: Some(expiration),
            auto_renewal: false,
            conditions: Vec::new(),
        }
    }

    /// Add a condition to the credit terms
    pub fn add_condition(&mut self, condition: CreditCondition) {
        self.conditions.push(condition);
    }

    /// Check if the credit terms have a specific condition type
    pub fn has_condition_type<F>(&self, predicate: F) -> bool
    where
        F: Fn(&CreditCondition) -> bool,
    {
        self.conditions.iter().any(predicate)
    }
}

impl Default for CreditTerms {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_credit_line_basics() {
        let from = DID::new("from");
        let to = DID::new("to");
        let limit = Amount::new(100);
        let terms = CreditTerms::new();
        
        let mut credit_line = CreditLine::new(from, to, limit, terms);
        
        assert!(credit_line.is_active());
        assert_eq!(credit_line.balance, Amount::zero());
        assert_eq!(credit_line.available_credit(), Amount::new(100));
        
        // Update balance
        credit_line.update_balance(Amount::new(30)).unwrap();
        assert_eq!(credit_line.balance, Amount::new(30));
        assert_eq!(credit_line.available_credit(), Amount::new(100));
        
        // Negative balance (credit extended)
        credit_line.update_balance(Amount::new(-50)).unwrap();
        assert_eq!(credit_line.balance, Amount::new(-20));
        assert_eq!(credit_line.available_credit(), Amount::new(80));
        
        // Would exceed limit
        assert!(credit_line.would_exceed_limit(&Amount::new(-90)));
        assert!(!credit_line.would_exceed_limit(&Amount::new(-70)));
        
        // Update limit
        credit_line.update_limit(Amount::new(200));
        assert_eq!(credit_line.limit, Amount::new(200));
        assert_eq!(credit_line.available_credit(), Amount::new(180));
    }

    #[test]
    fn test_credit_terms() {
        let mut terms = CreditTerms::new();
        assert_eq!(terms.interest_rate, Decimal::new(0, 0));
        assert!(terms.expiration.is_none());
        assert!(terms.auto_renewal);
        assert!(terms.conditions.is_empty());
        
        // Add conditions
        terms.add_condition(CreditCondition::MinimumReputation(0.7));
        terms.add_condition(CreditCondition::ActiveParticipation(Duration::from_secs(86400 * 30))); // 30 days
        
        assert_eq!(terms.conditions.len(), 2);
        assert!(terms.has_condition_type(|c| matches!(c, CreditCondition::MinimumReputation(_))));
        
        // With expiration
        let expiration = chrono::Utc::now() + chrono::Duration::days(90);
        let terms_with_expiration = CreditTerms::with_expiration(expiration);
        
        assert!(terms_with_expiration.expiration.is_some());
        assert!(!terms_with_expiration.auto_renewal);
    }

    #[test]
    fn test_credit_line_expiration() {
        let from = DID::new("from");
        let to = DID::new("to");
        let limit = Amount::new(100);
        
        // Create expired credit line
        let expired_date = chrono::Utc::now() - chrono::Duration::days(1);
        let expired_terms = CreditTerms::with_expiration(expired_date);
        let expired_line = CreditLine::new(from.clone(), to.clone(), limit.clone(), expired_terms);
        
        assert!(!expired_line.is_active());
        
        // Create active credit line with future expiration
        let future_date = chrono::Utc::now() + chrono::Duration::days(30);
        let active_terms = CreditTerms::with_expiration(future_date);
        let mut active_line = CreditLine::new(from, to, limit, active_terms);
        
        assert!(active_line.is_active());
        
        // Extend expiration
        active_line.extend_expiration(Duration::from_secs(86400 * 60)); // 60 more days
        
        if let Some(new_expiration) = active_line.terms.expiration {
            assert!(new_expiration > future_date);
            let difference = new_expiration - future_date;
            assert!(difference >= chrono::Duration::days(59)); // Allow for small timing differences
        } else {
            panic!("Expiration should be set");
        }
    }
} 