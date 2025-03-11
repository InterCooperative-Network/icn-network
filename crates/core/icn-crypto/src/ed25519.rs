//! Ed25519 signature scheme implementation
use ed25519_dalek::{Keypair as DalekKeypair, PublicKey as DalekPublicKey,
                    SecretKey as DalekSecretKey, Signature as DalekSignature,
                    Signer as DalekSigner, Verifier as DalekVerifier};
use icn_common::{Error, Result};
use rand::rngs::OsRng;
use crate::hash::{sha256, Hash};
use crate::keys::{KeyPair, KeyType, PrivateKey, PublicKey};
use crate::signature::{Signature, SignatureAlgorithm, Signer, Verifier};

/// Ed25519 public key implementation
#[derive(Debug, Clone)]
pub struct Ed25519PublicKey {
    /// The inner dalek public key
    pub(crate) key: DalekPublicKey,
}

impl Ed25519PublicKey {
    /// Create a new Ed25519 public key from raw bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let key = DalekPublicKey::from_bytes(bytes)
            .map_err(|e| Error::validation(format!("Invalid Ed25519 public key: {}", e)))?;
        
        Ok(Self { key })
    }
}

impl PublicKey for Ed25519PublicKey {
    fn key_type(&self) -> KeyType {
        KeyType::Ed25519
    }
    
    fn as_bytes(&self) -> &[u8] {
        self.key.as_bytes()
    }
    
    fn to_base58(&self) -> String {
        bs58::encode(self.as_bytes()).into_string()
    }
    
    fn fingerprint(&self) -> String {
        let hash = sha256(self.as_bytes());
        hash.to_hex()[0..16].to_string()
    }
    
    fn clone_box(&self) -> Box<dyn PublicKey> {
        Box::new(self.clone())
    }
}

impl Verifier for Ed25519PublicKey {
    fn algorithm(&self) -> SignatureAlgorithm {
        SignatureAlgorithm::Ed25519
    }
    
    fn verify(&self, data: &[u8], signature: &Signature) -> Result<bool> {
        if signature.algorithm != SignatureAlgorithm::Ed25519 {
            return Err(Error::validation(format!(
                "Expected Ed25519 signature, got {:?}", signature.algorithm
            )));
        }
        
        let dalek_sig = DalekSignature::from_bytes(&signature.value)
            .map_err(|e| Error::validation(format!("Invalid Ed25519 signature: {}", e)))?;
            
        Ok(self.key.verify(data, &dalek_sig).is_ok())
    }
}

/// Ed25519 private key implementation
#[derive(Debug, Clone)]
pub struct Ed25519PrivateKey {
    /// The inner dalek secret key
    key: DalekSecretKey,
    /// The corresponding public key
    public_key: Ed25519PublicKey,
}

impl Ed25519PrivateKey {
    /// Create a new Ed25519 private key from raw bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let secret = DalekSecretKey::from_bytes(bytes)
            .map_err(|e| Error::validation(format!("Invalid Ed25519 private key: {}", e)))?;
        
        let public = DalekPublicKey::from(&secret);
        
        Ok(Self {
            key: secret,
            public_key: Ed25519PublicKey { key: public },
        })
    }
    
    /// Generate a new random Ed25519 private key
    pub fn generate() -> Result<Self> {
        let mut csprng = OsRng;
        let secret = DalekSecretKey::generate(&mut csprng);
        let public = DalekPublicKey::from(&secret);
        
        Ok(Self {
            key: secret,
            public_key: Ed25519PublicKey { key: public },
        })
    }
}

impl PrivateKey for Ed25519PrivateKey {
    fn key_type(&self) -> KeyType {
        KeyType::Ed25519
    }
    
    fn as_bytes(&self) -> &[u8] {
        self.key.as_bytes()
    }
    
    fn public_key(&self) -> Box<dyn PublicKey> {
        Box::new(self.public_key.clone())
    }
    
    fn to_base58(&self) -> String {
        bs58::encode(self.as_bytes()).into_string()
    }
    
    fn clone_box(&self) -> Box<dyn PrivateKey> {
        Box::new(self.clone())
    }
}

/// Ed25519 key pair implementation
#[derive(Debug, Clone)]
pub struct Ed25519KeyPair {
    /// The inner dalek keypair
    key_pair: DalekKeypair,
    /// The public key
    public_key: Ed25519PublicKey,
    /// The private key
    private_key: Ed25519PrivateKey,
}

impl Ed25519KeyPair {
    /// Create a new Ed25519 keypair from separate public and private keys
    pub fn new(private_key: Ed25519PrivateKey, public_key: Ed25519PublicKey) -> Result<Self> {
        // Verify that the private and public keys match
        let expected_public = DalekPublicKey::from(&private_key.key);
        if expected_public.as_bytes() != public_key.key.as_bytes() {
            return Err(Error::validation("Private key does not match public key"));
        }
        
        let dalek_keypair = DalekKeypair {
            secret: private_key.key,
            public: public_key.key,
        };
        
        Ok(Self {
            key_pair: dalek_keypair,
            public_key,
            private_key,
        })
    }
    
    /// Generate a new random Ed25519 keypair
    pub fn generate() -> Result<Self> {
        let mut csprng = OsRng;
        let dalek_keypair = DalekKeypair::generate(&mut csprng);
        
        let public_key = Ed25519PublicKey {
            key: dalek_keypair.public,
        };
        
        let private_key = Ed25519PrivateKey {
            key: dalek_keypair.secret,
            public_key: public_key.clone(),
        };
        
        Ok(Self {
            key_pair: dalek_keypair,
            public_key,
            private_key,
        })
    }
}

impl KeyPair for Ed25519KeyPair {
    fn key_type(&self) -> KeyType {
        KeyType::Ed25519
    }
    
    fn private_key(&self) -> &dyn PrivateKey {
        &self.private_key
    }
    
    fn public_key(&self) -> &dyn PublicKey {
        &self.public_key
    }
    
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
        let signature: DalekSignature = self.key_pair.sign(message);
        Ok(signature.to_bytes().to_vec())
    }
    
    fn clone_box(&self) -> Box<dyn KeyPair> {
        Box::new(self.clone())
    }
}

impl Signer for Ed25519KeyPair {
    fn algorithm(&self) -> SignatureAlgorithm {
        SignatureAlgorithm::Ed25519
    }
    
    fn sign(&self, data: &[u8]) -> Result<Signature> {
        let signature = self.key_pair.sign(data);
        Ok(Signature::new(
            SignatureAlgorithm::Ed25519,
            signature.to_bytes().to_vec(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_keypair() {
        let keypair = Ed25519KeyPair::generate().unwrap();
        assert_eq!(keypair.key_type(), KeyType::Ed25519);
        
        // Check that public key can be derived from private key
        let derived_public = keypair.private_key().public_key();
        assert_eq!(
            derived_public.as_bytes(),
            keypair.public_key().as_bytes()
        );
    }
    
    #[test]
    fn test_sign_and_verify() {
        let keypair = Ed25519KeyPair::generate().unwrap();
        let message = b"test message";
        
        // Sign using the KeyPair trait
        let sig_bytes = keypair.sign(message).unwrap();
        
        // Verify using the Verifier trait on the public key
        let signature = Signature::new(SignatureAlgorithm::Ed25519, sig_bytes);
        let verified = keypair.public_key().verify(message, &signature).unwrap();
        
        assert!(verified);
        
        // Test with modified message
        let wrong_message = b"wrong message";
        let verified_wrong = keypair.public_key().verify(wrong_message, &signature).unwrap();
        
        assert!(!verified_wrong);
    }
    
    #[test]
    fn test_base58_encoding() {
        let keypair = Ed25519KeyPair::generate().unwrap();
        
        let pubkey_base58 = keypair.public_key().to_base58();
        let privkey_base58 = keypair.private_key().to_base58();
        
        // Check that encodings are different
        assert_ne!(pubkey_base58, privkey_base58);
        
        // Check that encodings are non-empty
        assert!(!pubkey_base58.is_empty());
        assert!(!privkey_base58.is_empty());
    }
    
    #[test]
    fn test_key_fingerprint() {
        let keypair = Ed25519KeyPair::generate().unwrap();
        let fingerprint = keypair.public_key().fingerprint();
        
        // Fingerprint should be a 16-character hex string
        assert_eq!(fingerprint.len(), 16);
        assert!(fingerprint.chars().all(|c| c.is_ascii_hexdigit()));
    }
}