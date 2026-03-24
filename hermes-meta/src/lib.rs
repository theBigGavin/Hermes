//! HermesOS 元层
//! 
//! 我思考我自己——自举、反思、进化的能力。

use std::path::PathBuf;
use std::sync::Arc;

use hermes_core::{Config, Context, Evaluation, HermesError, Id, Intent, Outcome, Result};
use hermes_memory::{Experience, MemoryStore, Reflection, SelfModel};
use hermes_safety::SafetyKernel;
use tracing::{debug, info, instrument, warn};

pub mod bootstrap;
pub mod reflection;

pub use bootstrap::BootstrapSystem;
pub use reflection::ReflectionSystem;

/// 元层 - 自举和反思的整合
pub struct MetaLayer {
    bootstrap: Arc<BootstrapSystem>,
    reflection: Arc<ReflectionSystem>,
    memory: Arc<MemoryStore>,
    /// 安全核心引用 - 预留用于未来自我修改时的安全验证
    #[allow(dead_code)]
    safety: Arc<SafetyKernel>,
}

impl MetaLayer {
    /// 创建元层
    pub async fn new(
        config: &Config,
        safety: Arc<SafetyKernel>,
        memory: Arc<MemoryStore>,
    ) -> Result<Self> {
        let bootstrap = Arc::new(BootstrapSystem::new(config).await?);
        let reflection = Arc::new(ReflectionSystem::new(memory.clone()));

        Ok(Self {
            bootstrap,
            reflection,
            memory,
            safety,
        })
    }

    /// 自我感知 - 读取并理解自己的代码
    #[instrument(skip(self))]
    pub async fn self_perceive(&self) -> Result<SelfUnderstanding> {
        info!("开始自我感知...");
        
        let understanding = self.bootstrap.understand_myself().await?;
        
        info!("自我感知完成");
        debug!("模块数量: {}", understanding.modules.len());
        
        Ok(understanding)
    }

    /// 执行反思
    #[instrument(skip(self))]
    pub async fn reflect(&self) -> Result<Reflection> {
        info!("开始反思...");
        
        let reflection = self.reflection.reflect().await?;
        
        // 存储反思
        self.memory.store_reflection(&reflection).await?;
        
        info!("反思完成: {} 成功, {} 失败", reflection.successes, reflection.failures);
        
        Ok(reflection)
    }

    /// 从经验学习
    #[instrument(skip(self))]
    pub async fn learn_from_experience(&self, experience: &Experience) -> Result<()> {
        debug!("从经验学习: {}", experience.id);
        
        // 存储经验
        self.memory.store_experience(experience).await?;
        
        // 如果失败，尝试提取教训
        if experience.evaluation == Evaluation::Failure {
            warn!("检测到失败，准备学习");
            // TODO: 从失败中提取技能
        }
        
        Ok(())
    }

    /// 获取自我状态
    pub async fn self_status(&self) -> Result<SelfStatus> {
        let model = self.memory.load_self_model().await?;
        let stats = self.memory.stats().await?;
        let recent_experiences = self.memory.recent_experiences(10).await?;
        
        Ok(SelfStatus {
            identity: model.identity,
            capabilities: model.capabilities,
            stats,
            recent_experiences,
        })
    }

    /// 尝试自我改进（简化版）
    #[instrument(skip(self))]
    pub async fn attempt_self_improvement(&self) -> Result<ImprovementResult> {
        info!("尝试自我改进...");
        
        // 1. 执行反思
        let reflection = self.reflect().await?;
        
        // 2. 检查是否需要改进
        if reflection.successes > reflection.failures * 2 {
            info!("当前状态良好，无需改进");
            return Ok(ImprovementResult::NoChangeNeeded);
        }
        
        // 3. 分析失败模式
        if !reflection.suggested_improvements.is_empty() {
            info!("发现改进机会: {:?}", reflection.suggested_improvements);
            
            // 目前只是记录建议，实际修改需要人工确认
            return Ok(ImprovementResult::SuggestionsGenerated(
                reflection.suggested_improvements
            ));
        }
        
        Ok(ImprovementResult::NoChangeNeeded)
    }

    pub fn bootstrap(&self) -> Arc<BootstrapSystem> {
        self.bootstrap.clone()
    }

    pub fn reflection(&self) -> Arc<ReflectionSystem> {
        self.reflection.clone()
    }
}

/// 自我理解 - 我对自己的认知
#[derive(Debug, Clone)]
pub struct SelfUnderstanding {
    pub modules: Vec<ModuleInfo>,
    pub total_lines: usize,
    pub public_apis: Vec<String>,
    pub unsafe_count: usize,
}

#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub name: String,
    pub path: PathBuf,
    pub lines: usize,
    pub functions: Vec<String>,
}

/// 自我状态
#[derive(Debug, Clone)]
pub struct SelfStatus {
    pub identity: hermes_memory::Identity,
    pub capabilities: hermes_memory::Capabilities,
    pub stats: hermes_memory::MemoryStats,
    pub recent_experiences: Vec<Experience>,
}

/// 改进结果
#[derive(Debug, Clone)]
pub enum ImprovementResult {
    NoChangeNeeded,
    SuggestionsGenerated(Vec<String>),
    ChangesApplied(Vec<String>),
    Error(String),
}

impl std::fmt::Display for SelfStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== HermesOS 状态 ===")?;
        writeln!(f, "名称: {}", self.identity.name)?;
        writeln!(f, "版本: {}", self.identity.version)?;
        writeln!(f, "目的: {}", self.identity.purpose)?;
        writeln!(f)?;
        writeln!(f, "--- 能力 ---")?;
        writeln!(f, "感知: {:?}", self.capabilities.can_perceive)?;
        writeln!(f, "行动: {:?}", self.capabilities.can_act)?;
        writeln!(f, "学习: {}", self.capabilities.can_learn)?;
        writeln!(f, "自修改: {}", self.capabilities.can_self_modify)?;
        writeln!(f)?;
        writeln!(f, "--- 记忆统计 ---")?;
        writeln!(f, "经验数: {}", self.stats.total_experiences)?;
        writeln!(f, "技能数: {}", self.stats.total_skills)?;
        writeln!(f, "反思数: {}", self.stats.total_reflections)?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 测试需要完整的依赖注入，可能需要集成测试
}
