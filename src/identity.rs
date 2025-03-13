use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;
use rand::rngs::OsRng;
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, Signer, Verifier};
use serde::{Deserialize, Serialize};

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

#[derive(Debug)]
pub enum IdentityError {
    KeyGeneration(String),
    DidCreation(String),
    Serialization(String),
    Verification(String),
    Storage(String),
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IdentityError::KeyGeneration(msg) => write!(f, "Key generation error: {}", msg),
            IdentityError::DidCreation(msg) => write!(f, "DID creation error: {}", msg),
            IdentityError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            IdentityError::Verification(msg) => write!(f, "Verification error: {}", msg),
            IdentityError::Storage(msg) => write!(f, "Storage error: {}", msg),
        }
    }
}

impl Error for IdentityError {}

// DID Document structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidDocument {
    pub id: String,
    pub verification_method: Vec<VerificationMethod>,
    pub authentication: Vec<String>,
    pub assertion_method: Vec<String>,
    pub capability_invocation: Vec<String>,
    pub capability_delegation: Vec<String>,
    pub service: Vec<Service>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMethod {
    pub id: String,
    pub controller: String,
    pub type_: String,
    pub public_key_base58: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: String,
    pub type_: String,
    pub service_endpoint: String,
}

// Main identity structure
pub struct Identity {
    pub did: String,
    pub did_document: DidDocument,
    keypair: Keypair,
    pub node_id: String,
    pub coop_id: String,
}

impl Identity {
    // Update the new method to match our test requirements
    pub fn new(coop_id: String, node_id: String, did: String, storage: Storage) -> Result<Self, Box<dyn Error>> {
        // Generate a new keypair
        let mut csprng = OsRng{};
        let keypair = Keypair::generate(&mut csprng);
        
        // Create a DID document
        let verification_method = VerificationMethod {
            id: format!("{}#keys-1", did),
            controller: did.clone(),
            type_: "Ed25519VerificationKey2020".to_string(),
            public_key_base58: bs58::encode(keypair.public.as_bytes()).into_string(),
        };
        
        let did_document = DidDocument {
            id: did.clone(),
            verification_method: vec![verification_method.clone()],
            authentication: vec![verification_method.id.clone()],
            assertion_method: vec![verification_method.id.clone()],
            capability_invocation: vec![verification_method.id.clone()],
            capability_delegation: vec![verification_method.id.clone()],
            service: vec![],
        };
        
        // Store the DID document
        storage.put_json(&format!("identity/{}/did-document", node_id), &did_document)?;
        
        Ok(Identity {
            did,
            did_document,
            keypair,
            node_id,
            coop_id,
        })
    }
    
    // Sign a message
    pub fn sign(&self, message: &[u8]) -> Result<Signature, Box<dyn Error>> {
        Ok(self.keypair.sign(message))
    }
    
    // Verify a signature
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<bool, Box<dyn Error>> {
        Ok(self.keypair.public.verify(message, signature).is_ok())
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
            None => return Err(Box::new(IdentityError::Verification("Missing signature".to_string()))),
        };
        
        // Create a copy of the credential without the signature for verification
        let mut verification_credential = credential.clone();
        verification_credential.signature = None;
        
        // Serialize the credential for verification
        let credential_json = serde_json::to_string(&verification_credential)?;
        
        // Decode the signature
        let signature_bytes = bs58::decode(signature_base58)
            .into_vec()
            .map_err(|e| IdentityError::Verification(format!("Invalid signature encoding: {}", e)))?;
            
        let signature = Signature::from_bytes(&signature_bytes)
            .map_err(|e| IdentityError::Verification(format!("Invalid signature: {}", e)))?;
            
        // For simplicity, we're assuming we have the issuer's public key
        // In a real system, we would need to resolve the issuer's DID to get their public key
        // This is a placeholder for demonstration
        let public_key_base58 = &self.did_document.verification_method[0].public_key_base58;
        let public_key_bytes = bs58::decode(public_key_base58)
            .into_vec()
            .map_err(|e| IdentityError::Verification(format!("Invalid public key encoding: {}", e)))?;
            
        let public_key = PublicKey::from_bytes(&public_key_bytes)
            .map_err(|e| IdentityError::Verification(format!("Invalid public key: {}", e)))?;
            
        // Verify the signature
        Ok(public_key.verify(credential_json.as_bytes(), &signature).is_ok())
    }
    
    // Export DID document to file
    pub fn export_did_document(&self, path: &str) -> Result<(), Box<dyn Error>> {
        let json = serde_json::to_string_pretty(&self.did_document)?;
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