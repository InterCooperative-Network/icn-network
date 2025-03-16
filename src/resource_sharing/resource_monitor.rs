use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{self, Duration};
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};

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
            
            // Add any resource-specific metrics
            match &resource.resource_type {
                crate::resource_sharing::ResourceType::Computing { .. } => {
                    metrics.insert("cpu.temperature".to_string(), 55.0 + (now as f64 % 10.0));
                    metrics.insert("cpu.processes".to_string(), 120.0 + (now as f64 % 30.0));
                },
                crate::resource_sharing::ResourceType::Storage { .. } => {
                    metrics.insert("storage.iops".to_string(), 250.0 + (now as f64 % 100.0));
                    metrics.insert("storage.latency".to_string(), 5.0 + (now as f64 % 3.0));
                },
                crate::resource_sharing::ResourceType::Network { .. } => {
                    metrics.insert("network.packets".to_string(), 1500.0 + (now as f64 % 500.0));
                    metrics.insert("network.errors".to_string(), (now as f64 % 5.0));
                },
                _ => {}
            }
            
            // Create resource metrics
            let resource_metrics = ResourceMetrics {
                resource_id: resource.id.clone(),
                timestamp: now,
                metrics,
                metadata: HashMap::new(),
            };
            
            // Create resource utilization record
            let utilization = ResourceUtilization {
                resource_id: resource.id.clone(),
                cpu_utilization: cpu_util,
                memory_utilization: mem_util,
                storage_utilization: storage_util,
                network_utilization: network_util,
                timestamp: now,
            };
            
            // Store metrics
            let mut metrics_store = metrics_store.write().await;
            metrics_store
                .entry(resource.id.clone())
                .or_insert_with(Vec::new)
                .push(resource_metrics);
                
            // Store utilization
            let mut utilization_store = utilization_history.write().await;
            utilization_store
                .entry(resource.id.clone())
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
            if let Some(metrics_list) = metrics_store.get(&threshold.resource_id) {
                if let Some(latest_metrics) = metrics_list.last() {
                    if let Some(value) = latest_metrics.metrics.get(&threshold.metric) {
                        if *value >= threshold.critical_threshold {
                            // Handle critical threshold breach
                            Self::handle_threshold_breach(
                                &threshold, 
                                *value, 
                                "CRITICAL"
                            ).await?;
                        } else if *value >= threshold.warning_threshold {
                            // Handle warning threshold breach
                            Self::handle_threshold_breach(
                                &threshold,
                                *value,
                                "WARNING"
                            ).await?;
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
        println!("[{}] Resource {} - {} threshold breached: {} = {}", 
            level,
            threshold.resource_id,
            threshold.metric,
            threshold.metric,
            value);
            
        if let Some(action) = &threshold.action {
            match action {
                ThresholdAction::Notify => {
                    // In a real implementation, this would send a notification
                    println!("  Action: Notifying administrators");
                },
                ThresholdAction::ScaleUp => {
                    println!("  Action: Scaling up resource");
                    // Implement scaling logic
                },
                ThresholdAction::ScaleDown => {
                    println!("  Action: Scaling down resource");
                    // Implement scaling logic
                },
                ThresholdAction::Throttle => {
                    println!("  Action: Throttling resource usage");
                    // Implement throttling logic
                },
                ThresholdAction::Migrate => {
                    println!("  Action: Planning resource migration");
                    // Implement migration planning
                }
            }
        }
        
        Ok(())
    }
    
    // Clean up old metrics that are beyond the retention period
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
        for metrics_list in metrics.values_mut() {
            metrics_list.retain(|m| m.timestamp >= cutoff);
        }
        
        // Clean up utilization history
        let mut utilization = utilization_history.write().await;
        for util_list in utilization.values_mut() {
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
    
    // Get latest metrics for a resource
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
    
    // Get utilization history for a resource
    pub async fn get_utilization_history(
        &self,
        resource_id: &str,
        start_time: u64,
        end_time: u64
    ) -> Result<Vec<ResourceUtilization>, Box<dyn Error>> {
        let utilization = self.utilization_history.read().await;
        
        if let Some(history) = utilization.get(resource_id) {
            let filtered = history.iter()
                .filter(|u| u.timestamp >= start_time && u.timestamp <= end_time)
                .cloned()
                .collect();
            return Ok(filtered);
        }
        
        Ok(Vec::new())
    }
    
    // Get metric time series
    pub async fn get_metric_time_series(
        &self,
        resource_id: &str,
        metric: &str,
        start_time: u64,
        end_time: u64,
        interval: u64
    ) -> Result<MetricTimeSeries, Box<dyn Error>> {
        let metrics_store = self.metrics_store.read().await;
        
        let mut data_points = Vec::new();
        
        if let Some(metrics_list) = metrics_store.get(resource_id) {
            // Filter metrics within time range
            let filtered = metrics_list.iter()
                .filter(|m| m.timestamp >= start_time && m.timestamp <= end_time);
                
            // Group by interval and compute average
            let mut interval_map: HashMap<u64, Vec<f64>> = HashMap::new();
            
            for metric_point in filtered {
                if let Some(value) = metric_point.metrics.get(metric) {
                    let interval_key = (metric_point.timestamp / interval) * interval;
                    interval_map.entry(interval_key)
                        .or_insert_with(Vec::new)
                        .push(*value);
                }
            }
            
            // Compute averages for each interval
            for (timestamp, values) in interval_map {
                let avg = values.iter().sum::<f64>() / values.len() as f64;
                data_points.push(MetricDataPoint {
                    timestamp,
                    value: avg,
                });
            }
            
            // Sort by timestamp
            data_points.sort_by_key(|p| p.timestamp);
        }
        
        Ok(MetricTimeSeries {
            metric_name: metric.to_string(),
            resource_id: resource_id.to_string(),
            data_points,
            aggregation_interval: interval,
        })
    }
    
    // Sample CPU utilization (synthetic for demo)
    async fn sample_cpu_utilization(
        resource: &crate::resource_sharing::Resource
    ) -> Result<f64, Box<dyn Error>> {
        // In a real implementation, this would collect actual CPU metrics
        // For demonstration, we use a synthetic wave pattern with randomness
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        
        let base = 40.0; // baseline utilization
        let amplitude = 30.0; // amplitude of the wave
        let period = 300.0; // in seconds
        let jitter = (now % 17) as f64 * 0.5; // small random variation
        
        let value = base + amplitude * ((now as f64 / period) * 2.0 * std::f64::consts::PI).sin() + jitter;
        Ok(value.max(0.0).min(100.0)) // ensure within 0-100 range
    }
    
    // Sample memory utilization (synthetic for demo)
    async fn sample_memory_utilization(
        resource: &crate::resource_sharing::Resource
    ) -> Result<f64, Box<dyn Error>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        
        let base = 60.0; // baseline utilization
        let amplitude = 15.0; // amplitude
        let period = 600.0; // in seconds
        let jitter = (now % 13) as f64 * 0.3; // small random variation
        
        let value = base + amplitude * ((now as f64 / period) * 2.0 * std::f64::consts::PI).sin() + jitter;
        Ok(value.max(0.0).min(100.0))
    }
    
    // Sample storage utilization (synthetic for demo)
    async fn sample_storage_utilization(
        resource: &crate::resource_sharing::Resource
    ) -> Result<f64, Box<dyn Error>> {
        // Storage typically grows more linearly
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        
        let base = 50.0; // baseline utilization
        let growth = (now % 3600) as f64 * 0.005; // slow linear growth
        let jitter = (now % 7) as f64 * 0.1; // small random variation
        
        let value = base + growth + jitter;
        if value > 90.0 {
            // Simulate cleanup when reaching high utilization
            Ok(base)
        } else {
            Ok(value.max(0.0).min(100.0))
        }
    }
    
    // Sample network utilization (synthetic for demo)
    async fn sample_network_utilization(
        resource: &crate::resource_sharing::Resource
    ) -> Result<f64, Box<dyn Error>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        
        let base = 30.0; // baseline utilization
        let amplitude = 40.0; // amplitude (network tends to be bursty)
        let period = 120.0; // in seconds
        let jitter = (now % 29) as f64 * 1.2; // larger random variation for network
        
        let value = base + amplitude * ((now as f64 / period) * 2.0 * std::f64::consts::PI).sin() + jitter;
        Ok(value.max(0.0).min(100.0))
    }
    
    // Get list of resources to monitor
    async fn get_monitored_resources() -> Result<Vec<crate::resource_sharing::Resource>, Box<dyn Error>> {
        // In a real implementation, this would retrieve resources from the storage system
        // For demonstration, we return a mock list
        Ok(Vec::new()) // This would be populated in the real system
    }
} 