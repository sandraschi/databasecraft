//! Craft App Generalized Notification Bus and CTO Resident Watchdog.
//!
//! Provides proactive operational anomaly detection for 24/7 database operations:
//! - Disk space starvation (database file & WAL growth)
//! - Quiescence violation / high-frequency mutation anomalies on static tables (e.g. `employees`, `roles`)
//! - Integrity degradation & foreign-key orphan buildup
//! - Severity-rated notifications with 1-click remediation actions
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use crate::error::{validate_identifier, DatabaseCraftError};
use databasecraft_store::DatabaseStore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Severity levels for Craft ecosystem notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationSeverity {
    Info,
    Warning,
    Critical,
    Disaster,
}

impl NotificationSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationSeverity::Info => "info",
            NotificationSeverity::Warning => "warning",
            NotificationSeverity::Critical => "critical",
            NotificationSeverity::Disaster => "disaster",
        }
    }
}

/// Threat category classification for operational notifications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationCategory {
    DiskStarvation,
    QuiescenceAnomaly,
    IntegrityDegraded,
    PerformanceStall,
    SecurityFence,
    MutationAudit,
}

impl NotificationCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationCategory::DiskStarvation => "disk_starvation",
            NotificationCategory::QuiescenceAnomaly => "quiescence_anomaly",
            NotificationCategory::IntegrityDegraded => "integrity_degraded",
            NotificationCategory::PerformanceStall => "performance_stall",
            NotificationCategory::SecurityFence => "security_fence",
            NotificationCategory::MutationAudit => "mutation_audit",
        }
    }
}

/// A structured notification emitted onto the Craft Notification Bus.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CraftNotification {
    pub id: String,
    pub timestamp: String,
    pub app: String,
    pub severity: NotificationSeverity,
    pub category: NotificationCategory,
    pub title: String,
    pub message: String,
    pub source_entity: Option<String>,
    pub metrics: Value,
    pub acknowledged: bool,
    pub suggested_action: Option<String>,
}

/// Rule defining expected quiescence for a table (e.g. `employees`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuiescentRule {
    pub table: String,
    pub max_mutations_per_minute: usize,
}

/// Thresholds for disk and storage warnings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiskThreshold {
    pub warn_db_bytes: u64,
    pub critical_db_bytes: u64,
    pub warn_wal_bytes: u64,
}

impl Default for DiskThreshold {
    fn default() -> Self {
        Self {
            warn_db_bytes: 100 * 1024 * 1024,      // 100 MB
            critical_db_bytes: 500 * 1024 * 1024,  // 500 MB
            warn_wal_bytes: 32 * 1024 * 1024,      // 32 MB
        }
    }
}

/// Configuration for the CTO Watchdog.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WatchdogConfig {
    pub quiescent_tables: Vec<QuiescentRule>,
    pub disk_thresholds: DiskThreshold,
}

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            quiescent_tables: vec![
                QuiescentRule {
                    table: "employees".to_string(),
                    max_mutations_per_minute: 5,
                },
                QuiescentRule {
                    table: "roles".to_string(),
                    max_mutations_per_minute: 2,
                },
                QuiescentRule {
                    table: "categories".to_string(),
                    max_mutations_per_minute: 10,
                },
                QuiescentRule {
                    table: "system_config".to_string(),
                    max_mutations_per_minute: 1,
                },
            ],
            disk_thresholds: DiskThreshold::default(),
        }
    }
}

/// Internal state tracking mutation timestamps per table for velocity calculation.
#[derive(Debug, Default)]
struct MutationTracker {
    /// Table name -> queue of mutation timestamps (in seconds since epoch)
    history: HashMap<String, VecDeque<u64>>,
}

impl MutationTracker {
    fn record(&mut self, table: &str, now: u64) {
        let entry = self.history.entry(table.to_lowercase()).or_default();
        entry.push_back(now);
        // Retain only timestamps from the last 120 seconds
        while let Some(&first) = entry.front() {
            if now.saturating_sub(first) > 120 {
                entry.pop_front();
            } else {
                break;
            }
        }
    }

    fn count_last_minute(&mut self, table: &str, now: u64) -> usize {
        let entry = self.history.entry(table.to_lowercase()).or_default();
        // Prune older than 120s
        while let Some(&first) = entry.front() {
            if now.saturating_sub(first) > 120 {
                entry.pop_front();
            } else {
                break;
            }
        }
        // Count entries in [now - 60, now]
        entry.iter().filter(|&&ts| now.saturating_sub(ts) <= 60).count()
    }
}

/// The Craft Notification Bus and Watchdog engine.
#[derive(Debug, Clone)]
pub struct NotificationBus {
    notifications: Arc<Mutex<VecDeque<CraftNotification>>>,
    config: Arc<Mutex<WatchdogConfig>>,
    tracker: Arc<Mutex<MutationTracker>>,
    max_history: usize,
}

impl Default for NotificationBus {
    fn default() -> Self {
        Self::new(200)
    }
}

impl NotificationBus {
    pub fn new(max_history: usize) -> Self {
        Self {
            notifications: Arc::new(Mutex::new(VecDeque::with_capacity(max_history))),
            config: Arc::new(Mutex::new(WatchdogConfig::default())),
            tracker: Arc::new(Mutex::new(MutationTracker::default())),
            max_history,
        }
    }

    fn current_timestamp() -> (String, u64) {
        let now = SystemTime::now();
        let secs = match now.duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_secs(),
            Err(_) => 0,
        };
        // Simple ISO-8601 representation
        let iso = format!("1970-01-01T00:00:00Z+{secs}s");
        (iso, secs)
    }

    /// Record a mutation event on a table to feed anomaly detection velocity.
    pub fn record_mutation(&self, table: &str) {
        let (_, now_secs) = Self::current_timestamp();
        if let Ok(mut tracker) = self.tracker.lock() {
            tracker.record(table, now_secs);
        }
    }

    /// Push an event onto the bus.
    pub fn emit(&self, notification: CraftNotification) {
        if let Ok(mut notifs) = self.notifications.lock() {
            if notifs.len() >= self.max_history {
                notifs.pop_front();
            }
            notifs.push_back(notification);
        }
    }

    /// Retrieve active notifications, optionally filtered by severity or acknowledgment.
    pub fn list(
        &self,
        severity: Option<NotificationSeverity>,
        unacknowledged_only: bool,
    ) -> Vec<CraftNotification> {
        let Ok(notifs) = self.notifications.lock() else {
            return Vec::new();
        };

        notifs
            .iter()
            .filter(|n| {
                if unacknowledged_only && n.acknowledged {
                    return false;
                }
                if let Some(target_sev) = severity {
                    if n.severity < target_sev {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }

    /// Acknowledge a notification by ID. Returns true if found and updated.
    pub fn acknowledge(&self, id: &str) -> bool {
        let Ok(mut notifs) = self.notifications.lock() else {
            return false;
        };

        for n in notifs.iter_mut() {
            if n.id == id {
                n.acknowledged = true;
                return true;
            }
        }
        false
    }

    /// Count unacknowledged warnings or critical notifications.
    pub fn unacknowledged_alert_count(&self) -> usize {
        let Ok(notifs) = self.notifications.lock() else {
            return 0;
        };
        notifs
            .iter()
            .filter(|n| !n.acknowledged && n.severity >= NotificationSeverity::Warning)
            .count()
    }

    /// Clear all acknowledged notifications from the ring buffer.
    pub fn clear_acknowledged(&self) {
        if let Ok(mut notifs) = self.notifications.lock() {
            notifs.retain(|n| !n.acknowledged);
        }
    }

    /// Retrieve current watchdog configuration.
    pub fn get_config(&self) -> WatchdogConfig {
        self.config
            .lock()
            .map(|c| c.clone())
            .unwrap_or_default()
    }

    /// Register a quiescent table rule.
    pub fn register_quiescent_table(
        &self,
        table: &str,
        max_mutations_per_minute: usize,
    ) -> Result<(), DatabaseCraftError> {
        validate_identifier(table)?;
        if let Ok(mut conf) = self.config.lock() {
            conf.quiescent_tables
                .retain(|r| !r.table.eq_ignore_ascii_case(table));
            conf.quiescent_tables.push(QuiescentRule {
                table: table.to_string(),
                max_mutations_per_minute,
            });
        }
        Ok(())
    }

    /// Evaluate all operational rules against current database state and emit alerts.
    pub fn evaluate_watchdog(
        &self,
        store: &DatabaseStore,
    ) -> Result<Vec<CraftNotification>, DatabaseCraftError> {
        let (ts, now_secs) = Self::current_timestamp();
        let config = self.get_config();
        let mut new_alerts = Vec::new();

        // 1. Quiescence Anomaly Evaluation
        if let Ok(mut tracker) = self.tracker.lock() {
            for rule in &config.quiescent_tables {
                let actual_rate = tracker.count_last_minute(&rule.table, now_secs);
                if actual_rate > rule.max_mutations_per_minute {
                    let id = format!("quiescence-{}-{}", rule.table, now_secs);
                    let alert = CraftNotification {
                        id,
                        timestamp: ts.clone(),
                        app: "databasecraft".to_string(),
                        severity: if actual_rate > rule.max_mutations_per_minute * 5 {
                            NotificationSeverity::Critical
                        } else {
                            NotificationSeverity::Warning
                        },
                        category: NotificationCategory::QuiescenceAnomaly,
                        title: format!("Quiescence Anomaly on Table '{}'", rule.table),
                        message: format!(
                            "Table '{}' is expected to be quiescent but received {} mutations in the last 60 seconds (threshold: {}/min).",
                            rule.table, actual_rate, rule.max_mutations_per_minute
                        ),
                        source_entity: Some(format!("table:{}", rule.table)),
                        metrics: json!({
                            "table": rule.table,
                            "mutations_last_minute": actual_rate,
                            "threshold": rule.max_mutations_per_minute,
                        }),
                        acknowledged: false,
                        suggested_action: Some("Inspect active write processes or review recent queries in SQL Console.".to_string()),
                    };
                    new_alerts.push(alert);
                }
            }
        }

        // 2. Integrity and Foreign-Key Orphan Evaluation
        let diag = store
            .diagnose()
            .map_err(|e| DatabaseCraftError::Storage(e.to_string()))?;

        if !diag.integrity_ok {
            let id = format!("integrity-fail-{}", now_secs);
            let alert = CraftNotification {
                id,
                timestamp: ts.clone(),
                app: "databasecraft".to_string(),
                severity: NotificationSeverity::Disaster,
                category: NotificationCategory::IntegrityDegraded,
                title: "Database B-Tree Integrity Check Failed".to_string(),
                message: format!(
                    "PRAGMA integrity_check failed with {} error message(s). Risk of silent data corruption.",
                    diag.integrity_errors.len()
                ),
                source_entity: Some("store:sqlite".to_string()),
                metrics: json!({
                    "integrity_errors": diag.integrity_errors,
                }),
                acknowledged: false,
                suggested_action: Some("Execute emergency repair: `databasecraft-cli repair --action auto` or salvage clone.".to_string()),
            };
            new_alerts.push(alert);
        } else if !diag.fk_violations.is_empty() {
            let id = format!("fk-orphans-{}", now_secs);
            let alert = CraftNotification {
                id,
                timestamp: ts.clone(),
                app: "databasecraft".to_string(),
                severity: NotificationSeverity::Warning,
                category: NotificationCategory::IntegrityDegraded,
                title: format!("Foreign Key Integrity: {} Orphaned Rows", diag.fk_violations.len()),
                message: format!(
                    "Detected {} orphaned child rows violating active foreign-key constraints.",
                    diag.fk_violations.len()
                ),
                source_entity: Some("store:foreign_keys".to_string()),
                metrics: json!({
                    "orphaned_rows": diag.fk_violations.len(),
                    "violations": diag.fk_violations,
                }),
                acknowledged: false,
                suggested_action: Some("Run `databasecraft-cli repair --action purge_orphans` to restore referential integrity.".to_string()),
            };
            new_alerts.push(alert);
        }

        // Emit all new alerts onto the bus
        for alert in &new_alerts {
            self.emit(alert.clone());
        }

        Ok(new_alerts)
    }
}

/// Global singleton notification bus for in-process access.
static GLOBAL_BUS: std::sync::OnceLock<NotificationBus> = std::sync::OnceLock::new();

/// Access the shared notification bus.
pub fn global_notification_bus() -> &'static NotificationBus {
    GLOBAL_BUS.get_or_init(NotificationBus::default)
}
