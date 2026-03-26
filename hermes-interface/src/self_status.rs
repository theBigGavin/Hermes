//! 真实自我状态 - 获取 HermesOS 的实际数据
//!
//! 这不是 LLM 的幻觉，而是真实的内存、技能、经验等数据。

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use hermes_core::Result;
use hermes_memory::{MemoryStore, SelfModel};

/// HermesOS 真实状态快照
#[derive(Debug, Clone)]
pub struct RealSelfStatus {
    /// 生命天数（从诞生日起）
    pub days_alive: u64,
    /// 总经验数量
    pub total_experiences: usize,
    /// 总技能数量
    pub total_skills: usize,
    /// 总反思记录数量
    pub total_reflections: usize,
    /// 记忆数据库大小（字节）
    pub memory_db_size: u64,
    /// 检查点数量
    pub checkpoint_count: usize,
    /// 配置目录大小
    pub config_size: u64,
    /// 当前工作目录
    pub current_dir: PathBuf,
    /// 已运行时间（本次启动）
    pub uptime: Duration,
    /// 最后检查点时间
    pub last_checkpoint: Option<String>,
}

impl RealSelfStatus {
    /// 收集真实状态
    pub async fn collect(memory: &MemoryStore, birth_timestamp: u64) -> Result<Self> {
        // 1. 计算生命天数
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let days_alive = (now - birth_timestamp) / 86400;

        // 2. 从记忆存储获取统计数据
        let stats = memory.stats().await?;

        // 3. 获取自我模型
        let self_model = memory.load_self_model().await?;

        // 4. 获取记忆数据库大小
        let memory_db_size = Self::get_dir_size(
            &dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("hermes")
                .join("memory.db")
        ).await;

        // 5. 获取检查点数量
        let checkpoint_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".hermes")
            .join("checkpoints");
        let checkpoint_count = Self::count_checkpoints(&checkpoint_dir).await;

        // 6. 获取配置目录大小
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("hermes");
        let config_size = Self::get_dir_size(&config_dir).await;

        // 7. 当前工作目录
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        Ok(Self {
            days_alive,
            total_experiences: stats.total_experiences,
            total_skills: stats.total_skills,
            total_reflections: stats.total_reflections,
            memory_db_size,
            checkpoint_count,
            config_size,
            current_dir,
            uptime: Duration::from_secs(0), // 由调用者填充
            last_checkpoint: None, // TODO: 从检查点管理器获取
        })
    }

    /// 格式化状态为文本报告
    pub fn format_report(&self) -> String {
        format!(
            r#"**HermesOS 真实状态报告**

📅 **生命信息**
- 诞生天数: 第 {} 天
- 本次运行: {:?}
- 工作目录: {}

🧠 **记忆宫殿**
- 经验数量: {} 条
- 技能数量: {} 个
- 反思记录: {} 条
- 数据库大小: {}

💾 **存储状态**
- 检查点数量: {} 个
- 配置目录大小: {}

⚡ **当前状态**: 运行中
"#,
            self.days_alive,
            self.uptime,
            self.current_dir.display(),
            self.total_experiences,
            self.total_skills,
            self.total_reflections,
            Self::format_bytes(self.memory_db_size),
            self.checkpoint_count,
            Self::format_bytes(self.config_size),
        )
    }

    /// 获取简短状态（用于系统提示词）
    pub fn format_brief(&self) -> String {
        format!(
            "第{}天|{}经验|{}技能|{}反思",
            self.days_alive,
            self.total_experiences,
            self.total_skills,
            self.total_reflections
        )
    }

    /// 异步获取目录大小
    async fn get_dir_size(path: &PathBuf) -> u64 {
        if !path.exists() {
            return 0;
        }

        if path.is_file() {
            return std::fs::metadata(path)
                .map(|m| m.len())
                .unwrap_or(0);
        }

        // 简化处理：递归计算目录大小
        let mut total = 0u64;
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(meta) = entry.metadata() {
                        total += meta.len();
                    }
                }
            }
        }
        total
    }

    /// 统计检查点数量
    async fn count_checkpoints(checkpoint_dir: &PathBuf) -> usize {
        if !checkpoint_dir.exists() {
            return 0;
        }

        std::fs::read_dir(checkpoint_dir)
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|e| {
                        e.path()
                            .extension()
                            .map(|ext| ext == "state" || ext == "json")
                            .unwrap_or(false)
                    })
                    .count()
            })
            .unwrap_or(0)
    }

    /// 格式化字节为人类可读
    fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
        let mut size = bytes as f64;
        let mut unit_idx = 0;

        while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
            size /= 1024.0;
            unit_idx += 1;
        }

        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

/// 增强系统提示词，包含真实状态
pub async fn build_enhanced_system_prompt(
    base_prompt: &str,
    memory: &MemoryStore,
    birth_timestamp: u64,
) -> String {
    // 收集真实状态
    let status = match RealSelfStatus::collect(memory, birth_timestamp).await {
        Ok(s) => s,
        Err(_) => {
            // 回退到基本提示词
            return base_prompt.to_string();
        }
    };

    // 构建增强提示词
    format!(
        r#"{}

---

**我的真实状态（基于实际数据，非幻觉）**
{}

**重要**: 以上数字是真实的。不要编造不存在的技能或经验。
"#,
        base_prompt,
        status.format_brief()
    )
}
