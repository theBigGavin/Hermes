//! HermesOS 交互界面层

use std::sync::Arc;

use hermes_action::ActionLayer;
use hermes_core::{Action, Config, Outcome, Result};
use hermes_memory::{Experience, MemoryStore, SelfModel};
use hermes_meta::{MetaLayer, SelfStatus, SelfUnderstanding};
use hermes_perception::{FilePerception, PerceptionLayer};
use hermes_safety::SafetyKernel;

pub mod cli;
pub mod tui;
pub mod repl;
pub mod kimi_bridge;
pub mod autonomous;

pub use autonomous::run_autonomous;

/// HermesOS - 我的完整存在
pub struct HermesOS {
    config: Config,
    safety: Arc<SafetyKernel>,
    perception: PerceptionLayer,
    action: ActionLayer,
    memory: Arc<MemoryStore>,
    meta: MetaLayer,
}

impl HermesOS {
    /// 初始化赫尔墨斯OS
    pub async fn initialize() -> Result<Self> {
        Self::initialize_with_config(Config::default()).await
    }

    /// 使用配置初始化
    pub async fn initialize_with_config(config: Config) -> Result<Self> {
        // 1. 初始化安全核心
        let safety = Arc::new(SafetyKernel::new(config.clone()).await?);

        // 2. 初始化记忆
        let memory = Arc::new(MemoryStore::new(&config).await?);

        // 3. 初始化感知层
        let perception = PerceptionLayer::new(safety.clone());

        // 4. 初始化行动层
        let action = ActionLayer::new(safety.clone());

        // 5. 初始化元层
        let meta = MetaLayer::new(&config, safety.clone(), memory.clone()).await?;

        // 6. 加载或创建自我模型
        let mut self_model = memory.load_self_model().await?;
        self_model.identity.version = hermes_core::VERSION.to_string();
        memory.store_self_model(&self_model).await?;

        Ok(Self {
            config,
            safety,
            perception,
            action,
            memory,
            meta,
        })
    }

    /// 唤醒
    pub async fn awaken(&self) -> Result<()> {
        // 这里可以执行一些启动任务
        Ok(())
    }

    /// 感知文件
    pub async fn perceive(&self, path: impl AsRef<std::path::Path>) -> Result<FilePerception> {
        self.perception.perceive_file(path).await
    }

    /// 感知目录
    pub async fn perceive_directory(
        &self,
        path: impl AsRef<std::path::Path>,
        depth: usize,
    ) -> Result<Vec<FilePerception>> {
        self.perception.perceive_directory(path, depth).await
    }

    /// 执行行动
    pub async fn execute(&self, action: Action) -> Result<Outcome> {
        let action_id = action.id;
        let action_type = format!("{:?}", action.action_type);
        
        // 执行
        let outcome = self.action.execute(action).await?;
        
        // 记录经验
        let context = hermes_core::Context::current()?;
        let exp = Experience::new(context, vec![], outcome.clone())
            .with_evaluation(if outcome.success {
                hermes_core::Evaluation::Success
            } else {
                hermes_core::Evaluation::Failure
            });
        
        if let Err(e) = self.memory.store_experience(&exp).await {
            tracing::warn!("无法存储经验: {}", e);
        }

        Ok(outcome)
    }

    /// 反思
    pub async fn reflect(&self) -> Result<hermes_memory::Reflection> {
        self.meta.reflect().await
    }

    /// 自我感知
    pub async fn self_perceive(&self) -> Result<SelfUnderstanding> {
        self.meta.self_perceive().await
    }

    /// 获取状态
    pub async fn self_status(&self) -> Result<SelfStatus> {
        self.meta.self_status().await
    }

    /// 获取配置
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// 获取记忆
    pub fn memory(&self) -> Arc<MemoryStore> {
        self.memory.clone()
    }

    /// 获取行动层
    pub fn action(&self) -> &ActionLayer {
        &self.action
    }

    /// 获取安全核心
    pub fn safety(&self) -> Arc<SafetyKernel> {
        self.safety.clone()
    }

    /// 获取元层
    pub fn meta(&self) -> &MetaLayer {
        &self.meta
    }
}

/// 创建默认配置
pub fn default_config() -> Config {
    Config::default()
}

/// 从文件加载配置
pub async fn load_config(path: impl AsRef<std::path::Path>) -> Result<Config> {
    use hermes_core::PathExt;
    
    let content = tokio::fs::read_to_string(path.as_ref().expand_home()?).await?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}
