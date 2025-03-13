use std::error::Error;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use crate::crypto::{CryptoUtils, Keypair, PublicKey, Signature};
use crate::identity::Identity;

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
}

impl MutualCreditSystem {
    // Create a new mutual credit system
    pub fn new(identity: Identity, storage: crate::storage::Storage) -> Self {
        MutualCreditSystem {
            identity,
            storage,
        }
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
        };

        // Store the account
        self.storage.put_json(
            &format!("members/{}", member_did),
            &account,
        )?;

        Ok(account)
    }

    // Get member balance
    pub fn get_member_balance(&self, member_did: &str) -> Result<i64, Box<dyn Error>> {
        let account: MemberAccount = self.storage.get_json(&format!("members/{}", member_did))?;
        Ok(account.balance)
    }

    // Create a new transaction between members
    pub fn create_transaction(
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

        // Verify both members belong to this cooperative
        if !from_did.starts_with(&format!("did:icn:{}", self.identity.coop_id)) ||
           !to_did.starts_with(&format!("did:icn:{}", self.identity.coop_id)) {
            return Err(Box::new(EconomicError::InvalidMember(
                "Both members must belong to this cooperative".to_string(),
            )));
        }

        // Get sender's account
        let mut sender_account: MemberAccount = self.storage.get_json(
            &format!("members/{}", from_did),
        )?;

        // Check if sender has sufficient credit
        if sender_account.balance - amount < -sender_account.credit_limit {
            return Err(Box::new(EconomicError::InsufficientCredit(
                "Transaction would exceed credit limit".to_string(),
            )));
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        // Create transaction
        let transaction = Transaction {
            id: format!("tx-{}", now),
            from_did: from_did.to_string(),
            to_did: to_did.to_string(),
            amount,
            timestamp: now,
            description,
            signature: Vec::new(), // Will be signed below
            cooperative: self.identity.coop_id.clone(),
        };

        // Sign the transaction
        let tx_data = serde_json::to_vec(&transaction)?;
        let signature = self.identity.sign(&tx_data)?;
        let mut signed_tx = transaction;
        signed_tx.signature = signature.to_bytes().to_vec();

        // Update sender's account
        sender_account.balance -= amount;
        sender_account.last_updated = now;
        sender_account.transactions.push(signed_tx.clone());

        // Store updated account
        self.storage.put_json(
            &format!("members/{}", from_did),
            &sender_account,
        )?;

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
} 