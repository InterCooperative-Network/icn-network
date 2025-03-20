use std::collections::HashMap;
use std::error::Error;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::sync::Arc;

use crate::allocation::AllocationPriority;

// Struct to store usage data for prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsagePatternData {
    pub resource_id: String,
    pub time_series: Vec<(u64, f64)>, // (timestamp, usage percentage)
    pub predictions: HashMap<String, Vec<f64>>, // Maps time ranges to predicted usage
}

// ML Optimizer for resource allocation optimization
pub struct MLOptimizer {
    usage_patterns: Arc<RwLock<HashMap<String, UsagePatternData>>>,
}

impl MLOptimizer {
    pub fn new() -> Self {
        MLOptimizer {
            usage_patterns: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    // Optimize resource allocation based on historical usage patterns
    pub async fn optimize_allocation(
        &self,
        resource_id: &str,
        requested_amount: u64,
        requested_duration: u64,
        priority: AllocationPriority,
    ) -> Result<(u64, u64), Box<dyn Error>> {
        // Get usage patterns for this resource
        let usage_patterns = self.usage_patterns.read().await;
        let pattern = usage_patterns.get(resource_id);
        
        // If we don't have usage data yet, return the requested values
        if pattern.is_none() || pattern.unwrap().time_series.len() < 10 {
            return Ok((requested_amount, requested_duration));
        }
        
        let pattern = pattern.unwrap();
        
        // Adjust based on time of day and historical usage
        let (amount_factor, duration_factor) = self.calculate_optimization_factors(
            pattern,
            requested_amount,
            requested_duration,
            &priority,
        )?;
        
        // Apply factors to calculate optimized values
        let optimal_amount = (requested_amount as f64 * amount_factor) as u64;
        let optimal_duration = (requested_duration as f64 * duration_factor) as u64;
        
        // Ensure we don't drop below 50% of requested resources
        let optimal_amount = optimal_amount.max(requested_amount / 2);
        
        // For high priority, don't reduce the amount at all
        let optimal_amount = match priority {
            AllocationPriority::High => requested_amount,
            _ => optimal_amount,
        };
        
        Ok((optimal_amount, optimal_duration))
    }
    
    // Calculate optimization factors based on usage patterns
    fn calculate_optimization_factors(
        &self,
        pattern: &UsagePatternData,
        requested_amount: u64,
        requested_duration: u64,
        priority: &AllocationPriority,
    ) -> Result<(f64, f64), Box<dyn Error>> {
        // Get current hour of day (0-23)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        let hour = (now % 86400) / 3600;
        
        // Check if we have predictions for this time window
        let time_window = format!("hour_{}", hour);
        let usage_prediction = pattern.predictions.get(&time_window)
            .map(|pred| pred.iter().sum::<f64>() / pred.len() as f64)
            .unwrap_or(0.8); // Default to 80% if no prediction available
        
        // Calculate optimization factors based on predicted usage and priority
        let amount_factor = match priority {
            AllocationPriority::High => 1.0, // Don't reduce for high priority
            AllocationPriority::Normal => {
                if usage_prediction > 0.9 {
                    0.8  // Reduce by 20% during high usage
                } else if usage_prediction > 0.7 {
                    0.9  // Reduce by 10% during moderate usage
                } else {
                    1.0  // No reduction during low usage
                }
            },
            AllocationPriority::Low => {
                if usage_prediction > 0.9 {
                    0.6  // Reduce by 40% during high usage
                } else if usage_prediction > 0.7 {
                    0.8  // Reduce by 20% during moderate usage
                } else {
                    0.9  // Reduce by 10% even during low usage
                }
            },
        };
        
        // For duration factor, apply similar logic but with different thresholds
        let duration_factor = match priority {
            AllocationPriority::High => 1.0,
            AllocationPriority::Normal => {
                if usage_prediction > 0.9 {
                    0.7
                } else if usage_prediction > 0.7 {
                    0.8
                } else {
                    1.0
                }
            },
            AllocationPriority::Low => {
                if usage_prediction > 0.9 {
                    0.5
                } else if usage_prediction > 0.7 {
                    0.7
                } else {
                    0.8
                }
            },
        };
        
        Ok((amount_factor, duration_factor))
    }
    
    // Update usage pattern with new data point
    pub async fn update_usage_pattern(
        &self,
        resource_id: &str,
        usage_data: Vec<(u64, f64)>,
    ) -> Result<(), Box<dyn Error>> {
        let mut usage_patterns = self.usage_patterns.write().await;
        
        // Get or create pattern data
        let pattern = usage_patterns.entry(resource_id.to_string())
            .or_insert(UsagePatternData {
                resource_id: resource_id.to_string(),
                time_series: Vec::new(),
                predictions: HashMap::new(),
            });
        
        // Add new data points
        pattern.time_series.extend(usage_data);
        
        // Keep only the last 1000 data points
        if pattern.time_series.len() > 1000 {
            pattern.time_series = pattern.time_series.split_off(pattern.time_series.len() - 1000);
        }
        
        // Update predictions based on new data
        self.update_predictions(pattern)?;
        
        Ok(())
    }
    
    // Update predictions based on time series data
    fn update_predictions(&self, pattern: &mut UsagePatternData) -> Result<(), Box<dyn Error>> {
        // Group data by hour of day
        let mut hourly_data: HashMap<u64, Vec<f64>> = HashMap::new();
        
        for (timestamp, usage) in &pattern.time_series {
            let hour = (timestamp % 86400) / 3600;
            hourly_data.entry(hour).or_insert_with(Vec::new).push(*usage);
        }
        
        // Calculate average usage for each hour
        for (hour, usages) in hourly_data {
            let avg_usage: f64 = usages.iter().sum::<f64>() / usages.len() as f64;
            pattern.predictions.insert(format!("hour_{}", hour), vec![avg_usage]);
        }
        
        Ok(())
    }
    
    // Predict usage for a future time window
    pub async fn predict_usage(
        &self,
        resource_id: &str,
        future_time: u64,
        window_size: u64,
    ) -> Result<Vec<f64>, Box<dyn Error>> {
        let usage_patterns = self.usage_patterns.read().await;
        let pattern = usage_patterns.get(resource_id)
            .ok_or("No usage pattern data available for resource")?;
        
        // Calculate hour range for prediction
        let start_hour = (future_time % 86400) / 3600;
        let hours_count = (window_size + 3599) / 3600; // Ceiling division
        
        let mut predictions = Vec::new();
        
        for i in 0..hours_count {
            let hour = (start_hour + i) % 24;
            let time_window = format!("hour_{}", hour);
            
            if let Some(pred) = pattern.predictions.get(&time_window) {
                predictions.push(pred[0]); // Use first prediction for now
            } else {
                // Use average if no prediction for this hour
                let avg: f64 = pattern.predictions.values()
                    .flat_map(|v| v.iter())
                    .sum::<f64>() / 
                    pattern.predictions.values()
                        .flat_map(|v| v.iter())
                        .count() as f64;
                predictions.push(avg);
            }
        }
        
        Ok(predictions)
    }
} 