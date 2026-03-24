//! 审计日志系统
//! 
//! 记录所有行动，用于安全分析、故障排查和自我反思。

use std::path::PathBuf;

use hermes_core::{Action, Decision, HermesError, Id, PathExt, Result, SafetyError, Timestamp, now};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::{debug, error, info};

/// 审计事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Id,
    pub timestamp: Timestamp,
    pub event_type: EventType,
    pub action_id: Option<Id>,
    pub details: serde_json::Value,
    pub outcome: EventOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    ActionValidated,
    ActionRejected,
    ActionExecuted,
    ViolationDetected,
    EmergencyStop,
    SelfModification,
    Evolution,
    ConfigChanged,
    SystemStartup,
    SystemShutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventOutcome {
    Success,
    Failure(String),
    Warning(String),
}

/// 审计日志
pub struct AuditLog {
    log_path: PathBuf,
    events: Vec<AuditEvent>,
    max_memory_events: usize,
}

impl AuditLog {
    pub async fn new(config: &hermes_core::Config) -> Result<Self> {
        let log_path = PathBuf::from(&config.memory.persistence_path)
            .expand_home()?
            .parent()
            .ok_or_else(|| HermesError::Config("无效的记忆路径".to_string()))?
            .join("audit.log");

        // 确保目录存在
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        info!("审计日志初始化: {:?}", log_path);

        Ok(Self {
            log_path,
            events: Vec::new(),
            max_memory_events: 10000,
        })
    }

    /// 记录行动
    pub async fn record(&mut self, action: &Action, decision: Decision) -> Result<()> {
        let event = AuditEvent {
            id: Id::new(),
            timestamp: now(),
            event_type: match decision {
                Decision::Approve => EventType::ActionValidated,
                Decision::Reject { .. } => EventType::ActionRejected,
                _ => EventType::ActionValidated,
            },
            action_id: Some(action.id),
            details: serde_json::json!({
                "action_type": format!("{:?}", action.action_type),
                "decision": format!("{:?}", decision),
            }),
            outcome: match decision {
                Decision::Approve => EventOutcome::Success,
                Decision::Reject { reason } => EventOutcome::Failure(reason),
                _ => EventOutcome::Success,
            },
        };

        self.persist_event(&event).await?;
        self.add_to_memory(event);

        Ok(())
    }

    /// 记录违规
    pub async fn record_violation(
        &mut self,
        action: &Action,
        violation: &SafetyError,
    ) -> Result<()> {
        let event = AuditEvent {
            id: Id::new(),
            timestamp: now(),
            event_type: EventType::ViolationDetected,
            action_id: Some(action.id),
            details: serde_json::json!({
                "action_type": format!("{:?}", action.action_type),
                "violation": format!("{}", violation),
            }),
            outcome: EventOutcome::Failure(violation.to_string()),
        };

        error!("安全违规: {}", violation);
        self.persist_event(&event).await?;
        self.add_to_memory(event);

        Ok(())
    }

    /// 记录紧急制动
    pub async fn emergency(&mut self, reason: &str) -> Result<()> {
        let event = AuditEvent {
            id: Id::new(),
            timestamp: now(),
            event_type: EventType::EmergencyStop,
            action_id: None,
            details: serde_json::json!({
                "reason": reason,
            }),
            outcome: EventOutcome::Failure("紧急停止".to_string()),
        };

        // 紧急日志同步写入
        self.persist_event_sync(&event)?;

        Ok(())
    }

    /// 记录自我修改
    pub async fn record_self_modification(
        &mut self,
        files: &[PathBuf],
        success: bool,
    ) -> Result<()> {
        let event = AuditEvent {
            id: Id::new(),
            timestamp: now(),
            event_type: EventType::SelfModification,
            action_id: None,
            details: serde_json::json!({
                "files": files.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>(),
            }),
            outcome: if success {
                EventOutcome::Success
            } else {
                EventOutcome::Failure("修改失败".to_string())
            },
        };

        self.persist_event(&event).await?;
        self.add_to_memory(event);

        Ok(())
    }

    /// 查询最近事件
    pub fn recent_events(&self, count: usize) -> Vec<&AuditEvent> {
        self.events.iter().rev().take(count).collect()
    }

    /// 查询违规事件
    pub fn violations(&self) -> Vec<&AuditEvent> {
        self.events
            .iter()
            .filter(|e| matches!(e.event_type, EventType::ViolationDetected))
            .collect()
    }

    /// 获取统计数据
    pub fn stats(&self) -> AuditStats {
        let total = self.events.len();
        let violations = self.violations().len();
        let successes = self
            .events
            .iter()
            .filter(|e| matches!(e.outcome, EventOutcome::Success))
            .count();

        AuditStats {
            total_events: total,
            violations,
            successes,
            failures: total - successes - violations,
        }
    }

    // 内部方法

    async fn persist_event(&self, event: &AuditEvent) -> Result<()> {
        let line = serde_json::to_string(event)?;
        
        use tokio::io::AsyncWriteExt;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .await?;
        
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;

        debug!("审计事件已记录: {}", event.id);
        Ok(())
    }

    fn persist_event_sync(&self, event: &AuditEvent) -> Result<()> {
        use std::io::Write;
        let line = serde_json::to_string(event)?;
        
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;
        
        writeln!(file, "{}", line)?;
        file.flush()?;

        Ok(())
    }

    fn add_to_memory(&mut self, event: AuditEvent) {
        self.events.push(event);
        
        // 清理旧事件
        if self.events.len() > self.max_memory_events {
            let to_remove = self.events.len() - self.max_memory_events;
            self.events.drain(0..to_remove);
        }
    }
}

/// 审计统计
#[derive(Debug, Clone)]
pub struct AuditStats {
    pub total_events: usize,
    pub violations: usize,
    pub successes: usize,
    pub failures: usize,
}

impl AuditStats {
    pub fn success_rate(&self) -> f64 {
        if self.total_events == 0 {
            0.0
        } else {
            self.successes as f64 / self.total_events as f64
        }
    }

    pub fn violation_rate(&self) -> f64 {
        if self.total_events == 0 {
            0.0
        } else {
            self.violations as f64 / self.total_events as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hermes_core::ActionType;

    #[tokio::test]
    async fn test_audit_log() {
        let config = hermes_core::Config::default();
        let mut log = AuditLog::new(&config).await.unwrap();

        let action = Action::new(ActionType::FileRead {
            path: PathBuf::from("test.txt"),
        });

        log.record(&action, Decision::Approve).await.unwrap();

        assert_eq!(log.stats().total_events, 1);
        assert_eq!(log.stats().successes, 1);
    }
}
