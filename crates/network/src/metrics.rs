use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use prometheus::{
    Counter, Gauge, Histogram, HistogramOpts, IntCounter, IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry,
};
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use crate::NetworkResult;

/// Network metrics collector
#[derive(Clone)]
pub struct NetworkMetrics {
    registry: Registry,
    
    // Connection metrics
    peers_connected: IntGauge,
    connection_attempts: IntCounter,
    connection_successes: IntCounter,
    connection_failures: IntCounter,
    disconnections: IntCounter,
    
    // Message metrics
    messages_received: IntCounterVec,
    messages_sent: IntCounterVec,
    message_bytes_received: IntCounterVec,
    message_bytes_sent: IntCounterVec,
    message_processing_time: Histogram,
    
    // Discovery metrics
    peers_discovered: IntCounter,
    bootstrap_connections: IntCounter,
    mdns_discoveries: IntCounter,
    kad_discoveries: IntCounter,
    
    // Protocol metrics
    protocol_negotiation_time: Histogram,
    protocol_failures: IntCounterVec,
    
    // Resource metrics
    memory_usage: IntGaugeVec,
    cpu_usage: Gauge,
    
    // Error metrics
    errors: IntCounterVec,
    
    // Latency tracking
    peer_latencies: Arc<RwLock<HashMap<String, Duration>>>,
    
    // Reputation metrics
    reputation_scores: IntGaugeVec,
    reputation_changes: IntCounterVec,
    banned_peers: IntGaugeVec,
    total_banned_peers: IntGauge,
}

impl NetworkMetrics {
    /// Create a new NetworkMetrics instance
    pub fn new() -> Self {
        let registry = Registry::new();
        
        // Connection metrics
        let peers_connected = IntGauge::new("network_peers_connected", "Number of connected peers").unwrap();
        let connection_attempts = IntCounter::new("network_connection_attempts", "Number of connection attempts").unwrap();
        let connection_successes = IntCounter::new("network_connection_successes", "Number of successful connections").unwrap();
        let connection_failures = IntCounter::new("network_connection_failures", "Number of failed connections").unwrap();
        let disconnections = IntCounter::new("network_disconnections", "Number of disconnections").unwrap();
        
        // Message metrics
        let messages_received = IntCounterVec::new(
            Opts::new("network_messages_received", "Number of messages received by type"),
            &["message_type"],
        ).unwrap();
        
        let messages_sent = IntCounterVec::new(
            Opts::new("network_messages_sent", "Number of messages sent by type"),
            &["message_type"],
        ).unwrap();
        
        let message_bytes_received = IntCounterVec::new(
            Opts::new("network_message_bytes_received", "Number of bytes received by message type"),
            &["message_type"],
        ).unwrap();
        
        let message_bytes_sent = IntCounterVec::new(
            Opts::new("network_message_bytes_sent", "Number of bytes sent by message type"),
            &["message_type"],
        ).unwrap();
        
        let message_processing_time = Histogram::with_opts(
            HistogramOpts::new(
                "network_message_processing_time",
                "Time to process messages in milliseconds",
            )
            .buckets(vec![
                1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0,
            ]),
        ).unwrap();
        
        // Discovery metrics
        let peers_discovered = IntCounter::new("network_peers_discovered", "Number of peers discovered").unwrap();
        let bootstrap_connections = IntCounter::new("network_bootstrap_connections", "Number of bootstrap connections").unwrap();
        let mdns_discoveries = IntCounter::new("network_mdns_discoveries", "Number of mDNS discoveries").unwrap();
        let kad_discoveries = IntCounter::new("network_kad_discoveries", "Number of Kademlia discoveries").unwrap();
        
        // Protocol metrics
        let protocol_negotiation_time = Histogram::with_opts(
            HistogramOpts::new(
                "network_protocol_negotiation_time",
                "Time to negotiate protocols in milliseconds",
            )
            .buckets(vec![
                1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0,
            ]),
        ).unwrap();
        
        let protocol_failures = IntCounterVec::new(
            Opts::new("network_protocol_failures", "Number of protocol negotiation failures"),
            &["protocol"],
        ).unwrap();
        
        // Resource metrics
        let memory_usage = IntGaugeVec::new(
            Opts::new("network_memory_usage", "Memory usage in bytes"),
            &["component"],
        ).unwrap();
        
        let cpu_usage = Gauge::new("network_cpu_usage", "CPU usage percentage").unwrap();
        
        // Error metrics
        let errors = IntCounterVec::new(
            Opts::new("network_errors", "Number of errors by type"),
            &["error_type"],
        ).unwrap();
        
        // Reputation metrics
        let reputation_scores = IntGaugeVec::new(
            Opts::new("network_peer_reputation_scores", "Reputation scores by peer"),
            &["peer_id"],
        ).unwrap();
        
        let reputation_changes = IntCounterVec::new(
            Opts::new("network_reputation_changes", "Number of reputation changes by peer"),
            &["peer_id", "type"],
        ).unwrap();
        
        let banned_peers = IntGaugeVec::new(
            Opts::new("network_banned_peers", "Whether a peer is banned (1) or not (0)"),
            &["peer_id"],
        ).unwrap();
        
        let total_banned_peers = IntGauge::new(
            "network_total_banned_peers", 
            "Total number of banned peers"
        ).unwrap();
        
        // Register metrics
        registry.register(Box::new(peers_connected.clone())).unwrap();
        registry.register(Box::new(connection_attempts.clone())).unwrap();
        registry.register(Box::new(connection_successes.clone())).unwrap();
        registry.register(Box::new(connection_failures.clone())).unwrap();
        registry.register(Box::new(disconnections.clone())).unwrap();
        registry.register(Box::new(messages_received.clone())).unwrap();
        registry.register(Box::new(messages_sent.clone())).unwrap();
        registry.register(Box::new(message_bytes_received.clone())).unwrap();
        registry.register(Box::new(message_bytes_sent.clone())).unwrap();
        registry.register(Box::new(message_processing_time.clone())).unwrap();
        registry.register(Box::new(peers_discovered.clone())).unwrap();
        registry.register(Box::new(bootstrap_connections.clone())).unwrap();
        registry.register(Box::new(mdns_discoveries.clone())).unwrap();
        registry.register(Box::new(kad_discoveries.clone())).unwrap();
        registry.register(Box::new(protocol_negotiation_time.clone())).unwrap();
        registry.register(Box::new(protocol_failures.clone())).unwrap();
        registry.register(Box::new(memory_usage.clone())).unwrap();
        registry.register(Box::new(cpu_usage.clone())).unwrap();
        registry.register(Box::new(errors.clone())).unwrap();
        registry.register(Box::new(reputation_scores.clone())).unwrap();
        registry.register(Box::new(reputation_changes.clone())).unwrap();
        registry.register(Box::new(banned_peers.clone())).unwrap();
        registry.register(Box::new(total_banned_peers.clone())).unwrap();
        
        info!("Network metrics initialized");
        
        Self {
            registry,
            peers_connected,
            connection_attempts,
            connection_successes,
            connection_failures,
            disconnections,
            messages_received,
            messages_sent,
            message_bytes_received,
            message_bytes_sent,
            message_processing_time,
            peers_discovered,
            bootstrap_connections,
            mdns_discoveries,
            kad_discoveries,
            protocol_negotiation_time,
            protocol_failures,
            memory_usage,
            cpu_usage,
            errors,
            peer_latencies: Arc::new(RwLock::new(HashMap::new())),
            reputation_scores,
            reputation_changes,
            banned_peers,
            total_banned_peers,
        }
    }
    
    /// Get the metrics registry
    pub fn registry(&self) -> &Registry {
        &self.registry
    }
    
    /// Record a peer connection
    pub fn record_peer_connected(&self) {
        self.peers_connected.inc();
        debug!("Peer connected, total: {}", self.peers_connected.get());
    }
    
    /// Record a peer disconnection
    pub fn record_peer_disconnected(&self) {
        self.peers_connected.dec();
        self.disconnections.inc();
        debug!("Peer disconnected, total: {}", self.peers_connected.get());
    }
    
    /// Record a connection attempt
    pub fn record_connection_attempt(&self) {
        self.connection_attempts.inc();
    }
    
    /// Record a connection success
    pub fn record_connection_success(&self) {
        self.connection_successes.inc();
    }
    
    /// Record a connection failure
    pub fn record_connection_failure(&self) {
        self.connection_failures.inc();
    }
    
    /// Record a received message
    pub fn record_message_received(&self, message_type: &str, size_bytes: usize) {
        self.messages_received.with_label_values(&[message_type]).inc();
        self.message_bytes_received.with_label_values(&[message_type]).inc_by(size_bytes as u64);
    }
    
    /// Record a sent message
    pub fn record_message_sent(&self, message_type: &str, size_bytes: usize) {
        self.messages_sent.with_label_values(&[message_type]).inc();
        self.message_bytes_sent.with_label_values(&[message_type]).inc_by(size_bytes as u64);
    }
    
    /// Record message processing time
    pub fn record_message_processing_time(&self, duration: Duration) {
        self.message_processing_time.observe(duration.as_millis() as f64);
    }
    
    /// Start timing message processing
    pub fn start_message_processing_timer(&self) -> Instant {
        Instant::now()
    }
    
    /// Stop timing message processing and record the result
    pub fn stop_message_processing_timer(&self, start: Instant) {
        let duration = start.elapsed();
        self.record_message_processing_time(duration);
    }
    
    /// Record a peer discovery
    pub fn record_peer_discovered(&self) {
        self.peers_discovered.inc();
    }
    
    /// Record a bootstrap connection
    pub fn record_bootstrap_connection(&self) {
        self.bootstrap_connections.inc();
    }
    
    /// Record an mDNS discovery
    pub fn record_mdns_discovery(&self) {
        self.mdns_discoveries.inc();
    }
    
    /// Record a Kademlia discovery
    pub fn record_kad_discovery(&self) {
        self.kad_discoveries.inc();
    }
    
    /// Record protocol negotiation time
    pub fn record_protocol_negotiation_time(&self, duration: Duration) {
        self.protocol_negotiation_time.observe(duration.as_millis() as f64);
    }
    
    /// Record a protocol failure
    pub fn record_protocol_failure(&self, protocol: &str) {
        self.protocol_failures.with_label_values(&[protocol]).inc();
    }
    
    /// Record memory usage
    pub fn record_memory_usage(&self, component: &str, bytes: i64) {
        self.memory_usage.with_label_values(&[component]).set(bytes);
    }
    
    /// Record CPU usage
    pub fn record_cpu_usage(&self, percentage: f64) {
        self.cpu_usage.set(percentage);
    }
    
    /// Record an error
    pub fn record_error(&self, error_type: &str) {
        self.errors.with_label_values(&[error_type]).inc();
        debug!("Recorded error: {}", error_type);
    }
    
    /// Record peer latency
    pub async fn record_peer_latency(&self, peer_id: &str, latency: Duration) {
        let mut latencies = self.peer_latencies.write().await;
        latencies.insert(peer_id.to_string(), latency);
    }
    
    /// Get average peer latency
    pub async fn get_average_peer_latency(&self) -> Option<Duration> {
        let latencies = self.peer_latencies.read().await;
        
        if latencies.is_empty() {
            return None;
        }
        
        let total: Duration = latencies.values().sum();
        Some(total / latencies.len() as u32)
    }
    
    /// Get peer latency
    pub async fn get_peer_latency(&self, peer_id: &str) -> Option<Duration> {
        let latencies = self.peer_latencies.read().await;
        latencies.get(peer_id).cloned()
    }
    
    /// Reset all metrics
    pub fn reset(&self) {
        self.peers_connected.set(0);
        // We don't reset counters as they should be monotonic
    }
    
    /// Record a reputation change for a peer
    pub fn record_reputation_change(&self, peer_id: &str, change: i32) {
        let change_type = if change > 0 {
            "positive"
        } else if change < 0 {
            "negative"
        } else {
            "neutral"
        };
        
        self.reputation_changes.with_label_values(&[peer_id, change_type]).inc();
    }
    
    /// Record a positive action from a peer
    pub fn record_positive_action(&self, peer_id: &str, action: &str) {
        self.reputation_changes.with_label_values(&[peer_id, "action"]).inc();
    }
    
    /// Record a negative action from a peer
    pub fn record_negative_action(&self, peer_id: &str, action: &str) {
        self.reputation_changes.with_label_values(&[peer_id, "action"]).inc();
    }
    
    /// Update a peer's reputation score
    pub fn update_reputation_score(&self, peer_id: &str, score: i32) {
        self.reputation_scores.with_label_values(&[peer_id]).set(score);
    }
    
    /// Record that a peer was banned
    pub fn record_peer_banned(&self, peer_id: &str) {
        self.banned_peers.with_label_values(&[peer_id]).set(1);
        self.total_banned_peers.inc();
    }
    
    /// Record that a peer was unbanned
    pub fn record_peer_unbanned(&self, peer_id: &str) {
        self.banned_peers.with_label_values(&[peer_id]).set(0);
        self.total_banned_peers.dec();
    }
    
    /// Record reputation decay activity
    pub fn record_reputation_decay(&self, peers_processed: u64) {
        // No specific metric needed here, but we can log it
        debug!("Processed reputation decay for {} peers", peers_processed);
    }
}

impl Default for NetworkMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to measure code execution time and record it
pub struct Timer<'a> {
    metrics: &'a NetworkMetrics,
    start: Instant,
    label: String,
}

impl<'a> Timer<'a> {
    /// Create a new timer for message processing
    pub fn new_message_timer(metrics: &'a NetworkMetrics) -> Self {
        Self {
            metrics,
            start: Instant::now(),
            label: "message".to_string(),
        }
    }
    
    /// Create a new timer for protocol negotiation
    pub fn new_protocol_timer(metrics: &'a NetworkMetrics) -> Self {
        Self {
            metrics,
            start: Instant::now(),
            label: "protocol".to_string(),
        }
    }
}

impl<'a> Drop for Timer<'a> {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        
        if self.label == "message" {
            self.metrics.record_message_processing_time(duration);
        } else if self.label == "protocol" {
            self.metrics.record_protocol_negotiation_time(duration);
        }
    }
}

/// Create an HTTP server to expose Prometheus metrics
pub async fn start_metrics_server(metrics: NetworkMetrics, addr: &str) -> NetworkResult<()> {
    use hyper::{
        service::{make_service_fn, service_fn},
        Body, Request, Response, Server,
    };
    use prometheus::{Encoder, TextEncoder};
    
    // Create a service to handle the request
    let metrics_clone = metrics.clone();
    let make_svc = make_service_fn(move |_| {
        let metrics = metrics_clone.clone();
        async move {
            Ok::<_, hyper::Error>(service_fn(move |_req: Request<Body>| {
                let metrics = metrics.clone();
                async move {
                    let encoder = TextEncoder::new();
                    let mut buffer = vec![];
                    
                    // Gather the metrics
                    let metric_families = metrics.registry().gather();
                    encoder.encode(&metric_families, &mut buffer).unwrap();
                    
                    // Create the response
                    let response = Response::builder()
                        .status(200)
                        .header("content-type", encoder.format_type())
                        .body(Body::from(buffer))
                        .unwrap();
                    
                    Ok::<_, hyper::Error>(response)
                }
            }))
        }
    });
    
    // Parse the address
    let addr = addr.parse()
        .map_err(|e| crate::NetworkError::InternalError(format!("Invalid metrics address: {}", e)))?;
    
    // Create and start the server
    let server = Server::bind(&addr).serve(make_svc);
    
    info!("Metrics server listening on {}", addr);
    
    // Run the server in the background
    tokio::spawn(async move {
        if let Err(e) = server.await {
            error!("Metrics server error: {}", e);
        }
    });
    
    Ok(())
}

/// Scheduled metrics collection task
pub async fn start_metrics_collection(metrics: NetworkMetrics) {
    use tokio::time::interval;
    
    // Start a background task to collect metrics periodically
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(15));
        
        loop {
            interval.tick().await;
            
            // Collect system metrics
            if let Some(memory) = get_process_memory_usage() {
                metrics.record_memory_usage("process", memory as i64);
            }
            
            if let Some(cpu) = get_process_cpu_usage() {
                metrics.record_cpu_usage(cpu);
            }
            
            // Log some periodic stats
            let avg_latency = metrics.get_average_peer_latency().await;
            
            if let Some(latency) = avg_latency {
                debug!(
                    "Network stats: peers={}, avg_latency={:?}ms",
                    metrics.peers_connected.get(),
                    latency.as_millis()
                );
            } else {
                debug!(
                    "Network stats: peers={}",
                    metrics.peers_connected.get()
                );
            }
        }
    });
}

/// Get the current process memory usage
fn get_process_memory_usage() -> Option<usize> {
    #[cfg(target_os = "linux")]
    {
        use std::fs::File;
        use std::io::Read;
        
        let mut buffer = String::new();
        if let Ok(mut file) = File::open("/proc/self/status") {
            if file.read_to_string(&mut buffer).is_ok() {
                if let Some(line) = buffer.lines().find(|l| l.starts_with("VmRSS:")) {
                    if let Some(size_str) = line.split_whitespace().nth(1) {
                        if let Ok(size) = size_str.parse::<usize>() {
                            return Some(size * 1024); // Convert KB to bytes
                        }
                    }
                }
            }
        }
    }
    
    // For non-Linux platforms or if reading /proc fails
    None
}

/// Get the current process CPU usage
fn get_process_cpu_usage() -> Option<f64> {
    // This is a simplified implementation and may not be accurate
    // For production code, consider using a cross-platform library
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[test]
    fn test_metrics_creation() {
        let metrics = NetworkMetrics::new();
        
        // Verify basic metrics are created
        assert_eq!(metrics.peers_connected.get(), 0);
        assert_eq!(metrics.connection_attempts.get(), 0);
        assert_eq!(metrics.connection_successes.get(), 0);
    }
    
    #[test]
    fn test_metrics_recording() {
        let metrics = NetworkMetrics::new();
        
        // Test connection metrics
        metrics.record_peer_connected();
        metrics.record_peer_connected();
        metrics.record_connection_attempt();
        metrics.record_connection_success();
        
        assert_eq!(metrics.peers_connected.get(), 2);
        assert_eq!(metrics.connection_attempts.get(), 1);
        assert_eq!(metrics.connection_successes.get(), 1);
        
        // Test peer disconnection
        metrics.record_peer_disconnected();
        assert_eq!(metrics.peers_connected.get(), 1);
        assert_eq!(metrics.disconnections.get(), 1);
        
        // Test message metrics
        metrics.record_message_received("transaction", 1024);
        metrics.record_message_sent("identity", 512);
        
        // Test error recording
        metrics.record_error("test_error");
        
        // Test reset
        metrics.reset();
        assert_eq!(metrics.peers_connected.get(), 0);
        // Counters should not be reset
        assert_eq!(metrics.connection_attempts.get(), 1);
    }
    
    #[tokio::test]
    async fn test_peer_latency() {
        let metrics = NetworkMetrics::new();
        
        // Record latencies
        metrics.record_peer_latency("peer1", Duration::from_millis(100)).await;
        metrics.record_peer_latency("peer2", Duration::from_millis(200)).await;
        
        // Test getting specific peer latency
        let latency1 = metrics.get_peer_latency("peer1").await;
        assert_eq!(latency1, Some(Duration::from_millis(100)));
        
        // Test average latency calculation
        let avg = metrics.get_average_peer_latency().await;
        assert_eq!(avg, Some(Duration::from_millis(150)));
        
        // Test non-existent peer
        let latency3 = metrics.get_peer_latency("peer3").await;
        assert_eq!(latency3, None);
    }
    
    #[test]
    fn test_timer() {
        let metrics = NetworkMetrics::new();
        
        // Use the timer
        {
            let _timer = Timer::new_message_timer(&metrics);
            std::thread::sleep(Duration::from_millis(10));
        }
        
        // The timer should automatically record when dropped
    }
} 