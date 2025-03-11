use icn_networking::{
    error::{NetworkError, Result},
    tls::TlsConfig,
    test_utils::generate_test_certificate,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use std::error::Error;

#[tokio::test]
async fn test_tls_connection() -> Result<()> {
    // Generate test certificates
    let (cert_chain, private_key) = generate_test_certificate();
    
    // Create TLS config
    let server_config = TlsConfig::new(
        cert_chain.clone(),
        private_key.clone_key(),
        Some(cert_chain.clone()),
    )?;
    
    let client_config = TlsConfig::new(
        cert_chain.clone(),
        private_key,
        Some(cert_chain),
    )?;

    // Start server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Server task
    let server_config = server_config.clone();
    let server_handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut tls_stream = server_config.acceptor().accept(stream).await.unwrap();
        
        let mut buf = [0u8; 13];
        tls_stream.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"Hello, server");
        
        tls_stream.write_all(b"Hello, client").await.unwrap();
        tls_stream.flush().await.unwrap();
        
        Ok::<_, Box<dyn Error + Send + Sync>>(())
    });

    // Client task
    let client_handle = tokio::spawn(async move {
        let stream = TcpStream::connect(addr).await.unwrap();
        let connector = tokio_rustls::TlsConnector::from(client_config.client_config());
        let mut tls_stream = connector
            .connect(rustls::pki_types::ServerName::try_from("localhost").unwrap(), stream)
            .await
            .unwrap();

        tls_stream.write_all(b"Hello, server").await.unwrap();
        tls_stream.flush().await.unwrap();
        
        let mut buf = [0u8; 13];
        tls_stream.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"Hello, client");
        
        Ok::<_, Box<dyn Error + Send + Sync>>(())
    });

    // Wait for both tasks to complete
    let server_result = server_handle.await.map_err(|e| NetworkError::Other(e.to_string()))?;
    let client_result = client_handle.await.map_err(|e| NetworkError::Other(e.to_string()))?;
    
    // Check results
    server_result.map_err(|e| NetworkError::Other(e.to_string()))?;
    client_result.map_err(|e| NetworkError::Other(e.to_string()))?;
    
    Ok(())
} 