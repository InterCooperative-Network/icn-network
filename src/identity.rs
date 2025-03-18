use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;
use rand::rngs::OsRng;
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, Signer, Verifier};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use icn_core::storage::Storage;
use hex;
use bs58;
use std::collections::HashMap;
use std::sync::RwLock;

// DID Method specific to ICN
const DID_METHOD: &str = "icn";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    pub id: String,
    pub issuer: String,
    pub subject: String,
    pub claims: serde_json::Value,
    pub signature: Option<String>,
    pub issued_at: u64,
    pub expires_at: Option<u64>,
}

// Identity error types
#[derive(Debug)]
pub enum IdentityError {
    InvalidDid(String),
    CryptoError(String),
    StorageError(String),
    ValidationError(String),
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdentityError::InvalidDid(msg) => write!(f, "Invalid DID: {}", msg),
            IdentityError::CryptoError(msg) => write!(f, "Crypto error: {}", msg),
            IdentityError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            IdentityError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl Error for IdentityError {}

// DID Document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidDocument {
    pub id: String,
    pub context: Vec<String>,
    pub verification_method: Vec<VerificationMethod>,
    pub authentication: Vec<String>,
    pub assertion_method: Vec<String>,
    pub capability_invocation: Vec<String>,
    pub capability_delegation: Vec<String>,
    pub key_agreement: Vec<String>,
    pub service: Vec<Service>,
}

// Verification method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMethod {
    pub id: String,
    pub controller: String,
    pub type_: String,
    pub public_key_multibase: String,
}

// Service definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: String,
    pub type_: String,
    pub service_endpoint: String,
}

// Identity with DID
pub struct Identity {
    pub coop_id: String,
    pub node_id: String,
    pub did: String,
    pub document: DidDocument,
    pub keypair: Keypair,
    pub listen_addr: String,
    pub tls: bool,
    storage: Arc<dyn Storage>,
}

impl Identity {
    // Create a new identity
    pub fn new(
        coop_id: String, 
        node_id: String, 
        did: String, 
        storage: Arc<dyn Storage>
    ) -> Result<Self, Box<dyn Error>> {
        // Generate keypair for identity
        let keypair = CryptoUtils::generate_keypair()?;
        
        // Set default listening address
        let listen_addr = "127.0.0.1:9090".to_string();
        
        // Create the DID document
        let public_key_hex = hex::encode(keypair.public.to_bytes());
        
        let verification_method = VerificationMethod {
            id: format!("{}#keys-1", did),
            controller: did.clone(),
            type_: "Ed25519VerificationKey2020".to_string(),
            public_key_multibase: format!("z{}", public_key_hex),
        };
        
        let document = DidDocument {
            id: did.clone(),
            context: vec!["https://www.w3.org/ns/did/v1".to_string()],
            verification_method: vec![verification_method.clone()],
            authentication: vec![format!("{}#keys-1", did)],
            assertion_method: vec![format!("{}#keys-1", did)],
            capability_invocation: vec![format!("{}#keys-1", did)],
            capability_delegation: vec![],
            key_agreement: vec![],
            service: vec![
                Service {
                    id: format!("{}#node", did),
                    type_: "ICNNode".to_string(),
                    service_endpoint: format!("https://{}", listen_addr),
                },
            ],
        };
        
        // Store the DID document
        storage.put_json(&format!("dids/{}", did), &document)?;
        
        Ok(Identity {
            coop_id,
            node_id,
            did,
            document,
            keypair,
            listen_addr,
            tls: true,
            storage,
        })
    }
    
    // Get the DID document
    pub fn get_document(&self) -> &DidDocument {
        &self.document
    }
    
    // Update the DID document
    pub fn update_document(&mut self, document: DidDocument) -> Result<(), Box<dyn Error>> {
        // Verify document ID matches our DID
        if document.id != self.did {
            return Err(Box::new(IdentityError::ValidationError(
                "Document ID does not match identity DID".to_string(),
            )));
        }
        
        // Store the updated document
        self.storage.put_json(&format!("dids/{}", self.did), &document)?;
        
        // Update local copy
        self.document = document;
        
        Ok(())
    }
    
    // Sign data with the identity's keypair
    pub fn sign(&self, data: &[u8]) -> Result<Signature, Box<dyn Error>> {
        Ok(self.keypair.sign(data))
    }
    
    // Verify a signature using the identity's public key
    pub fn verify(&self, data: &[u8], signature: &Signature) -> Result<bool, Box<dyn Error>> {
        match self.keypair.public.verify(data, signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    // Verify a signature using a provided public key
    pub fn verify_with_key(&self, public_key: &PublicKey, data: &[u8], signature: &Signature) -> Result<bool, Box<dyn Error>> {
        match public_key.verify(data, signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    // Resolve a DID to a DID document
    pub fn resolve_did(&self, did: &str) -> Result<DidDocument, Box<dyn Error>> {
        // For now, we only support local resolution from our storage
        // In a real system, this would use more advanced resolution methods
        match self.storage.get_json(&format!("dids/{}", did)) {
            Ok(document) => Ok(document),
            Err(_) => Err(Box::new(IdentityError::InvalidDid(
                format!("Could not resolve DID: {}", did),
            ))),
        }
    }
    
    // Create a credential
    pub fn create_credential(&self, subject: &str, claims: serde_json::Value) -> Result<Credential, Box<dyn Error>> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
            
        let credential_id = format!("{}#vc-{}", self.did, now);
        
        let credential = Credential {
            id: credential_id,
            issuer: self.did.clone(),
            subject: subject.to_string(),
            claims,
            signature: None,
            issued_at: now,
            expires_at: Some(now + 86400 * 365), // 1 year expiration
        };
        
        // Sign the credential
        let credential_json = serde_json::to_string(&credential)?;
        let signature = self.sign(credential_json.as_bytes())?;
        let signature_base58 = bs58::encode(signature.to_bytes()).into_string();
        
        // Create the final credential with signature
        let mut signed_credential = credential;
        signed_credential.signature = Some(signature_base58);
        
        Ok(signed_credential)
    }
    
    // Verify a credential
    pub fn verify_credential(&self, credential: &Credential) -> Result<bool, Box<dyn Error>> {
        // Get the signature
        let signature_base58 = match &credential.signature {
            Some(sig) => sig,
            None => return Err(Box::new(IdentityError::ValidationError("Missing signature".to_string()))),
        };
        
        // Create a copy of the credential without the signature for verification
        let mut verification_credential = credential.clone();
        verification_credential.signature = None;
        
        // Serialize the credential for verification
        let credential_json = serde_json::to_string(&verification_credential)?;
        
        // Decode the signature
        let signature_bytes = bs58::decode(signature_base58)
            .into_vec()
            .map_err(|e| IdentityError::ValidationError(format!("Invalid signature encoding: {}", e)))?;
            
        let signature = Signature::from_bytes(&signature_bytes)
            .map_err(|e| IdentityError::ValidationError(format!("Invalid signature: {}", e)))?;
            
        // For simplicity, we're assuming we have the issuer's public key
        // In a real system, we would need to resolve the issuer's DID to get their public key
        // This is a placeholder for demonstration
        let public_key_multibase = &self.document.verification_method[0].public_key_multibase;
        let public_key_bytes = bs58::decode(public_key_multibase)
            .into_vec()
            .map_err(|e| IdentityError::ValidationError(format!("Invalid public key encoding: {}", e)))?;
            
        let public_key = PublicKey::from_bytes(&public_key_bytes)
            .map_err(|e| IdentityError::ValidationError(format!("Invalid public key: {}", e)))?;
            
        // Verify the signature
        Ok(public_key.verify(credential_json.as_bytes(), &signature).is_ok())
    }
    
    // Export DID document to file
    pub fn export_did_document(&self, path: &str) -> Result<(), Box<dyn Error>> {
        let json = serde_json::to_string_pretty(&self.document)?;
        fs::write(path, json)?;
        Ok(())
    }
    
    // Import DID document from file
    pub fn import_did_document(path: &str) -> Result<DidDocument, Box<dyn Error>> {
        let json = fs::read_to_string(path)?;
        let did_document: DidDocument = serde_json::from_str(&json)?;
        Ok(did_document)
    }
}

struct IdentityManager {
    identities: RwLock<HashMap<String, Identity>>,
    storage: Arc<dyn Storage>,
}

impl IdentityManager {
    pub fn new(
        storage: Arc<dyn Storage>
    ) -> Self {
        // ... existing code ...
    }
    // ... existing code ...
} 