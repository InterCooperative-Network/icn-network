use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use crate::identity::Identity;
use crate::storage::Storage;
use ed25519_dalek::Verifier;

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
    identity: Arc<Identity>,
    storage: Arc<Storage>,
}

impl FederationSystem {
    // Create a new federation system
    pub fn new(
        identity: Arc<Identity>,
        storage: Arc<Storage>,
        _economic: Arc<dyn std::any::Any>,
    ) -> Self {
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

        let mut transaction = FederationTransaction {
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
        let tx_data = serde_json::to_string(&transaction)?;
        let signature = self.identity.sign(tx_data.as_bytes())?;
        transaction.signature = signature.to_bytes().to_vec();

        // Store the transaction
        self.storage.put_json(
            &format!("transactions/federation/{}", transaction.id),
            &transaction,
        )?;

        Ok(transaction)
    }
    
    // Process a federation transaction
    pub fn process_federation_transaction(
        &self,
        transaction: &FederationTransaction,
    ) -> Result<(), Box<dyn Error>> {
        // Get federation
        let federation: Federation = self.storage.get_json(
            &format!("federations/{}", transaction.federation_id),
        )?;
        
        // Verify both cooperatives are federation members
        if !federation.members.iter().any(|m| m.cooperative_id == transaction.from_coop) ||
           !federation.members.iter().any(|m| m.cooperative_id == transaction.to_coop) {
            return Err(Box::new(FederationError::InvalidMember(
                "One or both cooperatives are not federation members".to_string(),
            )));
        }
        
        // Verify signature
        let mut verification_tx = transaction.clone();
        verification_tx.signature = Vec::new();
        
        let tx_data = serde_json::to_string(&verification_tx)?;
        
        // Get the sender's identity
        let sender_did_doc = self.identity.resolve_did(&transaction.from_did)?;
        let sender_key_id = format!("{}#keys-1", transaction.from_did);
        
        let sender_key = sender_did_doc.verification_method.iter()
            .find(|vm| vm.id == sender_key_id)
            .ok_or_else(|| FederationError::VerificationFailed(
                "Sender key not found".to_string(),
            ))?;
            
        // Parse the public key
        let public_key_bytes = bs58::decode(&sender_key.public_key_multibase[1..])
            .into_vec()
            .map_err(|_| FederationError::VerificationFailed(
                "Failed to decode sender public key".to_string(),
            ))?;
            
        let public_key = ed25519_dalek::PublicKey::from_bytes(&public_key_bytes)
            .map_err(|_| FederationError::VerificationFailed(
                "Invalid sender public key".to_string(),
            ))?;
            
        let signature = ed25519_dalek::Signature::from_bytes(&transaction.signature)
            .map_err(|_| FederationError::VerificationFailed(
                "Invalid signature format".to_string(),
            ))?;
            
        // Verify the signature
        if !public_key.verify(tx_data.as_bytes(), &signature).is_ok() {
            return Err(Box::new(FederationError::VerificationFailed(
                "Invalid transaction signature".to_string(),
            )));
        }
        
        // In a real implementation, we would update account balances
        // But for our simplified example, we just update the status
        
        let mut updated_tx = transaction.clone();
        updated_tx.status = FederationTransactionStatus::Completed;
        self.storage.put_json(
            &format!("transactions/federation/{}", updated_tx.id),
            &updated_tx,
        )?;
        
        Ok(())
    }
    
    // Get federation by ID
    pub fn get_federation(&self, federation_id: &str) -> Result<Federation, Box<dyn Error>> {
        let federation: Federation = self.storage.get_json(
            &format!("federations/{}", federation_id),
        )?;
        
        Ok(federation)
    }
    
    // List all federations
    pub fn list_federations(&self) -> Result<Vec<Federation>, Box<dyn Error>> {
        let federation_keys = self.storage.list("federations/")?;
        let mut federations = Vec::new();
        
        for key in federation_keys {
            let federation: Federation = self.storage.get_json(&key)?;
            federations.push(federation);
        }
        
        Ok(federations)
    }
    
    // Get federation transactions
    pub fn get_federation_transactions(
        &self,
        federation_id: &str,
    ) -> Result<Vec<FederationTransaction>, Box<dyn Error>> {
        let transactions_keys = self.storage.list("transactions/federation/")?;
        let mut transactions = Vec::new();
        
        for key in transactions_keys {
            let transaction: FederationTransaction = self.storage.get_json(&key)?;
            if transaction.federation_id == federation_id {
                transactions.push(transaction);
            }
        }
        
        Ok(transactions)
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