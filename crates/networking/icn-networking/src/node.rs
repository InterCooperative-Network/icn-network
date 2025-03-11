use crate::{
    error::{NetworkError, Result},
    tls::TlsConfig,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use tokio_rustls::TlsAcceptor;
use std::{net::SocketAddr, sync::Arc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub listen_addr: SocketAddr,
    pub peers: Vec<SocketAddr>,
    pub node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Hello { node_id: String },
    Ping,
    Pong,
    Data(Vec<u8>),
}

pub struct Node {
    config: NodeConfig,
    tls_config: TlsConfig,
    message_tx: mpsc::Sender<(SocketAddr, Message)>,
    message_rx: mpsc::Receiver<(SocketAddr, Message)>,
}

impl Node {
    pub fn new(config: NodeConfig, tls_config: TlsConfig) -> Self {
        let (message_tx, message_rx) = mpsc::channel(100);
        Self {
            config,
            tls_config,
            message_tx,
            message_rx,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        // Start listening for incoming connections
        let listener = TcpListener::bind(self.config.listen_addr).await
            .map_err(|e| NetworkError::Connection(format!("Failed to bind: {}", e)))?;

        println!("Node {} listening on {}", self.config.node_id, self.config.listen_addr);

        // Connect to initial peers
        for &peer_addr in &self.config.peers {
            self.connect_to_peer(peer_addr).await?;
        }

        // Accept incoming connections
        let acceptor = self.tls_config.acceptor();
        let message_tx = self.message_tx.clone();

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        let acceptor = acceptor.clone();
                        let message_tx = message_tx.clone();
                        
                        tokio::spawn(async move {
                            match acceptor.accept(stream).await {
                                Ok(tls_stream) => {
                                    if let Err(e) = handle_connection(tls_stream, addr, message_tx).await {
                                        eprintln!("Error handling connection from {}: {}", addr, e);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("TLS handshake failed with {}: {}", addr, e);
                                }
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("Failed to accept connection: {}", e);
                    }
                }
            }
        });

        // Process incoming messages
        while let Some((peer_addr, message)) = self.message_rx.recv().await {
            self.handle_message(peer_addr, message).await?;
        }

        Ok(())
    }

    async fn connect_to_peer(&self, peer_addr: SocketAddr) -> Result<()> {
        let stream = TcpStream::connect(peer_addr).await
            .map_err(|e| NetworkError::Connection(format!("Failed to connect to {}: {}", peer_addr, e)))?;

        let connector = tokio_rustls::TlsConnector::from(self.tls_config.client_config());
        let tls_stream = connector
            .connect(
                rustls::pki_types::ServerName::try_from("localhost").unwrap(),
                stream,
            )
            .await
            .map_err(|e| NetworkError::Tls(e))?;

        let message_tx = self.message_tx.clone();
        
        // Send initial hello message
        let hello = Message::Hello {
            node_id: self.config.node_id.clone(),
        };
        
        tokio::spawn(async move {
            if let Err(e) = handle_connection(tls_stream, peer_addr, message_tx).await {
                eprintln!("Error handling connection to {}: {}", peer_addr, e);
            }
        });

        Ok(())
    }

    async fn handle_message(&self, peer_addr: SocketAddr, message: Message) -> Result<()> {
        match message {
            Message::Hello { node_id } => {
                println!("Received hello from {} ({})", node_id, peer_addr);
            }
            Message::Ping => {
                println!("Received ping from {}", peer_addr);
                // TODO: Send pong response
            }
            Message::Pong => {
                println!("Received pong from {}", peer_addr);
            }
            Message::Data(data) => {
                println!("Received {} bytes from {}", data.len(), peer_addr);
            }
        }
        Ok(())
    }
}

async fn handle_connection<S>(
    mut stream: S,
    peer_addr: SocketAddr,
    message_tx: mpsc::Sender<(SocketAddr, Message)>,
) -> Result<()>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    let mut buf = [0u8; 1024];
    
    loop {
        let n = stream.read(&mut buf).await
            .map_err(|e| NetworkError::Io(e))?;
        
        if n == 0 {
            // Connection closed
            return Ok(());
        }
        
        let message: Message = serde_json::from_slice(&buf[..n])
            .map_err(|e| NetworkError::Serialization(e))?;
        
        message_tx.send((peer_addr, message)).await
            .map_err(|e| NetworkError::Other(e.to_string()))?;
    }
} 