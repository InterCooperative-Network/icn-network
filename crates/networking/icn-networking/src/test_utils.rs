use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rcgen::{
    Certificate as RcgenCertificate,
    CertificateParams,
    DistinguishedName,
    DnType,
    PKCS_ECDSA_P256_SHA256,
};

/// Generate a test certificate and private key for testing TLS
pub fn generate_test_certificate() -> (Vec<CertificateDer<'static>>, PrivateKeyDer<'static>) {
    // Create certificate parameters
    let mut params = CertificateParams::new(vec!["localhost".to_string()]);
    params.distinguished_name = DistinguishedName::new();
    params.distinguished_name.push(DnType::OrganizationName, "ICN Test");
    params.distinguished_name.push(DnType::CommonName, "localhost");
    params.alg = &PKCS_ECDSA_P256_SHA256;

    // Generate certificate
    let cert = RcgenCertificate::from_params(params).unwrap();
    
    // Get DER-encoded certificate and private key
    let cert_der = cert.serialize_der().unwrap();
    let key_der = cert.serialize_private_key_der();
    
    // Convert to rustls types
    let cert_chain = vec![CertificateDer::from(cert_der)];
    
    // Create private key in PKCS8 format
    let key_vec = key_der.into();
    let private_key = PrivateKeyDer::Pkcs8(key_vec);
    
    (cert_chain, private_key)
} 