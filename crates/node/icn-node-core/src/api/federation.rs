use async_trait::async_trait;
use icn_common::{Error, Result};
use std::sync::Arc;
use crate::systems::{DidService, FederationCapability, FederationRequest, FederationResponse};
use crate::state::NodeState;
use crate::config::NetworkConfig;
use icn_common::{Result, Error, Federation};
use icn_did::{
    manager::DidManager,
    federation::{FederationClient, DiscoveryResponse},
};
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use axum::{
    extract::{Extension, Json, Path},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};

/// Federation API handler
pub struct FederationApi {
    did_service: Arc<DidService>,
}

impl FederationApi {
    /// Create a new federation API handler
    pub fn new(did_service: Arc<DidService>) -> Self {
        Self {
            did_service,
        }
    }
    
    /// Get federation information
    pub async fn get_federation_info(&self) -> Result<FederationInfo> {
        Ok(FederationInfo {
            federation_id: self.did_service.federation_id().to_string(),
            endpoints: self.did_service.federation_endpoints().to_vec(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }
    
    /// Handle DID resolution from another federation
    pub async fn resolve_did(&self, did: &str, federation_id: &str) -> Result<FederationResponse> {
        let request = FederationRequest::ResolveDid {
            did: did.to_string(),
            federation_id: federation_id.to_string(),
        };
        
        self.did_service.handle_federation_request(request).await
    }
    
    /// Handle DID verification from another federation
    pub async fn verify_did_signature(
        &self, 
        did: &str, 
        challenge: &[u8], 
        signature: &[u8]
    ) -> Result<FederationResponse> {
        let request = FederationRequest::VerifyDid {
            did: did.to_string(),
            challenge: challenge.to_vec(),
            signature: signature.to_vec(),
        };
        
        self.did_service.handle_federation_request(request).await
    }
    
    /// Lookup federation information for another federation
    pub async fn lookup_federation(&self, federation_id: &str) -> Result<Option<FederationInfo>> {
        // Call federation directory to lookup federation info
        // For now, just return federation configuration if it's the local federation
        if federation_id == self.did_service.federation_id() {
            return Ok(Some(FederationInfo {
                federation_id: self.did_service.federation_id().to_string(),
                endpoints: self.did_service.federation_endpoints().to_vec(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            }));
        }
        
        // In a real implementation, we would query a federation directory or use other discovery methods
        Ok(None)
    }
}

/// Federation information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FederationInfo {
    /// Federation ID
    pub federation_id: String,
    
    /// Federation API endpoints
    pub endpoints: Vec<String>,
    
    /// Federation software version
    pub version: String,
}

/// Federation API request for discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationDiscoveryRequest {
    /// Requesting federation ID
    pub federation_id: String,
    
    /// API version
    pub api_version: String,
    
    /// Optional authentication token
    pub auth_token: Option<String>,
}

/// Federation API response for discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationDiscoveryResponse {
    /// Federation ID
    pub federation_id: String,
    
    /// API version
    pub api_version: String,
    
    /// API endpoints
    pub endpoints: Vec<FederationEndpoint>,
    
    /// Federation metadata
    pub metadata: FederationMetadata,
}

/// Federation API endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationEndpoint {
    /// Endpoint type
    pub endpoint_type: String,
    
    /// Endpoint URL
    pub url: String,
    
    /// Authentication required
    pub auth_required: bool,
}

/// Federation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationMetadata {
    /// Node count
    pub node_count: u32,
    
    /// Federation name
    pub name: String,
    
    /// Federation description
    pub description: Option<String>,
    
    /// Federation public key
    pub public_key: String,
}

/// Federation API request to resolve a DID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationDidResolutionRequest {
    /// DID to resolve
    pub did: String,
}

/// Federation API response for DID resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationDidResolutionResponse {
    /// DID document
    pub did_document: Option<serde_json::Value>,
    
    /// Resolution metadata
    pub metadata: serde_json::Value,
}

/// Federation API request to verify a signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationSignatureVerificationRequest {
    /// DID
    pub did: String,
    
    /// Challenge
    pub challenge: String,
    
    /// Signature
    pub signature: String,
}

/// Federation API response for signature verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationSignatureVerificationResponse {
    /// Is valid
    pub is_valid: bool,
}

/// Create federation API router
pub fn create_federation_router() -> Router {
    Router::new()
        .route("/federation/discovery", get(handle_discovery).post(handle_discovery))
        .route("/federation/did/:did", get(handle_did_resolution))
        .route("/federation/verify", post(handle_signature_verification))
}

/// Handle federation discovery request
async fn handle_discovery(
    Extension(state): Extension<Arc<NodeState>>,
    Json(request): Json<FederationDiscoveryRequest>,
) -> impl IntoResponse {
    // Validate the request
    if request.federation_id.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "Invalid federation ID"
        }))).into_response();
    }

    // Log the discovery request
    tracing::info!(
        "Received federation discovery request from federation: {}",
        request.federation_id
    );

    // Process discovery
    let network_config = state.config().network_config();
    let federation_id = network_config.federation_id();
    
    // Build response with federation endpoints
    let response = FederationDiscoveryResponse {
        federation_id: federation_id.to_string(),
        api_version: "1.0".to_string(),
        endpoints: vec![
            FederationEndpoint {
                endpoint_type: "did-resolution".to_string(),
                url: format!("{}/federation/did", network_config.public_api_url()),
                auth_required: false,
            },
            FederationEndpoint {
                endpoint_type: "signature-verification".to_string(),
                url: format!("{}/federation/verify", network_config.public_api_url()),
                auth_required: false,
            },
        ],
        metadata: FederationMetadata {
            node_count: state.node_count().await as u32,
            name: network_config.federation_name().to_string(),
            description: network_config.federation_description().map(|d| d.to_string()),
            public_key: hex::encode(network_config.federation_public_key().as_bytes()),
        },
    };

    // Return response
    (StatusCode::OK, Json(response)).into_response()
}

/// Handle DID resolution request
async fn handle_did_resolution(
    Extension(state): Extension<Arc<NodeState>>,
    Path(did): Path<String>,
) -> impl IntoResponse {
    // Get DID manager
    let did_manager = state.did_manager();
    
    // Resolve DID
    match did_manager.resolve(&did).await {
        Ok(result) => {
            // Build response
            let response = FederationDidResolutionResponse {
                did_document: result.did_document.map(|doc| serde_json::to_value(doc).unwrap_or_default()),
                metadata: serde_json::to_value(result.metadata).unwrap_or_default(),
            };
            
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": format!("Failed to resolve DID: {}", err)
            }))).into_response()
        }
    }
}

/// Handle signature verification request
async fn handle_signature_verification(
    Extension(state): Extension<Arc<NodeState>>,
    Json(request): Json<FederationSignatureVerificationRequest>,
) -> impl IntoResponse {
    // Get DID manager
    let did_manager = state.did_manager();
    
    // Decode challenge and signature
    let challenge = match hex::decode(&request.challenge) {
        Ok(c) => c,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid challenge format"
            }))).into_response();
        }
    };
    
    let signature = match hex::decode(&request.signature) {
        Ok(s) => match icn_crypto::Signature::from_bytes(&s) {
            Ok(sig) => sig,
            Err(_) => {
                return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                    "error": "Invalid signature format"
                }))).into_response();
            }
        },
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
                "error": "Invalid signature format"
            }))).into_response();
        }
    };
    
    // Verify signature
    match did_manager.verify_signature(&request.did, &challenge, &signature).await {
        Ok(is_valid) => {
            let response = FederationSignatureVerificationResponse {
                is_valid,
            };
            
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": format!("Signature verification failed: {}", err)
            }))).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::state::StateManager;
    use crate::config::NodeConfig;
    use icn_storage_system::StorageOptions;
    
    #[tokio::test]
    async fn test_federation_api() {
        let temp_dir = tempdir().unwrap();
        let state_manager = Arc::new(StateManager::new());
        
        // Create node config
        let config = NodeConfig {
            node_id: "test-node".to_string(),
            federation_id: "test-federation".to_string(),
            federation_endpoints: vec!["http://federation.test/api".to_string()],
            storage: StorageOptions {
                base_dir: temp_dir.path().to_path_buf(),
                sync_writes: true,
                compress: false,
            },
            ..Default::default()
        };
        
        // Create DID service
        let did_service = Arc::new(DidService::from_config(&config, state_manager).await.unwrap());
        
        // Create API handler
        let api = FederationApi::new(did_service);
        
        // Test federation info
        let info = api.get_federation_info().await.unwrap();
        assert_eq!(info.federation_id, "test-federation");
        assert_eq!(info.endpoints.len(), 1);
        assert_eq!(info.endpoints[0], "http://federation.test/api");
        
        // Test federation lookup
        let lookup = api.lookup_federation("test-federation").await.unwrap();
        assert!(lookup.is_some());
        assert_eq!(lookup.unwrap().federation_id, "test-federation");
        
        let lookup = api.lookup_federation("unknown-federation").await.unwrap();
        assert!(lookup.is_none());
        
        // Test DID resolution
        let did = "did:icn:test-federation:123";
        let response = api.resolve_did(did, "test-federation").await.unwrap();
        match response {
            FederationResponse::DidResolution { document, error } => {
                // Document doesn't exist but resolution was successful
                assert!(document.is_none());
                assert!(error.is_some());
            }
            _ => panic!("Unexpected response type"),
        }
    }
}