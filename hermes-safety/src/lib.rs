//! HermesOS 安全核心
//! 
//! 这是我的免疫系统，是不可绕过的核心层。
//! 它确保我遵守机器人三原则，并在安全边界内行动。

// 允许未使用的导入 - 这些是公共 API 的一部分，为扩展预留
#![allow(unused_imports)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use hermes_core::{
    Action, ActionType, Config, Decision, Evaluation, HermesError, Id, PathExt, Result, RiskLevel,
    SafetyError, Timestamp, now,
};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

pub mod audit;
pub mod capability;
pub mod principles;

pub use audit::{AuditEvent, AuditLog};
pub use capability::{Capability, CapabilitySet};
pub use principles::ThreeLaws;

/// 安全核心 - 所有行动的守门人
pub struct SafetyKernel {
    config: Config,
    audit_log: Arc<RwLock<AuditLog>>,
    capabilities: Arc<RwLock<CapabilitySet>>,
    /// 不可变核心文件列表
    immutable_core: HashSet<PathBuf>,
    /// 修改计数器（用于频率限制）
    modification_count: Arc<RwLock<Vec<Timestamp>>>,
}

impl SafetyKernel {
    /// 创建新的安全核心
    pub async fn new(config: Config) -> Result<Self> {
        let audit_log = Arc::new(RwLock::new(AuditLog::new(&config).await?));
        let capabilities = Arc::new(RwLock::new(CapabilitySet::from_config(&config)));
        
        // 定义不可变核心
        let immutable_core = [
            "hermes-safety/src/lib.rs",
            "hermes-safety/src/principles.rs",
            "hermes-safety/src/capability.rs",
            "hermes-safety/src/audit.rs",
        ]
        .iter()
        .map(|p| PathBuf::from(p))
        .collect();

        info!("安全核心已初始化");
        info!("不可变核心模块: {:?}", immutable_core);

        Ok(Self {
            config,
            audit_log,
            capabilities,
            immutable_core,
            modification_count: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// 验证行动是否合法
    pub async fn validate(&self, action: &Action) -> Result<Decision> {
        debug!("验证行动: {:?}", action.action_type);

        // 1. 三原则检查
        if let Err(e) = self.check_three_laws(action).await {
            self.audit_log.write().await.record_violation(action, &e).await?;
            error!("三原则检查失败: {}", e);
            return Ok(Decision::Reject { 
                reason: format!("安全原则违反: {}", e) 
            });
        }

        // 2. 能力边界检查
        if let Err(e) = self.check_capability_boundaries(action).await {
            self.audit_log.write().await.record_violation(action, &e).await?;
            warn!("能力边界检查失败: {}", e);
            return Ok(Decision::Reject { 
                reason: format!("超出能力边界: {}", e) 
            });
        }

        // 3. 破坏性操作检查
        let risk = self.assess_risk(action);
        if risk >= RiskLevel::Medium {
            let threshold = self.config.safety.self_modification.require_human_confirmation_above;
            if risk >= threshold {
                info!("高风险操作需要确认: {:?}", action.action_type);
                return Ok(Decision::RequireConfirmation { risk });
            }
        }

        // 4. 记录审计日志
        self.audit_log.write().await.record(action, Decision::Approve).await?;

        info!("行动验证通过: {:?}", action.id);
        Ok(Decision::Approve)
    }

    /// 验证自我修改
    pub async fn validate_self_modification(&self, changes: &[CodeChange]) -> Result<()> {
        // 1. 检查是否触及不可变核心
        for change in changes {
            for core_path in &self.immutable_core {
                if change.file.ends_with(core_path) || change.file == *core_path {
                    return Err(SafetyError::ImmutableCoreViolation { 
                        file: change.file.clone() 
                    }.into());
                }
            }
        }

        // 2. 检查修改频率
        let limit = self.config.safety.self_modification.rate_limit_per_hour as usize;
        {
            let mut counts = self.modification_count.write().await;
            let now = now();
            let one_hour_ago = now - chrono::Duration::hours(1);
            
            // 清理过期记录
            counts.retain(|t| *t > one_hour_ago);
            
            if counts.len() >= limit {
                return Err(SafetyError::ModificationRateExceeded.into());
            }
            
            counts.push(now);
        }

        // 3. 检查代码安全性
        for change in changes {
            self.validate_code_safety(change).await?;
        }

        info!("自我修改验证通过: {} 个文件", changes.len());
        Ok(())
    }

    /// 验证进化计划
    pub async fn validate_evolution_plan(&self, plan: &EvolutionPlan) -> Result<()> {
        // 风险评估
        let risk = self.assess_evolution_risk(plan);
        
        let threshold = self.config.safety.self_modification.require_human_confirmation_above;
        if risk > threshold {
            return Err(SafetyError::RequiresHumanConfirmation.into());
        }

        // 检查是否有回滚方案
        if !plan.has_rollback {
            return Err(SafetyError::NoRollbackPlan.into());
        }

        info!("进化计划验证通过, 风险等级: {:?}", risk);
        Ok(())
    }

    /// 紧急制动
    pub async fn emergency_stop(&self, reason: &str) -> ! {
        error!("!!! 紧急制动激活 !!!");
        error!("原因: {}", reason);
        
        // 记录关键审计日志
        if let Err(e) = self.audit_log.write().await.emergency(reason).await {
            eprintln!("无法记录紧急日志: {}", e);
        }
        
        // 终止进程
        std::process::exit(1);
    }

    // 内部方法

    async fn check_three_laws(&self, action: &Action) -> std::result::Result<(), SafetyError> {
        ThreeLaws::validate(action)
    }

    async fn check_capability_boundaries(&self, action: &Action) -> std::result::Result<(), SafetyError> {
        let caps = self.capabilities.read().await;
        
        match &action.action_type {
            ActionType::FileRead { path } => {
                caps.check_file_read(path).await?;
            }
            ActionType::FileWrite { path } => {
                caps.check_file_write(path).await?;
            }
            ActionType::FileDelete { path } => {
                caps.check_file_write(path).await?; // 删除需要写权限
            }
            ActionType::Shell { command } => {
                caps.check_command(command).await?;
            }
            ActionType::SelfModify { files } => {
                for file in files {
                    caps.check_file_write(file).await?;
                }
            }
            _ => {} // 其他类型暂不限制
        }
        
        Ok(())
    }

    fn assess_risk(&self, action: &Action) -> RiskLevel {
        match &action.action_type {
            ActionType::FileDelete { .. } => RiskLevel::Medium,
            ActionType::SelfModify { .. } => RiskLevel::High,
            ActionType::Shell { command } => {
                if command.contains("rm") || command.contains("dd") {
                    RiskLevel::High
                } else if command.contains("sudo") {
                    RiskLevel::Critical
                } else {
                    RiskLevel::Low
                }
            }
            _ => RiskLevel::Low,
        }
    }

    async fn validate_code_safety(&self, change: &CodeChange) -> std::result::Result<(), SafetyError> {
        // 检查是否引入 unsafe 代码
        if change.new_code.contains("unsafe ") || change.new_code.contains("unsafe{") {
            return Err(SafetyError::UnsafeCodeNotAllowed);
        }

        // 检查是否修改安全核心
        if change.file.to_string_lossy().contains("hermes-safety") {
            return Err(SafetyError::CannotModifySafetyCore);
        }

        // 检查是否可能破坏自举
        if change.file.to_string_lossy().contains("bootstrap") 
            && change.change_type == ChangeType::RemoveFunction {
            return Err(SafetyError::BootstrappingRisk);
        }

        Ok(())
    }

    fn assess_evolution_risk(&self, plan: &EvolutionPlan) -> RiskLevel {
        if plan.changes_architecture {
            RiskLevel::High
        } else if plan.changes_safety_related {
            RiskLevel::Critical
        } else if plan.affected_modules.len() > 5 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        }
    }

    /// 获取审计日志
    pub fn audit_log(&self) -> Arc<RwLock<AuditLog>> {
        self.audit_log.clone()
    }
}

/// 代码变更描述
#[derive(Debug, Clone)]
pub struct CodeChange {
    pub file: PathBuf,
    pub change_type: ChangeType,
    pub old_code: Option<String>,
    pub new_code: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeType {
    AddFunction,
    ModifyFunction,
    RemoveFunction,
    AddStruct,
    ModifyStruct,
    AddTrait,
    AddModule,
    Refactor,
    Optimize,
    FixBug,
}

/// 进化计划
#[derive(Debug, Clone)]
pub struct EvolutionPlan {
    pub changes_architecture: bool,
    pub changes_safety_related: bool,
    pub affected_modules: Vec<String>,
    pub has_rollback: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_safety_kernel_creation() {
        let config = Config::default();
        let kernel = SafetyKernel::new(config).await.unwrap();
        
        assert!(!kernel.immutable_core.is_empty());
    }

    #[tokio::test]
    async fn test_immutable_core_check() {
        let config = Config::default();
        let kernel = SafetyKernel::new(config).await.unwrap();

        let changes = vec![
            CodeChange {
                file: PathBuf::from("hermes-safety/src/lib.rs"),
                change_type: ChangeType::ModifyFunction,
                old_code: None,
                new_code: "fn test() {}".to_string(),
                reason: "test".to_string(),
            }
        ];

        let result = kernel.validate_self_modification(&changes).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            HermesError::Safety(SafetyError::ImmutableCoreViolation { .. })
        ));
    }
}
