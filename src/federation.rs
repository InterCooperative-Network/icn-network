use std::error::Error;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use crate::crypto::{CryptoUtils, Keypair, PublicKey, Signature};
use crate::identity::Identity;
use crate::economic::{Transaction, MemberAccount};

// Federation error types
#[derive(Debug)]
pub enum FederationError {
    InvalidFederation(String),
    InvalidMember(String),
    TransactionFailed(String),
    VerificationFailed(String),
    StorageError(String),
    FederationNotFound(String),
}

impl fmt::Display for FederationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FederationError::InvalidFederation(msg) => write!(f, "Invalid federation: {}", msg),
            FederationError::InvalidMember(msg) => write!(f, "Invalid member: {}", msg),
            FederationError::TransactionFailed(msg) => write!(f, "Transaction failed: {}", msg),
            FederationError::VerificationFailed(msg) => write!(f, "Verification failed: {}", msg),
            FederationError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            FederationError::FederationNotFound(msg) => write!(f, "Federation not found: {}", msg),
        }
    }
}

impl Error for FederationError {}

// Federation structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Federation {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: u64,
    pub members: Vec<FederationMember>,
    pub policies: FederationPolicies,
}

// Federation member structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationMember {
    pub cooperative_id: String,
    pub node_id: String,
    pub joined_at: u64,
    pub status: FederationMemberStatus,
    pub credit_limit: i64,
}

// Federation member status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FederationMemberStatus {
    Active,
    Suspended,
    Expelled,
}

// Federation policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPolicies {
    pub max_transaction_amount: i64,
    pub min_transaction_amount: i64,
    pub max_credit_limit: i64,
    pub min_credit_limit: i64,
    pub transaction_fee: i64,
    pub settlement_period: u64, // in seconds
}

// Federation transaction structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationTransaction {
    pub id: String,
    pub federation_id: String,
    pub from_did: String,
    pub to_did: String,
    pub from_coop: String,
    pub to_coop: String,
    pub amount: i64,
    pub timestamp: u64,
    pub description: Option<String>,
    pub signature: Vec<u8>,
    pub status: FederationTransactionStatus,
}

// Federation transaction status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FederationTransactionStatus {
    Pending,
    Approved,
    Rejected,
    Completed,
    Failed,
}

// Federation system
pub struct FederationSystem {
    identity: Identity,
    storage: crate::storage::Storage,
}

impl FederationSystem {
    // Create a new federation system
    pub fn new(identity: Identity, storage: crate::storage::Storage) -> Self {
        FederationSystem {
            identity,
            storage,
        }
    }

    // Create a new federation
    pub fn create_federation(
        &self,
        name: &str,
        description: Option<String>,
        policies: FederationPolicies,
    ) -> Result<Federation, Box<dyn Error>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        let federation = Federation {
            id: format!("fed-{}", now),
            name: name.to_string(),
            description,
            created_at: now,
            members: vec![FederationMember {
                cooperative_id: self.identity.coop_id.clone(),
                node_id: self.identity.node_id.clone(),
                joined_at: now,
                status: FederationMemberStatus::Active,
                credit_limit: policies.max_credit_limit,
            }],
            policies,
        };

        // Store the federation
        self.storage.put_json(
            &format!("federations/{}", federation.id),
            &federation,
        )?;

        Ok(federation)
    }

    // Join an existing federation
    pub fn join_federation(
        &self,
        federation_id: &str,
        credit_limit: i64,
    ) -> Result<(), Box<dyn Error>> {
        let mut federation: Federation = self.storage.get_json(
            &format!("federations/{}", federation_id),
        )?;

        // Verify credit limit is within federation policies
        if credit_limit > federation.policies.max_credit_limit {
            return Err(Box::new(FederationError::InvalidFederation(
                "Credit limit exceeds federation maximum".to_string(),
            )));
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        let member = FederationMember {
            cooperative_id: self.identity.coop_id.clone(),
            node_id: self.identity.node_id.clone(),
            joined_at: now,
            status: FederationMemberStatus::Active,
            credit_limit,
        };

        federation.members.push(member);
        self.storage.put_json(
            &format!("federations/{}", federation_id),
            &federation,
        )?;

        Ok(())
    }

    // Create a federation transaction
    pub fn create_federation_transaction(
        &self,
        federation_id: &str,
        from_did: &str,
        to_did: &str,
        amount: i64,
        description: Option<String>,
    ) -> Result<FederationTransaction, Box<dyn Error>> {
        // Get federation
        let federation: Federation = self.storage.get_json(
            &format!("federations/{}", federation_id),
        )?;

        // Verify amount is within federation policies
        if amount < federation.policies.min_transaction_amount {
            return Err(Box::new(FederationError::InvalidFederation(
                "Amount below federation minimum".to_string(),
            )));
        }
        if amount > federation.policies.max_transaction_amount {
            return Err(Box::new(FederationError::InvalidFederation(
                "Amount exceeds federation maximum".to_string(),
            )));
        }

        // Extract cooperative IDs from DIDs
        let from_coop = from_did.split(':').nth(2).ok_or_else(|| {
            FederationError::InvalidMember("Invalid from_did format".to_string())
        })?;
        let to_coop = to_did.split(':').nth(2).ok_or_else(|| {
            FederationError::InvalidMember("Invalid to_did format".to_string())
        })?;

        // Verify both cooperatives are federation members
        if !federation.members.iter().any(|m| m.cooperative_id == from_coop) ||
           !federation.members.iter().any(|m| m.cooperative_id == to_coop) {
            return Err(Box::new(FederationError::InvalidMember(
                "One or both cooperatives are not federation members".to_string(),
            )));
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        let transaction = FederationTransaction {
            id: format!("ftx-{}", now),
            federation_id: federation_id.to_string(),
            from_did: from_did.to_string(),
            to_did: to_did.to_string(),
            from_coop: from_coop.to_string(),
            to_coop: to_coop.to_string(),
            amount,
            timestamp: now,
            description,
            signature: Vec::new(), // Will be signed below
            status: FederationTransactionStatus::Pending,
        };

        // Sign the transaction
        let tx_data = serde_json::to_vec(&transaction)?;
        let signature = self.identity.sign(&tx_data)?;
        let mut signed_tx = transaction;
        signed_tx.signature = signature.to_bytes().to_vec();

        // Store the transaction
        self.storage.put_json(
            &format!("federation_transactions/{}", signed_tx.id),
            &signed_tx,
        )?;

        Ok(signed_tx)
    }

    // Process a federation transaction
    pub fn process_federation_transaction(
        &self,
        transaction: &FederationTransaction,
    ) -> Result<(), Box<dyn Error>> {
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
            return Err(Box::new(FederationError::VerificationFailed(
                "Invalid transaction signature".to_string(),
            )));
        }

        // Get federation
        let federation: Federation = self.storage.get_json(
            &format!("federations/{}", transaction.federation_id),
        )?;

        // Apply transaction fee
        let total_amount = transaction.amount + federation.policies.transaction_fee;

        // Update sender's account
        let mut sender_account: MemberAccount = self.storage.get_json(
            &format!("members/{}", transaction.from_did),
        )?;

        if sender_account.balance - total_amount < -sender_account.credit_limit {
            return Err(Box::new(FederationError::TransactionFailed(
                "Insufficient credit".to_string(),
            )));
        }

        sender_account.balance -= total_amount;
        sender_account.last_updated = transaction.timestamp;
        sender_account.transactions.push(Transaction {
            id: transaction.id.clone(),
            from_did: transaction.from_did.clone(),
            to_did: transaction.to_did.clone(),
            amount: transaction.amount,
            timestamp: transaction.timestamp,
            description: transaction.description.clone(),
            signature: transaction.signature.clone(),
            cooperative: transaction.from_coop.clone(),
        });

        // Update receiver's account
        let mut receiver_account: MemberAccount = self.storage.get_json(
            &format!("members/{}", transaction.to_did),
        )?;

        receiver_account.balance += transaction.amount;
        receiver_account.last_updated = transaction.timestamp;
        receiver_account.transactions.push(Transaction {
            id: transaction.id.clone(),
            from_did: transaction.from_did.clone(),
            to_did: transaction.to_did.clone(),
            amount: transaction.amount,
            timestamp: transaction.timestamp,
            description: transaction.description.clone(),
            signature: transaction.signature.clone(),
            cooperative: transaction.to_coop.clone(),
        });

        // Store updated accounts
        self.storage.put_json(
            &format!("members/{}", transaction.from_did),
            &sender_account,
        )?;
        self.storage.put_json(
            &format!("members/{}", transaction.to_did),
            &receiver_account,
        )?;

        // Update transaction status
        let mut updated_tx = transaction.clone();
        updated_tx.status = FederationTransactionStatus::Completed;
        self.storage.put_json(
            &format!("federation_transactions/{}", updated_tx.id),
            &updated_tx,
        )?;

        Ok(())
    }

    // Get federation transactions
    pub fn get_federation_transactions(
        &self,
        federation_id: &str,
    ) -> Result<Vec<FederationTransaction>, Box<dyn Error>> {
        let transactions: Vec<String> = self.storage.list_keys("federation_transactions/")?;
        let mut federation_transactions = Vec::new();
        
        for tx_id in transactions {
            let transaction: FederationTransaction = self.storage.get_json(
                &format!("federation_transactions/{}", tx_id),
            )?;
            if transaction.federation_id == federation_id {
                federation_transactions.push(transaction);
            }
        }
        
        Ok(federation_transactions)
    }

    // Get federation members
    pub fn get_federation_members(
        &self,
        federation_id: &str,
    ) -> Result<Vec<FederationMember>, Box<dyn Error>> {
        let federation: Federation = self.storage.get_json(
            &format!("federations/{}", federation_id),
        )?;
        Ok(federation.members)
    }
} 