use std::error::Error;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use crate::crypto::{CryptoUtils, Keypair, PublicKey, Signature};
use crate::identity::Identity;
use std::sync::Arc;

// Economic error types
#[derive(Debug)]
pub enum EconomicError {
    InvalidAmount(String),
    InsufficientCredit(String),
    InvalidTransaction(String),
    VerificationFailed(String),
    StorageError(String),
    MemberNotFound(String),
    InvalidMember(String),
}

impl fmt::Display for EconomicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EconomicError::InvalidAmount(msg) => write!(f, "Invalid amount: {}", msg),
            EconomicError::InsufficientCredit(msg) => write!(f, "Insufficient credit: {}", msg),
            EconomicError::InvalidTransaction(msg) => write!(f, "Invalid transaction: {}", msg),
            EconomicError::VerificationFailed(msg) => write!(f, "Verification failed: {}", msg),
            EconomicError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            EconomicError::MemberNotFound(msg) => write!(f, "Member not found: {}", msg),
            EconomicError::InvalidMember(msg) => write!(f, "Invalid member: {}", msg),
        }
    }
}

impl Error for EconomicError {}

// Member account structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberAccount {
    pub did: String,
    pub balance: i64,
    pub credit_limit: i64,
    pub last_updated: u64,
    pub transactions: Vec<Transaction>,
    pub cooperative: String, // The cooperative this member belongs to
    pub reputation_score: Option<f64>, // Optional reputation score from the reputation system
}

// Transaction structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub from_did: String,
    pub to_did: String,
    pub amount: i64,
    pub timestamp: u64,
    pub description: Option<String>,
    pub signature: Vec<u8>,
    pub cooperative: String, // The cooperative facilitating this transaction
}

// Mutual credit system
pub struct MutualCreditSystem {
    identity: Identity,
    storage: crate::storage::Storage,
    crypto: Arc<CryptoUtils>,
    reputation: Option<Arc<crate::reputation::ReputationSystem>>,
}

impl MutualCreditSystem {
    // Create a new mutual credit system
    pub fn new(identity: Identity, storage: crate::storage::Storage, crypto: Arc<CryptoUtils>) -> Self {
        MutualCreditSystem {
            identity,
            storage,
            crypto,
            reputation: None,
        }
    }
    
    // Set the reputation system reference (called after initialization)
    pub fn set_reputation_system(&mut self, reputation: Arc<crate::reputation::ReputationSystem>) {
        self.reputation = Some(reputation);
    }

    // Register a new member account
    pub fn register_member(&self, member_did: &str, credit_limit: i64) -> Result<MemberAccount, Box<dyn Error>> {
        if credit_limit <= 0 {
            return Err(Box::new(EconomicError::InvalidAmount(
                "Credit limit must be positive".to_string(),
            )));
        }

        // Verify the member belongs to this cooperative
        if !member_did.starts_with(&format!("did:icn:{}", self.identity.coop_id)) {
            return Err(Box::new(EconomicError::InvalidMember(
                "Member must belong to this cooperative".to_string(),
            )));
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        let account = MemberAccount {
            did: member_did.to_string(),
            balance: 0,
            credit_limit,
            last_updated: now,
            transactions: Vec::new(),
            cooperative: self.identity.coop_id.clone(),
            reputation_score: None, // New members start with no reputation
        };

        // Store the account
        self.storage.store_json(&format!("accounts/{}", member_did), &account)?;

        Ok(account)
    }

    // Get member balance
    pub fn get_member_balance(&self, member_did: &str) -> Result<i64, Box<dyn Error>> {
        let account: MemberAccount = self.storage.get_json(&format!("members/{}", member_did))?;
        Ok(account.balance)
    }

    // Create a new transaction
    pub async fn create_transaction(
        &self,
        from_did: &str,
        to_did: &str,
        amount: i64,
        description: Option<String>,
    ) -> Result<Transaction, Box<dyn Error>> {
        if amount <= 0 {
            return Err(Box::new(EconomicError::InvalidAmount(
                "Amount must be positive".to_string(),
            )));
        }

        // Get accounts with up-to-date reputation scores
        let mut from_account = self.get_account_with_reputation(from_did).await?;
        let mut to_account = self.get_account_with_reputation(to_did).await?;

        // Check if sender has sufficient credit
        if from_account.balance - amount < -from_account.credit_limit {
            return Err(Box::new(EconomicError::InsufficientCredit(
                format!(
                    "Insufficient credit: balance={}, amount={}, credit_limit={}",
                    from_account.balance, amount, from_account.credit_limit
                ),
            )));
        }
        
        // Additional reputation-based validations
        if let Some(reputation_system) = &self.reputation {
            // For larger transactions, check if sender has sufficient reputation
            let large_transaction_threshold = 500; // Configurable threshold
            
            if amount > large_transaction_threshold {
                let trust_score = reputation_system.calculate_trust_score(from_did)?;
                
                // For large transactions, require a minimum trust score
                let min_trust_for_large_tx = 0.6;
                if trust_score.overall_score < min_trust_for_large_tx {
                    return Err(Box::new(EconomicError::InsufficientCredit(
                        format!(
                            "Insufficient reputation for large transaction: score={}, required={}",
                            trust_score.overall_score, min_trust_for_large_tx
                        ),
                    )));
                }
                
                // Check for potential Sybil risk
                let sybil_indicators = reputation_system.sybil_resistance().check_sybil_indicators(from_did)?;
                let max_sybil_risk_for_large_tx = 0.3;
                
                if sybil_indicators.risk_score > max_sybil_risk_for_large_tx {
                    return Err(Box::new(EconomicError::VerificationFailed(
                        format!(
                            "High Sybil risk for large transaction: risk={}, max_allowed={}",
                            sybil_indicators.risk_score, max_sybil_risk_for_large_tx
                        ),
                    )));
                }
            }
        }

        // Create transaction
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        let transaction = Transaction {
            id: format!("tx:{}:{}:{}", from_did, to_did, now),
            from_did: from_did.to_string(),
            to_did: to_did.to_string(),
            amount,
            timestamp: now,
            description,
            signature: Vec::new(), // Will be filled in after signing
            cooperative: self.identity.coop_id.clone(),
        };

        // Sign the transaction
        let tx_data = serde_json::to_vec(&transaction)?;
        let signature = self.crypto.sign(&tx_data)?;
        let mut signed_tx = transaction;
        signed_tx.signature = signature.to_bytes().to_vec();

        // Update account balances
        from_account.balance -= amount;
        to_account.balance += amount;
        from_account.last_updated = now;
        to_account.last_updated = now;

        // Add transaction to accounts
        from_account.transactions.push(signed_tx.clone());
        to_account.transactions.push(signed_tx.clone());

        // Store updated accounts
        self.storage.store_json(&format!("accounts/{}", from_did), &from_account)?;
        self.storage.store_json(&format!("accounts/{}", to_did), &to_account)?;

        // If reputation system is available, create transaction attestation
        if let Some(reputation_system) = &self.reputation {
            // Create a transaction attestation for successful transaction
            // This helps build reputation over time based on transaction history
            let evidence = vec![
                crate::reputation::Evidence {
                    evidence_type: "transaction".to_string(),
                    evidence_id: signed_tx.id.clone(),
                    description: format!("Transaction: {} -> {}, amount: {}", from_did, to_did, amount),
                    timestamp: now,
                    data: Some(serde_json::to_value(&signed_tx)?),
                }
            ];
            
            // Success transaction boosts reputation slightly
            let _ = reputation_system.attestation_manager().create_attestation(
                from_did,
                crate::reputation::AttestationType::TransactionTrust,
                0.6, // Modest positive score
                serde_json::json!({ "transaction_success": true }),
                evidence.clone(),
                1, // Single-party attestation
                Some(90), // 90 day validity
            );
            
            let _ = reputation_system.attestation_manager().create_attestation(
                to_did,
                crate::reputation::AttestationType::TransactionTrust,
                0.6, // Modest positive score
                serde_json::json!({ "transaction_recipient": true }),
                evidence,
                1, // Single-party attestation
                Some(90), // 90 day validity
            );
        }

        Ok(signed_tx)
    }

    // Process a received transaction
    pub fn process_transaction(&self, transaction: &Transaction) -> Result<(), Box<dyn Error>> {
        // Verify transaction signature
        let mut verification_tx = transaction.clone();
        verification_tx.signature.clear();
        let tx_data = serde_json::to_vec(&verification_tx)?;
        
        let signature = ed25519_dalek::Signature::from_bytes(&transaction.signature)?;
        let from_did = &transaction.from_did;
        
        // In a real implementation, we would resolve the sender's DID to get their public key
        // For now, we'll assume we have it
        let sender_account: MemberAccount = self.storage.get_json(&format!("members/{}", from_did))?;
        let public_key = ed25519_dalek::PublicKey::from_bytes(
            &bs58::decode(&sender_account.did).into_vec()?,
        )?;

        if !public_key.verify(&tx_data, &signature).is_ok() {
            return Err(Box::new(EconomicError::VerificationFailed(
                "Invalid transaction signature".to_string(),
            )));
        }

        // Update receiver's account
        let mut receiver_account: MemberAccount = self.storage.get_json(
            &format!("members/{}", transaction.to_did),
        )?;

        receiver_account.balance += transaction.amount;
        receiver_account.last_updated = transaction.timestamp;
        receiver_account.transactions.push(transaction.clone());

        // Store updated account
        self.storage.put_json(
            &format!("members/{}", transaction.to_did),
            &receiver_account,
        )?;

        Ok(())
    }

    // Get member's transaction history
    pub fn get_member_transaction_history(&self, member_did: &str) -> Result<Vec<Transaction>, Box<dyn Error>> {
        let account: MemberAccount = self.storage.get_json(&format!("members/{}", member_did))?;
        Ok(account.transactions)
    }

    // Get all members in this cooperative
    pub fn get_cooperative_members(&self) -> Result<Vec<MemberAccount>, Box<dyn Error>> {
        let members: Vec<String> = self.storage.list_keys("members/")?;
        let mut accounts = Vec::new();
        
        for member_did in members {
            if member_did.starts_with(&format!("did:icn:{}", self.identity.coop_id)) {
                let account: MemberAccount = self.storage.get_json(&format!("members/{}", member_did))?;
                accounts.push(account);
            }
        }
        
        Ok(accounts)
    }

    // Update a member's credit limit based on their reputation score
    pub async fn update_credit_limit_from_reputation(&self, member_did: &str) -> Result<MemberAccount, Box<dyn Error>> {
        // Check if we have a reputation system available
        if let Some(reputation_system) = &self.reputation {
            // Calculate trust score
            let trust_score = reputation_system.calculate_trust_score(member_did)?;
            
            // Calculate new credit limit based on reputation
            // Base credit limit is multiplied by reputation factor
            let base_credit_limit = 1000; // Default base limit
            let reputation_factor = 1.0 + trust_score.overall_score; // 1.0 to 2.0 multiplier
            
            // Apply anti-Sybil protection factor
            let sybil_indicators = reputation_system.sybil_resistance().check_sybil_indicators(member_did)?;
            let sybil_factor = 1.0 - (sybil_indicators.risk_score * 0.8); // 0.2 to 1.0 multiplier
            
            // Calculate final credit limit
            let new_credit_limit = (base_credit_limit as f64 * reputation_factor * sybil_factor) as i64;
            
            // Load current account
            let mut account: MemberAccount = self.storage.load_json(&format!("accounts/{}", member_did))?;
            
            // Update credit limit and reputation score
            account.credit_limit = new_credit_limit;
            account.reputation_score = Some(trust_score.overall_score);
            account.last_updated = SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs();
            
            // Store updated account
            self.storage.store_json(&format!("accounts/{}", member_did), &account)?;
            
            Ok(account)
        } else {
            // If no reputation system is available, just return the current account
            let account: MemberAccount = self.storage.load_json(&format!("accounts/{}", member_did))?;
            Ok(account)
        }
    }
    
    // Get a member account with up-to-date reputation
    pub async fn get_account_with_reputation(&self, member_did: &str) -> Result<MemberAccount, Box<dyn Error>> {
        // Try to get account with updated reputation
        if let Some(reputation_system) = &self.reputation {
            let result = self.update_credit_limit_from_reputation(member_did).await;
            if result.is_ok() {
                return result;
            }
        }
        
        // Fall back to regular account lookup
        let account: MemberAccount = self.storage.load_json(&format!("accounts/{}", member_did))?;
        Ok(account)
    }
} 