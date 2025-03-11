// DID structure for ICN with enhanced W3C compliance
pub struct DID {
    method: String,         // The DID method (always "icn")
    federation_id: String,  // Federation identifier
    identifier: String,     // Unique identifier
    version: String,        // DID version for migration support
}

impl DID {
    // Create a new DID with version
    pub fn new(federation_id: &str, identifier: &str) -> Self {
        DID {
            method: "icn".to_string(),
            federation_id: federation_id.to_string(),
            identifier: identifier.to_string(),
            version: "2.0".to_string(), // Current DID version
        }
    }
    
    // Format DID as string
    pub fn to_string(&self) -> String {
        format!("did:{}:{}:{}", self.method, self.federation_id, self.identifier)
    }
    
    // Parse DID from string with enhanced error handling
    pub fn from_string(did_string: &str) -> Result<Self, DIDError> {
        let parts: Vec<&str> = did_string.split(':').collect();
        
        if parts.len() < 4 {
            return Err(DIDError::InvalidSyntax("Insufficient DID parts".to_string()));
        }
        
        if parts[0] != "did" {
            return Err(DIDError::InvalidSyntax("Missing 'did' prefix".to_string()));
        }
        
        if parts[1] != "icn" {
            return Err(DIDError::InvalidSyntax("Unsupported DID method".to_string()));
        }
        
        Ok(DID {
            method: "icn".to_string(),
            federation_id: parts[2].to_string(),
            identifier: parts[3].to_string(),
            version: "2.0".to_string(), // Default to current version
        })
    }
    
    // Migrate DID to a new version
    pub fn migrate_to_version(&mut self, target_version: &str) -> Result<(), DIDError> {
        match (self.version.as_str(), target_version) {
            ("1.0", "2.0") => {
                // Perform migration logic from 1.0 to 2.0
                self.version = "2.0".to_string();
                Ok(())
            }
            (current, target) if current == target => Ok(()),
            (current, target) => Err(DIDError::VersionMigrationError(
                format!("Cannot migrate from version {} to {}", current, target)
            )),
        }
    }
}

// W3C Compatible DID document 
pub struct DIDDocument {
    id: DID,
    controller: Option<DID>,
    verification_methods: Vec<VerificationMethod>,
    authentication: Vec<String>,
    assertion_method: Vec<String>,
    key_agreement: Vec<String>,
    service_endpoints: Vec<ServiceEndpoint>,
    context: Vec<String>,          // @context for W3C compliance
    also_known_as: Option<Vec<String>>, // Alternative identifiers
    metadata: HashMap<String, serde_json::Value>, // Additional metadata
}

// Enhanced verification method with support for various key formats
pub struct VerificationMethod {
    id: String,
    type_: String,
    controller: DID,
    public_key_multibase: Option<String>,
    public_key_jwk: Option<serde_json::Value>,
    blockchain_account_id: Option<String>,
}

// Enhanced service endpoint for multiple endpoint formats
pub struct ServiceEndpoint {
    id: String,
    type_: String,
    service_endpoint: ServiceEndpointValue,
    properties: HashMap<String, serde_json::Value>,
}

// Service endpoint can be a single URL, multiple URLs, or a complex object
pub enum ServiceEndpointValue {
    Single(String),
    Multiple(Vec<String>),
    Complex(HashMap<String, serde_json::Value>),
}

// Threshold cryptography support
pub struct ThresholdKey {
    threshold: u16,
    participants: Vec<String>,
    verification_methods: Vec<String>,
}

// Enhanced DID manager with support for W3C features and threshold crypto
pub struct DIDManager {
    storage: DIDStorage,
    resolver: DIDResolver,
    key_manager: KeyManager,
    federation_id: String,
    w3c_validator: W3CComplianceValidator,
}

impl DIDManager {
    // Create a new DID with enhanced options
    pub fn create_did(
        &self, 
        controller: &str, 
        key_type: KeyType,
        options: DIDCreationOptions,
    ) -> Result<DID, DIDError> {
        // Generate key pair
        let key_pair = self.key_manager.generate_key_pair(key_type)?;
        
        // Create DID identifier from public key
        let identifier = encode_multibase(&key_pair.public_key);
        
        // Create DID document with context
        let mut did_document = self.create_did_document(
            controller, 
            &identifier, 
            &key_pair.public_key
        )?;
        
        // Add W3C context
        did_document.context = vec![
            "https://www.w3.org/ns/did/v1".to_string(),
            "https://w3id.org/security/suites/ed25519-2020/v1".to_string(),
        ];
        
        // Apply options
        if let Some(also_known_as) = options.also_known_as {
            did_document.also_known_as = Some(also_known_as);
        }
        
        if let Some(metadata) = options.metadata {
            did_document.metadata = metadata;
        }
        
        // Validate W3C compliance
        self.w3c_validator.validate(&did_document)?;
        
        // Store DID document
        self.storage.store_did_document(&did_document)?;
        
        // Return DID
        Ok(DID::new(self.federation_id(), &identifier))
    }
    
    // Create a threshold DID (multiple signatures required)
    pub fn create_threshold_did(
        &self,
        controller: &str,
        participants: Vec<String>,
        threshold: u16,
    ) -> Result<DID, DIDError> {
        if threshold < 1 || threshold as usize > participants.len() {
            return Err(DIDError::InvalidThreshold);
        }
        
        // Create a DID with a threshold verification method
        let identifier = generate_random_identifier();
        let did = DID::new(self.federation_id(), &identifier);
        
        // Create verification methods for each participant
        let mut verification_methods = Vec::new();
        for participant in &participants {
            let vm = self.create_verification_method(&did, participant)?;
            verification_methods.push(vm);
        }
        
        // Create threshold key
        let threshold_key = ThresholdKey {
            threshold,
            participants: participants.clone(),
            verification_methods: verification_methods.iter()
                .map(|vm| vm.id.clone())
                .collect(),
        };
        
        // Create DID document with threshold
        let did_document = self.create_threshold_did_document(
            controller,
            &identifier,
            threshold_key,
            verification_methods,
        )?;
        
        // Store DID document
        self.storage.store_did_document(&did_document)?;
        
        Ok(did)
    }
    
    // Resolve a DID to its DID Document
    pub fn resolve_did(&self, did: &DID) -> Result<DIDDocument, DIDError> {
        self.resolver.resolve(did)
    }
    
    // Verify a signature using DID's verification method
    pub fn verify_signature(
        &self, 
        did: &DID, 
        message: &[u8], 
        signature: &Signature
    ) -> Result<bool, DIDError> {
        // Resolve DID to get document
        let did_document = self.resolve_did(did)?;
        
        // Get verification method
        let verification_method = did_document.verification_methods
            .iter()
            .find(|vm| vm.id.ends_with("#keys-1"))
            .ok_or(DIDError::VerificationMethodNotFound)?;
        
        // Verify signature based on verification method type
        if let Some(ref multibase) = verification_method.public_key_multibase {
            self.key_manager.verify_signature_multibase(
                multibase,
                message,
                signature
            )
        } else if let Some(ref jwk) = verification_method.public_key_jwk {
            self.key_manager.verify_signature_jwk(
                jwk,
                message,
                signature
            )
        } else {
            Err(DIDError::InvalidVerificationMethod)
        }
    }
    
    // Verify a threshold signature (requires multiple signatures)
    pub fn verify_threshold_signature(
        &self,
        did: &DID,
        message: &[u8],
        signatures: &HashMap<String, Signature>
    ) -> Result<bool, DIDError> {
        // Resolve DID to get document
        let did_document = self.resolve_did(did)?;
        
        // Find threshold key method
        let threshold_method = did_document.verification_methods
            .iter()
            .find(|vm| vm.type_ == "ThresholdKey")
            .ok_or(DIDError::VerificationMethodNotFound)?;
        
        // Extract threshold info from metadata
        let threshold_info: ThresholdKey = serde_json::from_value(
            threshold_method.public_key_jwk
                .as_ref()
                .ok_or(DIDError::InvalidVerificationMethod)?
                .clone()
        ).map_err(|_| DIDError::InvalidVerificationMethod)?;
        
        // Count valid signatures
        let mut valid_count = 0;
        for vm_id in &threshold_info.verification_methods {
            if let Some(sig) = signatures.get(vm_id) {
                if self.verify_signature_for_method(did, vm_id, message, sig)? {
                    valid_count += 1;
                }
            }
        }
        
        // Check if we have enough valid signatures
        Ok(valid_count >= threshold_info.threshold as usize)
    }
    
    // Validate W3C compliance
    pub fn validate_w3c_compliance(&self, did_document: &DIDDocument) -> Result<(), DIDError> {
        self.w3c_validator.validate(did_document)
    }
}

// Enhanced error handling for DID operations
#[derive(Debug, thiserror::Error)]
pub enum DIDError {
    #[error("Invalid DID syntax: {0}")]
    InvalidSyntax(String),
    
    #[error("Invalid DID data: {0}")]
    InvalidData(String),
    
    #[error("Unexpected end of file")]
    UnexpectedEof,
    
    #[error("Invalid DID format")]
    InvalidFormat,
    
    #[error("Verification method not found")]
    VerificationMethodNotFound,
    
    #[error("Invalid verification method")]
    InvalidVerificationMethod,
    
    #[error("Invalid threshold value")]
    InvalidThreshold,
    
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
    
    #[error("Version migration error: {0}")]
    VersionMigrationError(String),
    
    #[error("Not W3C compliant: {0:?}")]
    NonCompliant(Vec<String>),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Resolver error: {0}")]
    ResolverError(String),
    
    #[error("Cryptography error: {0}")]
    CryptographyError(String),
}
