use icn_networking::{
    error::Result,
    tls::TlsConfig,
};
use tokio::{
    net::{TcpListener, TcpStream},
    io::{AsyncReadExt, AsyncWriteExt},
};
use std::sync::Arc;

#[tokio::test]
async fn test_tls_connection() -> Result<()> {
    // Generate test certificates
    let (cert_chain, private_key) = icn_networking::test_utils::generate_test_certificate();
    
    // Create TLS config
    let tls_config = Arc::new(TlsConfig::new(
        cert_chain.clone(),
        private_key.clone(),
        Some(cert_chain.clone()),
    )?);
    
    // Start server
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    
    // Server task
    let server_config = tls_config.clone();
    let server = tokio::spawn(async move {
        let (tcp_stream, _) = listener.accept().await?;
        let acceptor = server_config.acceptor();
        let mut tls_stream = acceptor.accept(tcp_stream).await?;
        
        // Read client message
        let mut buf = [0u8; 13];
        tls_stream.read_exact(&mut buf).await?;
        assert_eq!(&buf, b"Hello, server!");
        
        // Send response
        tls_stream.write_all(b"Hello, client!").await?;
        
        Result::<_>::Ok(())
    });
    
    // Client task
    let client = tokio::spawn(async move {
        let tcp_stream = TcpStream::connect(addr).await?;
        let connector = tokio_rustls::TlsConnector::from(tls_config.client_config());
        let mut tls_stream = connector.connect("localhost".try_into().unwrap(), tcp_stream).await?;
        
        // Send message
        tls_stream.write_all(b"Hello, server!").await?;
        
        // Read response
        let mut buf = [0u8; 13];
        tls_stream.read_exact(&mut buf).await?;
        assert_eq!(&buf, b"Hello, client!");
        
        Result::<_>::Ok(())
    });
    
    // Wait for both tasks to complete
    let (server_result, client_result) = tokio::join!(server, client);
    server_result??;
    client_result??;
    
    Ok(())
} 