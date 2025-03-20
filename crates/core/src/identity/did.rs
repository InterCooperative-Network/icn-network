use std::error::Error;
use std::fmt;
use rand::rngs::OsRng;
use ed25519_dalek::{Keypair, PublicKey, Signature, Signer, Verifier};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use icn_core::storage::Storage;
use hex;
use bs58;

// DID Method specific to ICN
const DID_METHOD: &str = "icn";

#[derive(Debug)]
pub enum DidError {
    InvalidDid(String),
    CryptoError(String),
    StorageError(String),
    ValidationError(String),
}

impl fmt::Display for DidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DidError::InvalidDid(msg) => write!(f, "Invalid DID: {}", msg),
            DidError::CryptoError(msg) => write!(f, "Crypto error: {}", msg),
            DidError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            DidError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl Error for DidError {}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMethod {
    pub id: String,
    pub controller: String,
    pub type_: String,
    pub public_key_multibase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: String,
    pub type_: String,
    pub service_endpoint: String,
}

pub struct DidIdentity {
    pub coop_id: String,
    pub node_id: String,
    pub did: String,
    pub document: DidDocument,
    pub keypair: Keypair,
    pub listen_addr: String,
    pub tls: bool,
    storage: Arc<dyn Storage>,
}

impl DidIdentity {
    pub fn new(
        coop_id: String, 
        node_id: String, 
        did: String, 
        storage: Arc<dyn Storage>
    ) -> Result<Self, Box<dyn Error>> {
        let keypair = Keypair::generate(&mut OsRng);
        let listen_addr = "127.0.0.1:9090".to_string();
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
        
        storage.put_json(&format!("dids/{}", did), &document)?;
        
        Ok(DidIdentity {
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
    
    pub fn get_document(&self) -> &DidDocument {
        &self.document
    }
    
    pub fn update_document(&mut self, document: DidDocument) -> Result<(), Box<dyn Error>> {
        if document.id != self.did {
            return Err(Box::new(DidError::ValidationError(
                "Document ID does not match identity DID".to_string(),
            )));
        }
        
        self.storage.put_json(&format!("dids/{}", self.did), &document)?;
        self.document = document;
        Ok(())
    }
    
    pub fn sign(&self, data: &[u8]) -> Result<Signature, Box<dyn Error>> {
        Ok(self.keypair.sign(data))
    }
    
    pub fn verify(&self, data: &[u8], signature: &Signature) -> Result<bool, Box<dyn Error>> {
        match self.keypair.public.verify(data, signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    pub fn verify_with_key(&self, public_key: &PublicKey, data: &[u8], signature: &Signature) -> Result<bool, Box<dyn Error>> {
        match public_key.verify(data, signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    pub fn resolve_did(&self, did: &str) -> Result<DidDocument, Box<dyn Error>> {
        match self.storage.get_json(&format!("dids/{}", did)) {
            Ok(document) => Ok(document),
            Err(_) => Err(Box::new(DidError::InvalidDid(
                format!("Could not resolve DID: {}", did),
            ))),
        }
    }
    
    pub fn export_did_document(&self, path: &str) -> Result<(), Box<dyn Error>> {
        let json = serde_json::to_string_pretty(&self.document)?;
        std::fs::write(path, json)?;
        Ok(())
    }
    
    pub fn import_did_document(path: &str) -> Result<DidDocument, Box<dyn Error>> {
        let json = std::fs::read_to_string(path)?;
        let document: DidDocument = serde_json::from_str(&json)?;
        Ok(document)
    }
} 