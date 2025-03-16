use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use std::time::{Instant, Duration};

/// Metrics for the distributed storage system
pub struct StorageMetrics {
    // Core metrics
    operation_counts: RwLock<OperationCounts>,
    operation_latencies: RwLock<OperationLatencies>,
    // Data metrics
    data_metrics: RwLock<DataMetrics>,
    // Version metrics
    version_metrics: RwLock<VersionMetrics>,
    // Last metrics update time
    last_update: RwLock<Instant>,
}

/// Counts of different storage operations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OperationCounts {
    pub puts: u64,
    pub gets: u64,
    pub deletes: u64,
    pub list_keys: u64,
    pub version_list: u64,
    pub version_get: u64,
    pub version_revert: u64,
    pub encryption_operations: u64,
    pub failed_operations: u64,
}

/// Latency tracking for operations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OperationLatencies {
    pub put_latency_ms: ExponentialMovingAverage,
    pub get_latency_ms: ExponentialMovingAverage,
    pub delete_latency_ms: ExponentialMovingAverage,
    pub version_operations_latency_ms: ExponentialMovingAverage,
}

/// Data-related metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DataMetrics {
    pub total_keys: u64,
    pub total_size_bytes: u64,
    pub encrypted_keys: u64,
    pub encrypted_size_bytes: u64,
    pub versioned_keys: u64,
    pub bytes_written: u64,
    pub bytes_read: u64,
}

/// Version-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VersionMetrics {
    pub total_versions: u64,
    pub versions_per_key: ExponentialMovingAverage,
    pub version_size_bytes: ExponentialMovingAverage,
    pub revert_operations: u64,
    pub version_storage_overhead_bytes: u64,
}

/// Simple exponential moving average implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExponentialMovingAverage {
    value: f64,
    alpha: f64,
    count: u64,
}

impl Default for ExponentialMovingAverage {
    fn default() -> Self {
        Self {
            value: 0.0,
            alpha: 0.1, // Default weight for new values
            count: 0,
        }
    }
}

impl ExponentialMovingAverage {
    pub fn new(alpha: f64) -> Self {
        Self {
            value: 0.0,
            alpha,
            count: 0,
        }
    }
    
    pub fn update(&mut self, new_value: f64) {
        if self.count == 0 {
            self.value = new_value;
        } else {
            self.value = self.alpha * new_value + (1.0 - self.alpha) * self.value;
        }
        self.count += 1;
    }
    
    pub fn get(&self) -> f64 {
        self.value
    }
    
    pub fn count(&self) -> u64 {
        self.count
    }
}

/// Snapshot of all metrics for a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub operation_counts: OperationCounts,
    pub operation_latencies: OperationLatencies,
    pub data_metrics: DataMetrics,
    pub version_metrics: VersionMetrics,
    pub uptime_seconds: u64,
    pub timestamp: u64,
}

impl StorageMetrics {
    /// Create a new metrics tracker
    pub fn new() -> Self {
        Self {
            operation_counts: RwLock::new(OperationCounts::default()),
            operation_latencies: RwLock::new(OperationLatencies::default()),
            data_metrics: RwLock::new(DataMetrics::default()),
            version_metrics: RwLock::new(VersionMetrics::default()),
            last_update: RwLock::new(Instant::now()),
        }
    }
    
    /// Record a put operation
    pub async fn record_put(&self, size_bytes: u64, is_encrypted: bool, is_versioned: bool, latency_ms: u64) {
        // Update operation counts
        {
            let mut counts = self.operation_counts.write().await;
            counts.puts += 1;
        }
        
        // Update latency metrics
        {
            let mut latencies = self.operation_latencies.write().await;
            latencies.put_latency_ms.update(latency_ms as f64);
        }
        
        // Update data metrics
        {
            let mut data = self.data_metrics.write().await;
            data.total_keys += 1;
            data.total_size_bytes += size_bytes;
            data.bytes_written += size_bytes;
            
            if is_encrypted {
                data.encrypted_keys += 1;
                data.encrypted_size_bytes += size_bytes;
            }
            
            if is_versioned {
                data.versioned_keys += 1;
            }
        }
    }
    
    /// Record a get operation
    pub async fn record_get(&self, size_bytes: u64, latency_ms: u64) {
        // Update operation counts
        {
            let mut counts = self.operation_counts.write().await;
            counts.gets += 1;
        }
        
        // Update latency metrics
        {
            let mut latencies = self.operation_latencies.write().await;
            latencies.get_latency_ms.update(latency_ms as f64);
        }
        
        // Update data metrics
        {
            let mut data = self.data_metrics.write().await;
            data.bytes_read += size_bytes;
        }
    }
    
    /// Record a delete operation
    pub async fn record_delete(&self, latency_ms: u64) {
        // Update operation counts
        {
            let mut counts = self.operation_counts.write().await;
            counts.deletes += 1;
        }
        
        // Update latency metrics
        {
            let mut latencies = self.operation_latencies.write().await;
            latencies.delete_latency_ms.update(latency_ms as f64);
        }
    }
    
    /// Record a version list operation
    pub async fn record_version_list(&self, versions_count: u64, latency_ms: u64) {
        // Update operation counts
        {
            let mut counts = self.operation_counts.write().await;
            counts.version_list += 1;
        }
        
        // Update versioning metrics
        {
            let mut version_metrics = self.version_metrics.write().await;
            version_metrics.versions_per_key.update(versions_count as f64);
        }
        
        // Update latency metrics
        {
            let mut latencies = self.operation_latencies.write().await;
            latencies.version_operations_latency_ms.update(latency_ms as f64);
        }
    }
    
    /// Record a version get operation
    pub async fn record_version_get(&self, size_bytes: u64, latency_ms: u64) {
        // Update operation counts
        {
            let mut counts = self.operation_counts.write().await;
            counts.version_get += 1;
        }
        
        // Update versioning metrics
        {
            let mut version_metrics = self.version_metrics.write().await;
            version_metrics.version_size_bytes.update(size_bytes as f64);
        }
        
        // Update latency metrics
        {
            let mut latencies = self.operation_latencies.write().await;
            latencies.version_operations_latency_ms.update(latency_ms as f64);
        }
        
        // Update data metrics
        {
            let mut data = self.data_metrics.write().await;
            data.bytes_read += size_bytes;
        }
    }
    
    /// Record a version revert operation
    pub async fn record_version_revert(&self, latency_ms: u64) {
        // Update operation counts
        {
            let mut counts = self.operation_counts.write().await;
            counts.version_revert += 1;
        }
        
        // Update versioning metrics
        {
            let mut version_metrics = self.version_metrics.write().await;
            version_metrics.revert_operations += 1;
        }
        
        // Update latency metrics
        {
            let mut latencies = self.operation_latencies.write().await;
            latencies.version_operations_latency_ms.update(latency_ms as f64);
        }
    }
    
    /// Record a failed operation
    pub async fn record_failed_operation(&self) {
        let mut counts = self.operation_counts.write().await;
        counts.failed_operations += 1;
    }
    
    /// Record version creation
    pub async fn record_version_creation(&self, size_bytes: u64) {
        let mut version_metrics = self.version_metrics.write().await;
        version_metrics.total_versions += 1;
        version_metrics.version_storage_overhead_bytes += size_bytes;
    }
    
    /// Update metrics related to key count and storage usage
    pub async fn update_storage_metrics(&self, total_keys: u64, total_size: u64, versioned_keys: u64, encrypted_keys: u64, encrypted_size: u64) {
        let mut data = self.data_metrics.write().await;
        data.total_keys = total_keys;
        data.total_size_bytes = total_size;
        data.versioned_keys = versioned_keys;
        data.encrypted_keys = encrypted_keys;
        data.encrypted_size_bytes = encrypted_size;
    }
    
    /// Get a snapshot of all current metrics
    pub async fn get_snapshot(&self) -> MetricsSnapshot {
        let now = Instant::now();
        let uptime = {
            let last = self.last_update.read().await;
            now.duration_since(*last).as_secs()
        };
        
        // Capture current timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        MetricsSnapshot {
            operation_counts: self.operation_counts.read().await.clone(),
            operation_latencies: self.operation_latencies.read().await.clone(),
            data_metrics: self.data_metrics.read().await.clone(),
            version_metrics: self.version_metrics.read().await.clone(),
            uptime_seconds: uptime,
            timestamp,
        }
    }
    
    /// Reset all metrics
    pub async fn reset(&self) {
        *self.operation_counts.write().await = OperationCounts::default();
        *self.operation_latencies.write().await = OperationLatencies::default();
        *self.data_metrics.write().await = DataMetrics::default();
        *self.version_metrics.write().await = VersionMetrics::default();
        *self.last_update.write().await = Instant::now();
    }
}

/// Helper to create a metrics timer that will automatically record the operation duration
pub struct MetricsTimer {
    start: Instant,
    operation_type: OperationType,
    size_bytes: Option<u64>,
    is_encrypted: Option<bool>,
    is_versioned: Option<bool>,
    versions_count: Option<u64>,
    metrics: Arc<StorageMetrics>,
}

/// Type of operation being timed
pub enum OperationType {
    Put,
    Get,
    Delete,
    VersionList,
    VersionGet,
    VersionRevert,
}

impl MetricsTimer {
    /// Create a new timer for a put operation
    pub fn new_put(metrics: Arc<StorageMetrics>, size_bytes: u64, is_encrypted: bool, is_versioned: bool) -> Self {
        Self {
            start: Instant::now(),
            operation_type: OperationType::Put,
            size_bytes: Some(size_bytes),
            is_encrypted: Some(is_encrypted),
            is_versioned: Some(is_versioned),
            versions_count: None,
            metrics,
        }
    }
    
    /// Create a new timer for a get operation
    pub fn new_get(metrics: Arc<StorageMetrics>, size_bytes: u64) -> Self {
        Self {
            start: Instant::now(),
            operation_type: OperationType::Get,
            size_bytes: Some(size_bytes),
            is_encrypted: None,
            is_versioned: None,
            versions_count: None,
            metrics,
        }
    }
    
    /// Create a new timer for a delete operation
    pub fn new_delete(metrics: Arc<StorageMetrics>) -> Self {
        Self {
            start: Instant::now(),
            operation_type: OperationType::Delete,
            size_bytes: None,
            is_encrypted: None,
            is_versioned: None,
            versions_count: None,
            metrics,
        }
    }
    
    /// Create a new timer for a version list operation
    pub fn new_version_list(metrics: Arc<StorageMetrics>, versions_count: u64) -> Self {
        Self {
            start: Instant::now(),
            operation_type: OperationType::VersionList,
            size_bytes: None,
            is_encrypted: None,
            is_versioned: None,
            versions_count: Some(versions_count),
            metrics,
        }
    }
    
    /// Create a new timer for a version get operation
    pub fn new_version_get(metrics: Arc<StorageMetrics>, size_bytes: u64) -> Self {
        Self {
            start: Instant::now(),
            operation_type: OperationType::VersionGet,
            size_bytes: Some(size_bytes),
            is_encrypted: None,
            is_versioned: None,
            versions_count: None,
            metrics,
        }
    }
    
    /// Create a new timer for a version revert operation
    pub fn new_version_revert(metrics: Arc<StorageMetrics>) -> Self {
        Self {
            start: Instant::now(),
            operation_type: OperationType::VersionRevert,
            size_bytes: None,
            is_encrypted: None,
            is_versioned: None,
            versions_count: None,
            metrics,
        }
    }
    
    /// Record a successful operation, updating metrics
    pub async fn record_success(self) {
        let elapsed_ms = self.start.elapsed().as_millis() as u64;
        
        match self.operation_type {
            OperationType::Put => {
                self.metrics.record_put(
                    self.size_bytes.unwrap_or(0),
                    self.is_encrypted.unwrap_or(false),
                    self.is_versioned.unwrap_or(false),
                    elapsed_ms
                ).await;
            },
            OperationType::Get => {
                self.metrics.record_get(
                    self.size_bytes.unwrap_or(0),
                    elapsed_ms
                ).await;
            },
            OperationType::Delete => {
                self.metrics.record_delete(elapsed_ms).await;
            },
            OperationType::VersionList => {
                self.metrics.record_version_list(
                    self.versions_count.unwrap_or(0),
                    elapsed_ms
                ).await;
            },
            OperationType::VersionGet => {
                self.metrics.record_version_get(
                    self.size_bytes.unwrap_or(0),
                    elapsed_ms
                ).await;
            },
            OperationType::VersionRevert => {
                self.metrics.record_version_revert(elapsed_ms).await;
            },
        }
    }
    
    /// Record a failed operation
    pub async fn record_failure(self) {
        self.metrics.record_failed_operation().await;
    }
}

/// Utility functions for formatting metrics in human-readable form
pub mod format {
    use super::*;
    
    /// Format a size in bytes to a human-readable string (KB, MB, GB, etc.)
    pub fn format_size(size_bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        const TB: u64 = GB * 1024;
        
        if size_bytes < KB {
            format!("{} B", size_bytes)
        } else if size_bytes < MB {
            format!("{:.2} KB", size_bytes as f64 / KB as f64)
        } else if size_bytes < GB {
            format!("{:.2} MB", size_bytes as f64 / MB as f64)
        } else if size_bytes < TB {
            format!("{:.2} GB", size_bytes as f64 / GB as f64)
        } else {
            format!("{:.2} TB", size_bytes as f64 / TB as f64)
        }
    }
    
    /// Format a duration in seconds to a human-readable string
    pub fn format_duration(seconds: u64) -> String {
        let days = seconds / (24 * 60 * 60);
        let hours = (seconds % (24 * 60 * 60)) / (60 * 60);
        let minutes = (seconds % (60 * 60)) / 60;
        let secs = seconds % 60;
        
        if days > 0 {
            format!("{}d {}h {}m {}s", days, hours, minutes, secs)
        } else if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, secs)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, secs)
        } else {
            format!("{}s", secs)
        }
    }
    
    /// Generate a formatted metrics report as a string
    pub fn metrics_report(snapshot: &MetricsSnapshot) -> String {
        let mut report = String::new();
        
        report.push_str("=== STORAGE METRICS REPORT ===\n\n");
        
        // Uptime
        report.push_str(&format!("Uptime: {}\n\n", format_duration(snapshot.uptime_seconds)));
        
        // Operation statistics
        report.push_str("OPERATION COUNTS:\n");
        report.push_str(&format!("  Puts: {}\n", snapshot.operation_counts.puts));
        report.push_str(&format!("  Gets: {}\n", snapshot.operation_counts.gets));
        report.push_str(&format!("  Deletes: {}\n", snapshot.operation_counts.deletes));
        report.push_str(&format!("  Version operations: {}\n", 
            snapshot.operation_counts.version_list + 
            snapshot.operation_counts.version_get + 
            snapshot.operation_counts.version_revert));
        report.push_str(&format!("  Failed operations: {}\n\n", snapshot.operation_counts.failed_operations));
        
        // Latencies
        report.push_str("OPERATION LATENCIES:\n");
        report.push_str(&format!("  Put: {:.2} ms (samples: {})\n", 
            snapshot.operation_latencies.put_latency_ms.get(),
            snapshot.operation_latencies.put_latency_ms.count()));
        report.push_str(&format!("  Get: {:.2} ms (samples: {})\n", 
            snapshot.operation_latencies.get_latency_ms.get(),
            snapshot.operation_latencies.get_latency_ms.count()));
        report.push_str(&format!("  Delete: {:.2} ms (samples: {})\n", 
            snapshot.operation_latencies.delete_latency_ms.get(),
            snapshot.operation_latencies.delete_latency_ms.count()));
        report.push_str(&format!("  Version operations: {:.2} ms (samples: {})\n\n", 
            snapshot.operation_latencies.version_operations_latency_ms.get(),
            snapshot.operation_latencies.version_operations_latency_ms.count()));
        
        // Data statistics
        report.push_str("DATA METRICS:\n");
        report.push_str(&format!("  Total keys: {}\n", snapshot.data_metrics.total_keys));
        report.push_str(&format!("  Total storage used: {}\n", format_size(snapshot.data_metrics.total_size_bytes)));
        report.push_str(&format!("  Encrypted keys: {} ({:.1}%)\n", 
            snapshot.data_metrics.encrypted_keys,
            if snapshot.data_metrics.total_keys > 0 {
                (snapshot.data_metrics.encrypted_keys as f64 / snapshot.data_metrics.total_keys as f64) * 100.0
            } else {
                0.0
            }));
        report.push_str(&format!("  Encrypted storage: {} ({:.1}%)\n", 
            format_size(snapshot.data_metrics.encrypted_size_bytes),
            if snapshot.data_metrics.total_size_bytes > 0 {
                (snapshot.data_metrics.encrypted_size_bytes as f64 / snapshot.data_metrics.total_size_bytes as f64) * 100.0
            } else {
                0.0
            }));
        report.push_str(&format!("  Versioned keys: {} ({:.1}%)\n", 
            snapshot.data_metrics.versioned_keys,
            if snapshot.data_metrics.total_keys > 0 {
                (snapshot.data_metrics.versioned_keys as f64 / snapshot.data_metrics.total_keys as f64) * 100.0
            } else {
                0.0
            }));
        report.push_str(&format!("  Total bytes written: {}\n", format_size(snapshot.data_metrics.bytes_written)));
        report.push_str(&format!("  Total bytes read: {}\n\n", format_size(snapshot.data_metrics.bytes_read)));
        
        // Version statistics
        report.push_str("VERSION METRICS:\n");
        report.push_str(&format!("  Total versions: {}\n", snapshot.version_metrics.total_versions));
        report.push_str(&format!("  Average versions per key: {:.2}\n", 
            snapshot.version_metrics.versions_per_key.get()));
        report.push_str(&format!("  Average version size: {}\n", 
            format_size(snapshot.version_metrics.version_size_bytes.get() as u64)));
        report.push_str(&format!("  Revert operations: {}\n", 
            snapshot.version_metrics.revert_operations));
        report.push_str(&format!("  Version storage overhead: {}\n", 
            format_size(snapshot.version_metrics.version_storage_overhead_bytes)));
        
        report
    }
} 