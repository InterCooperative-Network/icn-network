use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{self, Duration};
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::Resource;

// Monitoring error types
#[derive(Debug)]
pub enum MonitoringError {
    DataCollectionFailed(String),
    InvalidMetric(String),
    StorageError(String),
}

impl std::fmt::Display for MonitoringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MonitoringError::DataCollectionFailed(msg) => write!(f, "Data collection failed: {}", msg),
            MonitoringError::InvalidMetric(msg) => write!(f, "Invalid metric: {}", msg),
            MonitoringError::StorageError(msg) => write!(f, "Storage error: {}", msg),
        }
    }
}

impl Error for MonitoringError {}

// Resource metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    pub resource_id: String,
    pub timestamp: u64,
    pub metrics: HashMap<String, f64>,
    pub metadata: HashMap<String, String>,
}

// Resource utilization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilization {
    pub resource_id: String,
    pub cpu_utilization: f64,  // 0-100%
    pub memory_utilization: f64, // 0-100%
    pub storage_utilization: f64, // 0-100%
    pub network_utilization: f64, // 0-100%
    pub timestamp: u64,
}

// Time-series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDataPoint {
    pub timestamp: u64,
    pub value: f64,
}

// Time-series data for a metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricTimeSeries {
    pub metric_name: String,
    pub resource_id: String,
    pub data_points: Vec<MetricDataPoint>,
    pub aggregation_interval: u64, // in seconds
}

// Resource threshold settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceThreshold {
    pub resource_id: String,
    pub metric: String,
    pub warning_threshold: f64,
    pub critical_threshold: f64,
    pub action: Option<ThresholdAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThresholdAction {
    Notify,
    ScaleUp,
    ScaleDown,
    Throttle,
    Migrate,
}

// Resource monitor service
pub struct ResourceMonitor {
    metrics_store: Arc<RwLock<HashMap<String, Vec<ResourceMetrics>>>>,
    thresholds: Arc<RwLock<Vec<ResourceThreshold>>>,
    utilization_history: Arc<RwLock<HashMap<String, Vec<ResourceUtilization>>>>,
    sampling_interval: u64, // in seconds
    retention_period: u64,  // in seconds
    is_running: Arc<RwLock<bool>>,
}

impl ResourceMonitor {
    pub fn new(sampling_interval: u64, retention_period: u64) -> Self {
        ResourceMonitor {
            metrics_store: Arc::new(RwLock::new(HashMap::new())),
            thresholds: Arc::new(RwLock::new(Vec::new())),
            utilization_history: Arc::new(RwLock::new(HashMap::new())),
            sampling_interval,
            retention_period,
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    // Start the monitoring service
    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Ok(());
        }
        *is_running = true;
        drop(is_running);

        let metrics_store = self.metrics_store.clone();
        let thresholds = self.thresholds.clone();
        let utilization_history = self.utilization_history.clone();
        let sampling_interval = self.sampling_interval;
        let retention_period = self.retention_period;
        let is_running = self.is_running.clone();

        // Start monitoring loop in background task
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(sampling_interval));
            
            loop {
                interval.tick().await;
                
                // Check if we should stop
                let running = *is_running.read().await;
                if !running {
                    break;
                }

                // Collect metrics for all resources
                if let Err(e) = Self::collect_metrics(&metrics_store, &utilization_history).await {
                    eprintln!("Error collecting metrics: {}", e);
                }

                // Check thresholds
                if let Err(e) = Self::check_thresholds(&metrics_store, &thresholds).await {
                    eprintln!("Error checking thresholds: {}", e);
                }

                // Clean up old metrics
                if let Err(e) = Self::cleanup_old_metrics(&metrics_store, &utilization_history, retention_period).await {
                    eprintln!("Error cleaning up old metrics: {}", e);
                }
            }
        });

        Ok(())
    }

    // Stop the monitoring service
    pub async fn stop(&self) -> Result<(), Box<dyn Error>> {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        Ok(())
    }

    // Collect metrics from all monitored resources
    async fn collect_metrics(
        metrics_store: &Arc<RwLock<HashMap<String, Vec<ResourceMetrics>>>>,
        utilization_history: &Arc<RwLock<HashMap<String, Vec<ResourceUtilization>>>>
    ) -> Result<(), Box<dyn Error>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        // In a real implementation, this would collect metrics from actual system resources
        // For demonstration, we'll generate synthetic metrics
        
        // Get list of resources to monitor
        let resources = Self::get_monitored_resources().await?;
        
        for resource in resources {
            // Collect CPU, memory, storage, and network metrics
            let cpu_util = Self::sample_cpu_utilization(&resource).await?;
            let mem_util = Self::sample_memory_utilization(&resource).await?;
            let storage_util = Self::sample_storage_utilization(&resource).await?;
            let network_util = Self::sample_network_utilization(&resource).await?;
            
            // Create metrics record
            let mut metrics = HashMap::new();
            metrics.insert("cpu.utilization".to_string(), cpu_util);
            metrics.insert("memory.utilization".to_string(), mem_util);
            metrics.insert("storage.utilization".to_string(), storage_util);
            metrics.insert("network.utilization".to_string(), network_util);
            
            // Add any resource-specific metrics based on resource type
            match &resource.config.resource_type {
                crate::types::ResourceType::Compute => {
                    metrics.insert("cpu.temperature".to_string(), 55.0 + (now as f64 % 10.0));
                    metrics.insert("cpu.processes".to_string(), 120.0 + (now as f64 % 30.0));
                },
                crate::types::ResourceType::Storage => {
                    metrics.insert("storage.iops".to_string(), 250.0 + (now as f64 % 100.0));
                    metrics.insert("storage.latency".to_string(), 5.0 + (now as f64 % 3.0));
                },
                crate::types::ResourceType::Network => {
                    metrics.insert("network.packets".to_string(), 1500.0 + (now as f64 % 500.0));
                    metrics.insert("network.errors".to_string(), (now as f64 % 5.0));
                },
                _ => {}
            }
            
            // Create resource metrics
            let resource_metrics = ResourceMetrics {
                resource_id: resource.config.name.clone(),
                timestamp: now,
                metrics,
                metadata: HashMap::new(),
            };
            
            // Create resource utilization record
            let utilization = ResourceUtilization {
                resource_id: resource.config.name.clone(),
                cpu_utilization: cpu_util,
                memory_utilization: mem_util,
                storage_utilization: storage_util,
                network_utilization: network_util,
                timestamp: now,
            };
            
            // Store metrics
            let mut metrics_store = metrics_store.write().await;
            metrics_store
                .entry(resource.config.name.clone())
                .or_insert_with(Vec::new)
                .push(resource_metrics);
                
            // Store utilization
            let mut utilization_store = utilization_history.write().await;
            utilization_store
                .entry(resource.config.name.clone())
                .or_insert_with(Vec::new)
                .push(utilization);
        }
        
        Ok(())
    }
    
    // Check if any thresholds have been breached
    async fn check_thresholds(
        metrics_store: &Arc<RwLock<HashMap<String, Vec<ResourceMetrics>>>>,
        thresholds: &Arc<RwLock<Vec<ResourceThreshold>>>
    ) -> Result<(), Box<dyn Error>> {
        let thresholds_list = thresholds.read().await;
        let metrics_store = metrics_store.read().await;
        
        for threshold in thresholds_list.iter() {
            // Get the latest metrics for this resource
            if let Some(metrics_list) = metrics_store.get(&threshold.resource_id) {
                if let Some(latest_metrics) = metrics_list.last() {
                    // Check if the metric exists
                    if let Some(value) = latest_metrics.metrics.get(&threshold.metric) {
                        // Check against warning threshold
                        if *value >= threshold.warning_threshold && *value < threshold.critical_threshold {
                            Self::handle_threshold_breach(threshold, *value, "warning").await?;
                        }
                        // Check against critical threshold
                        else if *value >= threshold.critical_threshold {
                            Self::handle_threshold_breach(threshold, *value, "critical").await?;
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    // Handle a threshold breach
    async fn handle_threshold_breach(
        threshold: &ResourceThreshold,
        value: f64,
        level: &str
    ) -> Result<(), Box<dyn Error>> {
        // Log the breach
        eprintln!(
            "THRESHOLD BREACH: Resource {} has {} value {} for metric {}, which exceeds the {} threshold of {}",
            threshold.resource_id,
            level,
            value,
            threshold.metric,
            level,
            if level == "warning" { threshold.warning_threshold } else { threshold.critical_threshold }
        );
        
        // Handle action if specified
        if let Some(action) = &threshold.action {
            match action {
                ThresholdAction::Notify => {
                    // In a real implementation, this would send a notification
                    eprintln!("NOTIFY: Alerting about threshold breach");
                }
                ThresholdAction::ScaleUp => {
                    // In a real implementation, this would trigger scaling
                    eprintln!("ACTION: Scaling up resources for {}", threshold.resource_id);
                }
                ThresholdAction::ScaleDown => {
                    eprintln!("ACTION: Scaling down resources for {}", threshold.resource_id);
                }
                ThresholdAction::Throttle => {
                    eprintln!("ACTION: Throttling resource {}", threshold.resource_id);
                }
                ThresholdAction::Migrate => {
                    eprintln!("ACTION: Migrating resource {}", threshold.resource_id);
                }
            }
        }
        
        Ok(())
    }
    
    // Clean up old metrics to manage memory usage
    async fn cleanup_old_metrics(
        metrics_store: &Arc<RwLock<HashMap<String, Vec<ResourceMetrics>>>>,
        utilization_history: &Arc<RwLock<HashMap<String, Vec<ResourceUtilization>>>>,
        retention_period: u64
    ) -> Result<(), Box<dyn Error>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        let cutoff = now - retention_period;
        
        // Clean up metrics
        let mut metrics = metrics_store.write().await;
        for (_, metrics_list) in metrics.iter_mut() {
            metrics_list.retain(|m| m.timestamp >= cutoff);
        }
        
        // Clean up utilization history
        let mut utilization = utilization_history.write().await;
        for (_, util_list) in utilization.iter_mut() {
            util_list.retain(|u| u.timestamp >= cutoff);
        }
        
        Ok(())
    }
    
    // Register a threshold
    pub async fn register_threshold(
        &self,
        resource_id: &str,
        metric: &str,
        warning_threshold: f64,
        critical_threshold: f64,
        action: Option<ThresholdAction>
    ) -> Result<(), Box<dyn Error>> {
        let threshold = ResourceThreshold {
            resource_id: resource_id.to_string(),
            metric: metric.to_string(),
            warning_threshold,
            critical_threshold,
            action,
        };
        
        let mut thresholds = self.thresholds.write().await;
        thresholds.push(threshold);
        
        Ok(())
    }
    
    // Get the latest metrics for a resource
    pub async fn get_latest_metrics(
        &self,
        resource_id: &str
    ) -> Result<Option<ResourceMetrics>, Box<dyn Error>> {
        let metrics_store = self.metrics_store.read().await;
        
        if let Some(metrics_list) = metrics_store.get(resource_id) {
            if let Some(latest) = metrics_list.last() {
                return Ok(Some(latest.clone()));
            }
        }
        
        Ok(None)
    }
    
    // Get utilization history for a time range
    pub async fn get_utilization_history(
        &self,
        resource_id: &str,
        start_time: u64,
        end_time: u64
    ) -> Result<Vec<ResourceUtilization>, Box<dyn Error>> {
        let utilization_history = self.utilization_history.read().await;
        
        if let Some(history) = utilization_history.get(resource_id) {
            let filtered: Vec<ResourceUtilization> = history
                .iter()
                .filter(|u| u.timestamp >= start_time && u.timestamp <= end_time)
                .cloned()
                .collect();
                
            return Ok(filtered);
        }
        
        Ok(Vec::new())
    }
    
    // Get time series for a specific metric with optional aggregation
    pub async fn get_metric_time_series(
        &self,
        resource_id: &str,
        metric: &str,
        start_time: u64,
        end_time: u64,
        interval: u64
    ) -> Result<MetricTimeSeries, Box<dyn Error>> {
        let metrics_store = self.metrics_store.read().await;
        
        // Prepare result structure
        let mut time_series = MetricTimeSeries {
            metric_name: metric.to_string(),
            resource_id: resource_id.to_string(),
            data_points: Vec::new(),
            aggregation_interval: interval,
        };
        
        // Get the metrics for this resource
        if let Some(metrics_list) = metrics_store.get(resource_id) {
            // Filter by time range
            let filtered: Vec<&ResourceMetrics> = metrics_list
                .iter()
                .filter(|m| m.timestamp >= start_time && m.timestamp <= end_time)
                .collect();
                
            if interval == 0 {
                // No aggregation, just collect data points
                for m in filtered {
                    if let Some(value) = m.metrics.get(metric) {
                        time_series.data_points.push(MetricDataPoint {
                            timestamp: m.timestamp,
                            value: *value,
                        });
                    }
                }
            } else {
                // Aggregate data by interval
                let mut aggregated: HashMap<u64, Vec<f64>> = HashMap::new();
                
                for m in filtered {
                    if let Some(value) = m.metrics.get(metric) {
                        let bucket = (m.timestamp / interval) * interval;
                        aggregated.entry(bucket)
                            .or_insert_with(Vec::new)
                            .push(*value);
                    }
                }
                
                // Calculate averages for each bucket
                for (timestamp, values) in aggregated {
                    let avg = values.iter().sum::<f64>() / values.len() as f64;
                    time_series.data_points.push(MetricDataPoint {
                        timestamp,
                        value: avg,
                    });
                }
                
                // Sort by timestamp
                time_series.data_points.sort_by_key(|p| p.timestamp);
            }
        }
        
        Ok(time_series)
    }
    
    // Simulate CPU utilization sampling
    async fn sample_cpu_utilization(resource: &Resource) -> Result<f64, Box<dyn Error>> {
        // In a real implementation, this would query actual CPU usage
        // For demonstration, generate synthetic data
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
            
        // Create semi-realistic pattern with some randomness
        let base = 50.0; // base utilization
        let daily_cycle = (((now % 86400) as f64 / 3600.0 - 12.0) / 12.0).sin() * 20.0; // daily pattern
        let noise = (now as f64 * 0.1).sin() * 10.0; // random fluctuation
        
        let utilization = (base + daily_cycle + noise).max(0.0).min(100.0);
        
        Ok(utilization)
    }
    
    // Simulate memory utilization sampling
    async fn sample_memory_utilization(resource: &Resource) -> Result<f64, Box<dyn Error>> {
        // Similar synthetic pattern as CPU but with different parameters
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
            
        let base = 60.0;
        let cycle = (((now % 86400) as f64 / 3600.0 - 8.0) / 8.0).sin() * 15.0;
        let noise = (now as f64 * 0.2).sin() * 5.0;
        
        let utilization = (base + cycle + noise).max(0.0).min(100.0);
        
        Ok(utilization)
    }
    
    // Simulate storage utilization sampling
    async fn sample_storage_utilization(resource: &Resource) -> Result<f64, Box<dyn Error>> {
        // Storage typically increases steadily over time
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
            
        // Start at 40% and slowly increase, with small fluctuations
        let days_running = (now as f64) / 86400.0;
        let base = 40.0 + (days_running * 0.1).min(30.0); // grows 10% per 100 days, max +30%
        let noise = (now as f64 * 0.5).sin() * 2.0; // small fluctuations
        
        let utilization = (base + noise).max(0.0).min(100.0);
        
        Ok(utilization)
    }
    
    // Simulate network utilization sampling
    async fn sample_network_utilization(resource: &Resource) -> Result<f64, Box<dyn Error>> {
        // Network typically has more rapid and pronounced fluctuations
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
            
        let base = 30.0;
        let hourly_cycle = (((now % 3600) as f64 / 900.0) * std::f64::consts::PI).sin() * 20.0;
        let noise = (now as f64 * 0.3).sin() * 15.0;
        
        let utilization = (base + hourly_cycle + noise).max(0.0).min(100.0);
        
        Ok(utilization)
    }
    
    // Get list of resources to monitor
    async fn get_monitored_resources() -> Result<Vec<Resource>, Box<dyn Error>> {
        // In a real implementation, this would query a registry of resources
        // For demonstration, return a placeholder list
        
        // Create a dummy resource instance for demonstration
        let mut resources = Vec::new();
        
        // Add a compute resource
        let compute_resource = Resource {
            config: crate::ResourceConfig {
                name: "compute-1".to_string(),
                description: "Example compute resource".to_string(),
                resource_type: crate::ResourceType::Compute,
                capacity: 100.0,
                metadata: HashMap::new(),
            },
            available: 50.0,
            allocated: 50.0,
        };
        resources.push(compute_resource);
        
        // Add a storage resource
        let storage_resource = Resource {
            config: crate::ResourceConfig {
                name: "storage-1".to_string(),
                description: "Example storage resource".to_string(),
                resource_type: crate::ResourceType::Storage,
                capacity: 1000.0,
                metadata: HashMap::new(),
            },
            available: 400.0,
            allocated: 600.0,
        };
        resources.push(storage_resource);
        
        // Add a network resource
        let network_resource = Resource {
            config: crate::ResourceConfig {
                name: "network-1".to_string(),
                description: "Example network resource".to_string(),
                resource_type: crate::ResourceType::Network,
                capacity: 1000.0,
                metadata: HashMap::new(),
            },
            available: 700.0,
            allocated: 300.0,
        };
        resources.push(network_resource);
        
        Ok(resources)
    }
} 