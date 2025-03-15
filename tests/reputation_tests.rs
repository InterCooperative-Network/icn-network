use icn_node::crypto::CryptoUtils;
use icn_node::identity::Identity;
use icn_node::reputation::{
    ReputationSystem, AttestationType, Evidence, Attestation, SybilIndicators, TrustScore
};
use icn_node::storage::Storage;

use std::sync::Arc;
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::tempdir;

// Utility function to create a test environment
fn setup_test_environment() -> Result<(Arc<Identity>, Arc<Storage>, Arc<CryptoUtils>), Box<dyn Error>> {
    let temp_dir = tempdir()?;
    let storage = Arc::new(Storage::new(temp_dir.path().to_path_buf())?);
    
    let identity = Arc::new(Identity::new(
        "test-coop".to_string(),
        "test-node".to_string(),
        "did:icn:test-coop:test-node".to_string(),
        storage.clone(),
    )?);
    
    let crypto = Arc::new(CryptoUtils::new());
    
    Ok((identity, storage, crypto))
}

#[tokio::test]
async fn test_create_and_verify_attestation() -> Result<(), Box<dyn Error>> {
    // Set up test environment
    let (identity, storage, crypto) = setup_test_environment()?;
    
    // Create reputation system
    let reputation_system = ReputationSystem::new(
        identity.clone(),
        storage.clone(),
        crypto.clone(),
    );
    
    // Subject DID
    let subject_did = "did:icn:test-coop:member-1";
    
    // Create an attestation
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();
        
    let evidence = vec![
        Evidence {
            evidence_type: "test".to_string(),
            evidence_id: "test-1".to_string(),
            description: "Test evidence".to_string(),
            timestamp: now,
            data: None,
        }
    ];
    
    let attestation = reputation_system.attestation_manager().create_attestation(
        subject_did,
        AttestationType::CooperativeVerification,
        0.8,
        serde_json::json!({
            "verified": true,
            "test": "value"
        }),
        evidence,
        1, // Quorum of 1
        Some(365), // Valid for 1 year
    )?;
    
    // Verify the attestation was created correctly
    assert_eq!(attestation.subject_did, subject_did);
    assert_eq!(attestation.attestation_type, AttestationType::CooperativeVerification);
    assert_eq!(attestation.score, 0.8);
    assert_eq!(attestation.signatures.len(), 1);
    
    // Calculate trust score
    let trust_score = reputation_system.calculate_trust_score(subject_did)?;
    
    // Verify the trust score
    assert!(trust_score.overall_score > 0.0);
    assert_eq!(trust_score.attestation_count, 1);
    
    Ok(())
}

#[tokio::test]
async fn test_multi_party_attestation() -> Result<(), Box<dyn Error>> {
    // Set up test environment
    let (identity, storage, crypto) = setup_test_environment()?;
    
    // Create reputation system
    let reputation_system = ReputationSystem::new(
        identity.clone(),
        storage.clone(),
        crypto.clone(),
    );
    
    // Subject DID
    let subject_did = "did:icn:test-coop:member-2";
    
    // Create a multi-party attestation requiring 2 signatures
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();
        
    let evidence = vec![
        Evidence {
            evidence_type: "test".to_string(),
            evidence_id: "test-2".to_string(),
            description: "Test evidence for multi-party attestation".to_string(),
            timestamp: now,
            data: None,
        }
    ];
    
    let attestation = reputation_system.attestation_manager().create_attestation(
        subject_did,
        AttestationType::CooperativeVerification,
        0.9,
        serde_json::json!({
            "verified": true,
            "multi_party": true
        }),
        evidence,
        2, // Quorum of 2
        Some(180), // Valid for 180 days
    )?;
    
    // Check quorum isn't reached yet
    assert_eq!(attestation.signatures.len(), 1);
    assert!(!reputation_system.attestation_manager().has_reached_quorum(&attestation));
    
    // Add a second signature
    let second_signer = "did:icn:test-coop:another-node";
    let signature_data = format!("sign:{}", attestation.id);
    let signature = crypto.sign(signature_data.as_bytes())?;
    
    let updated_attestation = reputation_system.attestation_manager().sign_attestation(
        &attestation.id,
        second_signer,
        signature.to_bytes().to_vec(),
    )?;
    
    // Check quorum is now reached
    assert_eq!(updated_attestation.signatures.len(), 2);
    assert!(reputation_system.attestation_manager().has_reached_quorum(&updated_attestation));
    
    // Calculate trust score with quorum-verified attestation
    let trust_score = reputation_system.calculate_trust_score(subject_did)?;
    
    // Verify the trust score
    assert!(trust_score.overall_score > 0.0);
    
    Ok(())
}

#[tokio::test]
async fn test_sybil_resistance() -> Result<(), Box<dyn Error>> {
    // Set up test environment
    let (identity, storage, crypto) = setup_test_environment()?;
    
    // Create reputation system
    let reputation_system = ReputationSystem::new(
        identity.clone(),
        storage.clone(),
        crypto.clone(),
    );
    
    // Create multiple subjects
    let subjects = vec![
        "did:icn:test-coop:member-3",
        "did:icn:test-coop:member-4",
        "did:icn:test-coop:member-5",
    ];
    
    // For the first subject, create multiple attestations from different issuers
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();
    
    // Create some attestations for the first subject from diverse issuers
    let issuers = vec![
        "did:icn:coop-1:node-1",
        "did:icn:coop-2:node-1",
        "did:icn:coop-3:node-1",
    ];
    
    for (i, issuer) in issuers.iter().enumerate() {
        // Simulate an attestation from this issuer
        let evidence = vec![
            Evidence {
                evidence_type: "test".to_string(),
                evidence_id: format!("test-{}", i),
                description: format!("Test evidence from {}", issuer),
                timestamp: now - (i as u64 * 86400), // Spread out over time
                data: None,
            }
        ];
        
        // Store attestation in the storage directly (simulating it's from another party)
        let attestation = Attestation {
            id: format!("att:{}:{}:{}", issuer, subjects[0], now - (i as u64 * 86400)),
            issuer_did: issuer.to_string(),
            subject_did: subjects[0].to_string(),
            attestation_type: AttestationType::GeneralTrust,
            score: 0.8,
            context: vec!["https://schema.icn.coop/attestation/v1".to_string()],
            claims: serde_json::json!({"verified": true}),
            evidence,
            signatures: vec![],
            quorum_threshold: 1,
            created_at: now - (i as u64 * 86400),
            expires_at: Some(now + 31536000), // 1 year
            is_revoked: false,
        };
        
        storage.store_json(&format!("attestations/{}", attestation.id), &attestation)?;
    }
    
    // For the second subject, create just one attestation
    let evidence = vec![
        Evidence {
            evidence_type: "test".to_string(),
            evidence_id: "test-single".to_string(),
            description: "Single attestation for subject 2".to_string(),
            timestamp: now,
            data: None,
        }
    ];
    
    let attestation = reputation_system.attestation_manager().create_attestation(
        subjects[1],
        AttestationType::GeneralTrust,
        0.8,
        serde_json::json!({"verified": true}),
        evidence,
        1,
        Some(365),
    )?;
    
    // Check Sybil indicators for both subjects
    let indicators1 = reputation_system.sybil_resistance().check_sybil_indicators(subjects[0])?;
    let indicators2 = reputation_system.sybil_resistance().check_sybil_indicators(subjects[1])?;
    
    // The first subject should have a lower risk score (better)
    assert!(indicators1.unique_issuer_count > indicators2.unique_issuer_count);
    assert!(indicators1.risk_score < indicators2.risk_score);
    
    // Check trust scores, which should reflect Sybil adjustments
    let score1 = reputation_system.calculate_trust_score(subjects[0])?;
    let score2 = reputation_system.calculate_trust_score(subjects[1])?;
    
    // The first subject should have a higher score and confidence
    assert!(score1.overall_score > score2.overall_score);
    assert!(score1.confidence > score2.confidence);
    
    Ok(())
}

#[tokio::test]
async fn test_indirect_trust() -> Result<(), Box<dyn Error>> {
    // Set up test environment
    let (identity, storage, crypto) = setup_test_environment()?;
    
    // Create reputation system
    let reputation_system = ReputationSystem::new(
        identity.clone(),
        storage.clone(),
        crypto.clone(),
    );
    
    // Create a chain of trust:
    // A trusts B, B trusts C, so A should indirectly trust C
    let a = "did:icn:coop-1:node-a";
    let b = "did:icn:coop-2:node-b";
    let c = "did:icn:coop-3:node-c";
    
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs();
    
    // A trusts B
    let a_trusts_b = Attestation {
        id: format!("att:{}:{}:{}", a, b, now),
        issuer_did: a.to_string(),
        subject_did: b.to_string(),
        attestation_type: AttestationType::GeneralTrust,
        score: 0.9,
        context: vec!["https://schema.icn.coop/attestation/v1".to_string()],
        claims: serde_json::json!({"verified": true}),
        evidence: vec![],
        signatures: vec![],
        quorum_threshold: 1,
        created_at: now,
        expires_at: Some(now + 31536000),
        is_revoked: false,
    };
    
    // B trusts C
    let b_trusts_c = Attestation {
        id: format!("att:{}:{}:{}", b, c, now),
        issuer_did: b.to_string(),
        subject_did: c.to_string(),
        attestation_type: AttestationType::GeneralTrust,
        score: 0.8,
        context: vec!["https://schema.icn.coop/attestation/v1".to_string()],
        claims: serde_json::json!({"verified": true}),
        evidence: vec![],
        signatures: vec![],
        quorum_threshold: 1,
        created_at: now,
        expires_at: Some(now + 31536000),
        is_revoked: false,
    };
    
    // Store attestations
    storage.store_json(&format!("attestations/{}", a_trusts_b.id), &a_trusts_b)?;
    storage.store_json(&format!("attestations/{}", b_trusts_c.id), &b_trusts_c)?;
    
    // Check indirect trust from A to C
    let indirect_trust = reputation_system.trust_graph().calculate_indirect_trust(
        a, c, 2, 0.5
    )?;
    
    // Should have found an indirect trust path
    assert!(indirect_trust.is_some());
    
    // The trust should be approximately 0.9 * 0.8 = 0.72
    let trust_score = indirect_trust.unwrap();
    assert!(trust_score > 0.7);
    assert!(trust_score < 0.75);
    
    // Try with insufficient depth (should not find a path)
    let insufficient_depth = reputation_system.trust_graph().calculate_indirect_trust(
        a, c, 1, 0.5
    )?;
    
    assert!(insufficient_depth.is_none());
    
    Ok(())
} 