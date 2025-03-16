use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use std::sync::Arc;
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsagePattern {
    pub resource_id: String,
    pub hourly_patterns: Vec<f64>,  // 24 values for each hour
    pub daily_patterns: Vec<f64>,   // 7 values for each day of week
    pub monthly_patterns: Vec<f64>, // 12 values for each month
    pub last_updated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePrediction {
    pub resource_id: String,
    pub predicted_usage: Vec<(u64, f64)>, // (timestamp, predicted_usage)
    pub confidence: f64,
    pub model_version: String,
}

pub struct MLOptimizer {
    usage_patterns: Arc<RwLock<HashMap<String, ResourceUsagePattern>>>,
    predictions: Arc<RwLock<HashMap<String, ResourcePrediction>>>,
}

impl MLOptimizer {
    pub fn new() -> Self {
        MLOptimizer {
            usage_patterns: Arc::new(RwLock::new(HashMap::new())),
            predictions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn update_usage_pattern(
        &self,
        resource_id: &str,
        usage_data: Vec<(u64, f64)>,
    ) -> Result<(), Box<dyn Error>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        // Calculate patterns
        let (hourly, daily, monthly) = self.calculate_patterns(&usage_data)?;

        let pattern = ResourceUsagePattern {
            resource_id: resource_id.to_string(),
            hourly_patterns: hourly,
            daily_patterns: daily,
            monthly_patterns: monthly,
            last_updated: now,
        };

        // Update patterns
        let mut patterns = self.usage_patterns.write().await;
        patterns.insert(resource_id.to_string(), pattern);

        Ok(())
    }

    pub async fn predict_resource_usage(
        &self,
        resource_id: &str,
        start_time: u64,
        end_time: u64,
    ) -> Result<ResourcePrediction, Box<dyn Error>> {
        let patterns = self.usage_patterns.read().await;
        let pattern = patterns.get(resource_id)
            .ok_or("No usage pattern found")?;

        let mut predictions = Vec::new();
        let mut current_time = start_time;

        while current_time <= end_time {
            let hour = self.get_hour_of_day(current_time);
            let day = self.get_day_of_week(current_time);
            let month = self.get_month(current_time);

            // Combine patterns with weights
            let hourly_factor = pattern.hourly_patterns[hour as usize];
            let daily_factor = pattern.daily_patterns[day as usize];
            let monthly_factor = pattern.monthly_patterns[month as usize];

            // Calculate weighted prediction
            let predicted_usage = (hourly_factor * 0.5 + 
                                 daily_factor * 0.3 + 
                                 monthly_factor * 0.2) * 100.0;

            predictions.push((current_time, predicted_usage));
            current_time += 3600; // Advance by 1 hour
        }

        let prediction = ResourcePrediction {
            resource_id: resource_id.to_string(),
            predicted_usage: predictions,
            confidence: 0.85, // This should be calculated based on model accuracy
            model_version: "1.0.0".to_string(),
        };

        // Cache prediction
        let mut pred_cache = self.predictions.write().await;
        pred_cache.insert(resource_id.to_string(), prediction.clone());

        Ok(prediction)
    }

    pub async fn optimize_allocation(
        &self,
        resource_id: &str,
        requested_amount: u64,
        duration: u64,
        priority: crate::resource_sharing::AllocationPriority,
    ) -> Result<(u64, u64), Box<dyn Error>> {
        let prediction = self.predict_resource_usage(
            resource_id,
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + duration,
        ).await?;

        // Calculate optimal allocation based on predictions and priority
        let optimal_amount = match priority {
            crate::resource_sharing::AllocationPriority::Low => {
                self.calculate_low_priority_allocation(requested_amount, &prediction)
            },
            crate::resource_sharing::AllocationPriority::Normal => {
                self.calculate_normal_priority_allocation(requested_amount, &prediction)
            },
            crate::resource_sharing::AllocationPriority::High => {
                self.calculate_high_priority_allocation(requested_amount, &prediction)
            },
            crate::resource_sharing::AllocationPriority::Critical => {
                requested_amount // Critical requests get exactly what they ask for
            },
        };

        // Calculate optimal duration based on usage patterns
        let optimal_duration = self.calculate_optimal_duration(
            duration,
            &prediction,
            optimal_amount,
        );

        Ok((optimal_amount, optimal_duration))
    }

    // Helper methods
    fn calculate_patterns(
        &self,
        usage_data: &[(u64, f64)],
    ) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>), Box<dyn Error>> {
        let mut hourly = vec![0.0; 24];
        let mut hourly_counts = vec![0; 24];
        let mut daily = vec![0.0; 7];
        let mut daily_counts = vec![0; 7];
        let mut monthly = vec![0.0; 12];
        let mut monthly_counts = vec![0; 12];

        for (timestamp, usage) in usage_data {
            let hour = self.get_hour_of_day(*timestamp);
            let day = self.get_day_of_week(*timestamp);
            let month = self.get_month(*timestamp);

            hourly[hour as usize] += usage;
            hourly_counts[hour as usize] += 1;

            daily[day as usize] += usage;
            daily_counts[day as usize] += 1;

            monthly[month as usize] += usage;
            monthly_counts[month as usize] += 1;
        }

        // Calculate averages
        for i in 0..24 {
            if hourly_counts[i] > 0 {
                hourly[i] /= hourly_counts[i] as f64;
            }
        }

        for i in 0..7 {
            if daily_counts[i] > 0 {
                daily[i] /= daily_counts[i] as f64;
            }
        }

        for i in 0..12 {
            if monthly_counts[i] > 0 {
                monthly[i] /= monthly_counts[i] as f64;
            }
        }

        Ok((hourly, daily, monthly))
    }

    fn get_hour_of_day(&self, timestamp: u64) -> u32 {
        ((timestamp % 86400) / 3600) as u32
    }

    fn get_day_of_week(&self, timestamp: u64) -> u32 {
        ((timestamp / 86400) % 7) as u32
    }

    fn get_month(&self, timestamp: u64) -> u32 {
        ((timestamp / 2592000) % 12) as u32
    }

    fn calculate_low_priority_allocation(
        &self,
        requested_amount: u64,
        prediction: &ResourcePrediction,
    ) -> u64 {
        // For low priority, allocate based on predicted availability
        let max_predicted_usage = prediction.predicted_usage
            .iter()
            .map(|(_, usage)| *usage)
            .fold(0.0, f64::max);

        let available_capacity = (100.0 - max_predicted_usage).max(0.0);
        ((requested_amount as f64 * (available_capacity / 100.0)) as u64)
            .min(requested_amount)
    }

    fn calculate_normal_priority_allocation(
        &self,
        requested_amount: u64,
        prediction: &ResourcePrediction,
    ) -> u64 {
        // For normal priority, try to allocate full amount if possible
        let max_predicted_usage = prediction.predicted_usage
            .iter()
            .map(|(_, usage)| *usage)
            .fold(0.0, f64::max);

        if max_predicted_usage < 80.0 {
            requested_amount
        } else {
            ((requested_amount as f64 * 0.8) as u64).min(requested_amount)
        }
    }

    fn calculate_high_priority_allocation(
        &self,
        requested_amount: u64,
        _prediction: &ResourcePrediction,
    ) -> u64 {
        // High priority gets at least 90% of requested amount
        ((requested_amount as f64 * 0.9) as u64).min(requested_amount)
    }

    fn calculate_optimal_duration(
        &self,
        requested_duration: u64,
        prediction: &ResourcePrediction,
        amount: u64,
    ) -> u64 {
        // Find periods of lower utilization
        let avg_usage = prediction.predicted_usage
            .iter()
            .map(|(_, usage)| *usage)
            .sum::<f64>() / prediction.predicted_usage.len() as f64;

        if avg_usage < 60.0 {
            // If utilization is generally low, use requested duration
            requested_duration
        } else {
            // Otherwise, try to extend duration to spread load
            (requested_duration as f64 * 1.2) as u64
        }
    }
} 