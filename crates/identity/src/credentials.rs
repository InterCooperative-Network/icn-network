use std::error::Error;
use std::fmt;
use serde::{Deserialize, Serialize};
use ed25519_dalek::{Signature, Signer};
use bs58;

#[derive(Debug)]
pub enum CredentialError {
    InvalidCredential(String),
    ValidationError(String),
    SignatureError(String),
}

impl fmt::Display for CredentialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CredentialError::InvalidCredential(msg) => write!(f, "Invalid credential: {}", msg),
            CredentialError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            CredentialError::SignatureError(msg) => write!(f, "Signature error: {}", msg),
        }
    }
}

impl Error for CredentialError {}

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

impl Credential {
    pub fn new(
        id: String,
        issuer: String,
        subject: String,
        claims: serde_json::Value,
        issued_at: u64,
        expires_at: Option<u64>,
    ) -> Self {
        Self {
            id,
            issuer,
            subject,
            claims,
            signature: None,
            issued_at,
            expires_at,
        }
    }

    pub fn sign(&mut self, signer: &impl Signer<Signature>) -> Result<(), Box<dyn Error>> {
        let mut credential = self.clone();
        credential.signature = None;
        
        let credential_json = serde_json::to_string(&credential)?;
        let signature = signer.sign(credential_json.as_bytes());
        let signature_base58 = bs58::encode(signature.to_bytes()).into_string();
        
        self.signature = Some(signature_base58);
        Ok(())
    }

    pub fn verify(&self, verifier: &impl Fn(&[u8], &Signature) -> Result<(), Box<dyn Error>>) -> Result<bool, Box<dyn Error>> {
        let signature_base58 = match &self.signature {
            Some(sig) => sig,
            None => return Err(Box::new(CredentialError::SignatureError("Missing signature".to_string()))),
        };

        let signature_bytes = bs58::decode(signature_base58)
            .into_vec()
            .map_err(|e| CredentialError::SignatureError(format!("Invalid base58 signature: {}", e)))?;
        
        let signature = Signature::from_bytes(&signature_bytes)
            .map_err(|e| CredentialError::SignatureError(format!("Invalid signature bytes: {}", e)))?;

        let mut verification_credential = self.clone();
        verification_credential.signature = None;
        
        let credential_json = serde_json::to_string(&verification_credential)?;
        
        match verifier(credential_json.as_bytes(), &signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            now > expires_at
        } else {
            false
        }
    }
} 