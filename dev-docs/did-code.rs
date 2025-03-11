// DID structure for ICN
pub struct DID {
    method: String,         // The DID method (always "icn")
    federation_id: String,  // Federation identifier
    identifier: String,     // Unique identifier
}

impl DID {
    // Create a new DID
    pub fn new(federation_id: &str, identifier: &str) -> Self {
        DID {
            method: "icn".to_string(),
            federation_id: federation_id.to_string(),
            identifier: identifier.to_string(),
        }
    }
    
    // Format DID as string
    pub fn to_string(&self) -> String {
        format!("did:{}:{}:{}", self.method, self.federation_id, self.identifier)
    }
    
    // Parse DID from string
    pub fn from_string(did_string: &str) -> Result<Self, DIDError> {
        let parts: Vec<&str> = did_string.split(':').collect();
        
        if parts.len() != 4 || parts[0] != "did" || parts[1] != "icn" {
            return Err(DIDError::InvalidFormat);
        }
        
        Ok(DID {
            method: "icn".to_string(),
            federation_id: parts[2].to_string(),
            identifier: parts[3].to_string(),
        })
    }
}

// DID document containing verification methods
pub struct DIDDocument {
    id: DID,
    controller: Option<DID>,
    verification_methods: Vec<VerificationMethod>,
    authentication: Vec<String>,
    assertion_method: Vec<String>,
    key_agreement: Vec<String>,
    service_endpoints: Vec<ServiceEndpoint>,
}

// Verification method for DID
pub struct VerificationMethod {
    id: String,
    type_: String,
    controller: DID,
    public_key_multibase: String,
}

// Service endpoint for DID
pub struct ServiceEndpoint {
    id: String,
    type_: String,
    service_endpoint: String,
}

// DID manager to handle DID operations
pub struct DIDManager {
    storage: DIDStorage,
    resolver: DIDResolver,
    key_manager: KeyManager,
}

impl DIDManager {
    // Create a new DID
    pub fn create_did(
        &self, 
        controller: &str, 
        key_type: KeyType
    ) -> Result<DID, DIDError> {
        // Generate key pair
        let key_pair = self.key_manager.generate_key_pair(key_type)?;
        
        // Create DID identifier from public key
        let identifier = encode_multibase(&key_pair.public_key);
        
        // Create DID document
        let did_document = self.create_did_document(
            controller, 
            &identifier, 
            &key_pair.public_key
        )?;
        
        // Store DID document
        self.storage.store_did_document(&did_document)?;
        
        // Return DID
        Ok(DID::new(self.federation_id(), &identifier))
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
        
        // Verify signature
        self.key_manager.verify_signature(
            &verification_method.public_key_multibase,
            message,
            signature
        )
    }
}
