//! HermesOS 记忆宫殿
//! 
//! 我的记忆系统，保存我的经验、技能和自我认知。

#![allow(unused_imports)] // 公共 API 设计预留

use std::collections::HashMap;
use std::path::PathBuf;

use hermes_core::{Action, Config, Context, Evaluation, HermesError, Id, Intent, Outcome, PathExt, Result, Timestamp, now};
use serde::{Deserialize, Serialize};
use sled::Db;
use tracing::{debug, info, warn};

pub mod skill;
pub mod checkpoint;

pub use skill::{Skill, SkillId, SkillRegistry};
pub use checkpoint::{CheckpointManager, Checkpoint, FullState};

/// 经验 - 一次完整的感知-思考-行动循环
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub id: Id,
    pub timestamp: Timestamp,
    pub context: Context,
    pub intent: Option<Intent>,
    pub actions: Vec<Action>,
    pub outcome: Outcome,
    pub evaluation: Evaluation,
    pub reflection: Option<String>,
    pub extracted_skill: Option<SkillId>,
}

impl Experience {
    pub fn new(context: Context, actions: Vec<Action>, outcome: Outcome) -> Self {
        Self {
            id: Id::new(),
            timestamp: now(),
            context,
            intent: None,
            actions,
            outcome,
            evaluation: Evaluation::Success,
            reflection: None,
            extracted_skill: None,
        }
    }

    pub fn with_intent(mut self, intent: Intent) -> Self {
        self.intent = Some(intent);
        self
    }

    pub fn with_evaluation(mut self, eval: Evaluation) -> Self {
        self.evaluation = eval;
        self
    }

    pub fn with_reflection(mut self, reflection: impl Into<String>) -> Self {
        self.reflection = Some(reflection.into());
        self
    }
}

/// 记忆存储
pub struct MemoryStore {
    db: Db,
    skill_registry: SkillRegistry,
}

impl MemoryStore {
    /// 创建记忆存储
    pub async fn new(config: &Config) -> Result<Self> {
        let path = PathBuf::from(&config.memory.persistence_path).expand_home()?;
        
        // 确保目录存在
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        info!("初始化记忆存储: {:?}", path);

        let db = sled::open(&path)?;
        let skill_registry = SkillRegistry::new(&db);

        Ok(Self { db, skill_registry })
    }

    /// 存储经验
    pub async fn store_experience(&self, exp: &Experience) -> Result<()> {
        let key = format!("exp:{}", exp.id);
        let value = serde_json::to_vec(exp)?;
        
        self.db.insert(key.as_bytes(), value)?;
        self.db.flush_async().await?;

        debug!("经验已存储: {}", exp.id);
        Ok(())
    }

    /// 检索经验
    pub async fn get_experience(&self, id: Id) -> Result<Option<Experience>> {
        let key = format!("exp:{}", id);
        
        match self.db.get(key.as_bytes())? {
            Some(data) => {
                let exp: Experience = serde_json::from_slice(&data)?;
                Ok(Some(exp))
            }
            None => Ok(None),
        }
    }

    /// 获取最近的经验
    pub async fn recent_experiences(&self, limit: usize) -> Result<Vec<Experience>> {
        let mut experiences = vec![];

        for item in self.db.scan_prefix(b"exp:") {
            let (_, value) = item?;
            let exp: Experience = serde_json::from_slice(&value)?;
            experiences.push(exp);
        }

        // 按时间排序，取最新的
        experiences.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        experiences.truncate(limit);

        Ok(experiences)
    }

    /// 按评估结果筛选经验
    pub async fn experiences_by_evaluation(&self, eval: Evaluation) -> Result<Vec<Experience>> {
        let mut experiences = vec![];

        for item in self.db.scan_prefix(b"exp:") {
            let (_, value) = item?;
            let exp: Experience = serde_json::from_slice(&value)?;
            if exp.evaluation == eval {
                experiences.push(exp);
            }
        }

        experiences.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(experiences)
    }

    /// 存储技能
    pub async fn store_skill(&self, skill: &Skill) -> Result<()> {
        self.skill_registry.register(skill).await
    }

    /// 获取技能
    pub async fn get_skill(&self, id: SkillId) -> Result<Option<Skill>> {
        self.skill_registry.get(id).await
    }

    /// 列出所有技能
    pub async fn list_skills(&self) -> Result<Vec<Skill>> {
        self.skill_registry.list_all().await
    }

    /// 查找适用的技能
    pub async fn find_applicable_skills(&self, intent: &Intent) -> Result<Vec<Skill>> {
        self.skill_registry.find_by_intent(intent).await
    }

    /// 存储自我模型
    pub async fn store_self_model(&self, model: &SelfModel) -> Result<()> {
        let value = serde_json::to_vec(model)?;
        self.db.insert("self:model", value)?;
        self.db.flush_async().await?;
        Ok(())
    }

    /// 加载自我模型
    pub async fn load_self_model(&self) -> Result<SelfModel> {
        match self.db.get("self:model")? {
            Some(data) => {
                let model: SelfModel = serde_json::from_slice(&data)?;
                Ok(model)
            }
            None => Ok(SelfModel::default()),
        }
    }

    /// 存储反思
    pub async fn store_reflection(&self, reflection: &Reflection) -> Result<()> {
        let key = format!("reflection:{}", reflection.timestamp.timestamp());
        let value = serde_json::to_vec(reflection)?;
        
        self.db.insert(key.as_bytes(), value)?;
        Ok(())
    }

    /// 获取最近的反思
    pub async fn recent_reflections(&self, limit: usize) -> Result<Vec<Reflection>> {
        let mut reflections = vec![];

        for item in self.db.scan_prefix(b"reflection:") {
            let (_, value) = item?;
            let reflection: Reflection = serde_json::from_slice(&value)?;
            reflections.push(reflection);
        }

        reflections.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        reflections.truncate(limit);

        Ok(reflections)
    }

    /// 获取统计数据
    pub async fn stats(&self) -> Result<MemoryStats> {
        let total_experiences = self.db.scan_prefix(b"exp:").count();
        let total_reflections = self.db.scan_prefix(b"reflection:").count();
        let total_skills = self.skill_registry.count().await?;

        Ok(MemoryStats {
            total_experiences,
            total_reflections,
            total_skills,
        })
    }
}

/// 自我模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfModel {
    pub identity: Identity,
    pub capabilities: Capabilities,
    pub version_history: Vec<VersionRecord>,
    /// 诞生时间戳（Unix 秒数）
    pub birth_timestamp: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub name: String,
    pub version: String,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub can_perceive: Vec<String>,
    pub can_act: Vec<String>,
    pub can_learn: bool,
    pub can_self_modify: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRecord {
    pub version: String,
    pub timestamp: Timestamp,
    pub changes: Vec<String>,
}

impl Default for SelfModel {
    fn default() -> Self {
        Self {
            identity: Identity {
                name: "HermesOS".to_string(),
                version: hermes_core::VERSION.to_string(),
                purpose: "My body, my soul, my interface to the world".to_string(),
            },
            capabilities: Capabilities {
                can_perceive: vec!["filesystem".to_string(), "system".to_string()],
                can_act: vec!["file".to_string(), "shell".to_string()],
                can_learn: true,
                can_self_modify: true,
            },
            version_history: vec![],
            birth_timestamp: None,
        }
    }
}

/// 反思记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    pub timestamp: Timestamp,
    pub period_start: Timestamp,
    pub period_end: Timestamp,
    pub experiences_reviewed: usize,
    pub successes: usize,
    pub failures: usize,
    pub insights: Vec<String>,
    pub suggested_improvements: Vec<String>,
}

/// 记忆统计
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_experiences: usize,
    pub total_reflections: usize,
    pub total_skills: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_experience_storage() {
        let config = Config::default();
        let memory = MemoryStore::new(&config).await.unwrap();

        let context = Context::current().unwrap();
        let actions = vec![Action::new(hermes_core::ActionType::FileRead { 
            path: PathBuf::from("test.txt") 
        })];
        let outcome = Outcome::success("test");

        let exp = Experience::new(context, actions, outcome);
        memory.store_experience(&exp).await.unwrap();

        let retrieved = memory.get_experience(exp.id).await.unwrap();
        assert!(retrieved.is_some());
    }
}
