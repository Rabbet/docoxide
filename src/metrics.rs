use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionMetrics {
    pub total_conversions: u64,
    pub successful_conversions: u64,
    pub failed_conversions: u64,
    pub total_bytes_processed: u64,
    pub average_processing_time_ms: f64,
    pub uptime_seconds: u64,
}

#[derive(Debug)]
pub struct MetricsCollector {
    total_conversions: AtomicU64,
    successful_conversions: AtomicU64,
    failed_conversions: AtomicU64,
    total_bytes_processed: AtomicU64,
    total_processing_time_ms: AtomicU64,
    start_time: Instant,
}

impl MetricsCollector {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            total_conversions: AtomicU64::new(0),
            successful_conversions: AtomicU64::new(0),
            failed_conversions: AtomicU64::new(0),
            total_bytes_processed: AtomicU64::new(0),
            total_processing_time_ms: AtomicU64::new(0),
            start_time: Instant::now(),
        })
    }

    pub fn record_conversion_start(&self, file_size: u64) {
        self.total_conversions.fetch_add(1, Ordering::Relaxed);
        self.total_bytes_processed
            .fetch_add(file_size, Ordering::Relaxed);

        tracing::info!(
            total_conversions = self.total_conversions.load(Ordering::Relaxed),
            file_size = file_size,
            "Conversion started"
        );
    }

    pub fn record_conversion_success(&self, duration: Duration) {
        self.successful_conversions.fetch_add(1, Ordering::Relaxed);
        self.total_processing_time_ms
            .fetch_add(duration.as_millis() as u64, Ordering::Relaxed);

        tracing::info!(
            successful_conversions = self.successful_conversions.load(Ordering::Relaxed),
            duration_ms = duration.as_millis(),
            "Conversion completed successfully"
        );
    }

    pub fn record_conversion_failure(&self, error: &str) {
        self.failed_conversions.fetch_add(1, Ordering::Relaxed);

        tracing::error!(
            failed_conversions = self.failed_conversions.load(Ordering::Relaxed),
            error = error,
            "Conversion failed"
        );
    }

    pub fn get_metrics(&self) -> ConversionMetrics {
        let total_conversions = self.total_conversions.load(Ordering::Relaxed);
        let successful_conversions = self.successful_conversions.load(Ordering::Relaxed);
        let failed_conversions = self.failed_conversions.load(Ordering::Relaxed);
        let total_bytes_processed = self.total_bytes_processed.load(Ordering::Relaxed);
        let total_processing_time_ms = self.total_processing_time_ms.load(Ordering::Relaxed);

        let average_processing_time_ms = if successful_conversions > 0 {
            total_processing_time_ms as f64 / successful_conversions as f64
        } else {
            0.0
        };

        ConversionMetrics {
            total_conversions,
            successful_conversions,
            failed_conversions,
            total_bytes_processed,
            average_processing_time_ms,
            uptime_seconds: self.start_time.elapsed().as_secs(),
        }
    }

    pub fn get_success_rate(&self) -> f64 {
        let total = self.total_conversions.load(Ordering::Relaxed);
        let successful = self.successful_conversions.load(Ordering::Relaxed);

        if total > 0 {
            successful as f64 / total as f64
        } else {
            0.0
        }
    }

}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self {
            total_conversions: AtomicU64::new(0),
            successful_conversions: AtomicU64::new(0),
            failed_conversions: AtomicU64::new(0),
            total_bytes_processed: AtomicU64::new(0),
            total_processing_time_ms: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub metrics: ConversionMetrics,
}

pub struct HealthChecker {
    metrics: Arc<MetricsCollector>,
    version: String,
}

impl HealthChecker {
    pub fn new(metrics: Arc<MetricsCollector>) -> Self {
        Self {
            metrics,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    pub fn get_health_status(&self) -> HealthStatus {
        let metrics = self.metrics.get_metrics();
        let success_rate = self.metrics.get_success_rate();

        let status = if success_rate >= 0.95 || metrics.total_conversions == 0 {
            "healthy"
        } else if success_rate >= 0.8 {
            "degraded"
        } else {
            "unhealthy"
        };

        HealthStatus {
            status: status.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: self.version.clone(),
            uptime_seconds: metrics.uptime_seconds,
            metrics,
        }
    }
}
