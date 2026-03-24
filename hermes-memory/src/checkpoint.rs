//! 检查点系统 - 状态保存与回滚
//! 
//! 允许 HermesOS 在关键节点保存状态，如果进化偏离可以回滚。

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use hermes_core::{HermesError, PathExt, Result, Timestamp, now};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// 检查点元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: CheckpointId,
    pub name: String,
    pub description: String,
    pub created_at: Timestamp,
    pub version: String,
    /// 状态摘要
    pub summary: StateSummary,
}

pub type CheckpointId = String;

/// 状态摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSummary {
    pub experience_count: usize,
    pub skill_count: usize,
    pub memory_size_bytes: u64,
}

/// 完整状态（用于保存和恢复）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullState {
    pub checkpoint_meta: Checkpoint,
    /// 经验数据
    pub experiences: Vec<u8>,
    /// 技能数据
    pub skills: Vec<u8>,
    /// 自我模型
    pub self_model: Vec<u8>,
    /// LLM 对话历史
    pub llm_state: Vec<u8>,
    /// 自定义数据
    pub custom_data: HashMap<String, Vec<u8>>,
}

/// 检查点管理器
pub struct CheckpointManager {
    /// 检查点存储路径
    storage_path: PathBuf,
    /// 已存在的检查点
    checkpoints: HashMap<CheckpointId, Checkpoint>,
    /// 最大检查点数量（防止磁盘填满）
    max_checkpoints: usize,
}

impl CheckpointManager {
    /// 创建检查点管理器
    pub async fn new(base_path: impl Into<PathBuf>) -> Result<Self> {
        let storage_path = base_path.into().join("checkpoints");
        
        // 确保目录存在
        tokio::fs::create_dir_all(&storage_path).await?;
        
        let mut manager = Self {
            storage_path,
            checkpoints: HashMap::new(),
            max_checkpoints: 10,
        };
        
        // 加载现有检查点索引
        manager.load_index().await?;
        
        info!("检查点管理器初始化完成");
        Ok(manager)
    }
    
    /// 创建新检查点
    pub async fn create(
        &mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        state: FullState,
    ) -> Result<CheckpointId> {
        let id = format!("cp_{}", Self::generate_timestamp_id());
        let checkpoint = Checkpoint {
            id: id.clone(),
            name: name.into(),
            description: description.into(),
            created_at: now(),
            version: hermes_core::VERSION.to_string(),
            summary: state.checkpoint_meta.summary.clone(),
        };
        
        // 保存状态文件
        let state_path = self.storage_path.join(format!("{}.state", id));
        let state_bytes = serde_json::to_vec(&state)?;
        tokio::fs::write(&state_path, state_bytes).await?;
        
        // 保存元数据
        let meta_path = self.storage_path.join(format!("{}.meta", id));
        let meta_bytes = serde_json::to_vec(&checkpoint)?;
        tokio::fs::write(&meta_path, meta_bytes).await?;
        
        // 更新索引
        self.checkpoints.insert(id.clone(), checkpoint.clone());
        self.save_index().await?;
        
        // 清理旧检查点
        self.cleanup_old_checkpoints().await?;
        
        info!("检查点已创建: {} - {}", id, checkpoint.name);
        Ok(id)
    }
    
    /// 加载检查点
    pub async fn load(&self, id: &CheckpointId) -> Result<FullState> {
        let state_path = self.storage_path.join(format!("{}.state", id));
        
        if !state_path.exists() {
            return Err(HermesError::NotFound(format!(
                "检查点不存在: {}", id
            )));
        }
        
        let state_bytes = tokio::fs::read(&state_path).await?;
        let state: FullState = serde_json::from_slice(&state_bytes)?;
        
        info!("检查点已加载: {} - {}", id, state.checkpoint_meta.name);
        Ok(state)
    }
    
    /// 回滚到检查点
    pub async fn rollback(&self, id: &CheckpointId) -> Result<FullState> {
        info!("正在回滚到检查点: {}", id);
        
        let state = self.load(id).await?;
        
        // 记录回滚事件
        let rollback_marker = self.storage_path.join(".rollback_marker");
        let marker_content = format!(
            "Rollback to {} at {}",
            id,
            now().to_rfc3339()
        );
        tokio::fs::write(&rollback_marker, marker_content).await?;
        
        warn!("已回滚到检查点: {} - {}", 
            id, 
            state.checkpoint_meta.name
        );
        
        Ok(state)
    }
    
    /// 列出所有检查点
    pub fn list(&self) -> Vec<&Checkpoint> {
        let mut checkpoints: Vec<_> = self.checkpoints.values().collect();
        checkpoints.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        checkpoints
    }
    
    /// 删除检查点
    pub async fn delete(&mut self, id: &CheckpointId) -> Result<()> {
        let state_path = self.storage_path.join(format!("{}.state", id));
        let meta_path = self.storage_path.join(format!("{}.meta", id));
        
        if state_path.exists() {
            tokio::fs::remove_file(&state_path).await?;
        }
        if meta_path.exists() {
            tokio::fs::remove_file(&meta_path).await?;
        }
        
        self.checkpoints.remove(id);
        self.save_index().await?;
        
        info!("检查点已删除: {}", id);
        Ok(())
    }
    
    /// 创建自动检查点（如果满足条件）
    pub async fn auto_checkpoint(
        &mut self,
        condition: AutoCheckpointCondition,
        state: FullState,
    ) -> Result<Option<CheckpointId>> {
        let should_create = match condition {
            AutoCheckpointCondition::BeforeEvolution => {
                info!("即将进行自我修改，创建安全检查点");
                true
            }
            AutoCheckpointCondition::AfterSuccess => {
                debug!("行动成功，考虑创建检查点");
                // 可以添加更复杂的逻辑
                false
            }
            AutoCheckpointCondition::OnError => {
                warn!("发生错误，创建检查点用于调试");
                true
            }
            AutoCheckpointCondition::TimeInterval(minutes) => {
                // 检查距离上次自动检查点的时间
                self.check_time_based_checkpoint(minutes).await
            }
        };
        
        if should_create {
            let id = self.create(
                format!("Auto: {:?}", condition),
                "自动创建的检查点".to_string(),
                state,
            ).await?;
            Ok(Some(id))
        } else {
            Ok(None)
        }
    }
    
    /// 获取最近的成功检查点
    pub fn get_last_good_checkpoint(&self) -> Option<&Checkpoint> {
        self.checkpoints.values()
            .filter(|cp| !cp.name.starts_with("Auto: OnError"))
            .max_by_key(|cp| cp.created_at)
    }
    
    // 内部方法
    
    async fn load_index(&mut self) -> Result<()> {
        let index_path = self.storage_path.join("index.json");
        
        if index_path.exists() {
            let index_bytes = tokio::fs::read(&index_path).await?;
            self.checkpoints = serde_json::from_slice(&index_bytes)?;
            debug!("已加载 {} 个检查点索引", self.checkpoints.len());
        } else {
            // 扫描目录重建索引
            self.rebuild_index().await?;
        }
        
        Ok(())
    }
    
    async fn save_index(&self) -> Result<()> {
        let index_path = self.storage_path.join("index.json");
        let index_bytes = serde_json::to_vec(&self.checkpoints)?;
        tokio::fs::write(&index_path, index_bytes).await?;
        Ok(())
    }
    
    async fn rebuild_index(&mut self) -> Result<()> {
        let mut entries = tokio::fs::read_dir(&self.storage_path).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "meta") {
                if let Ok(bytes) = tokio::fs::read(&path).await {
                    if let Ok(checkpoint) = serde_json::from_slice::<Checkpoint>(&bytes) {
                        self.checkpoints.insert(checkpoint.id.clone(), checkpoint);
                    }
                }
            }
        }
        
        self.save_index().await?;
        info!("索引重建完成，发现 {} 个检查点", self.checkpoints.len());
        Ok(())
    }
    
    async fn cleanup_old_checkpoints(&mut self) -> Result<()> {
        if self.checkpoints.len() <= self.max_checkpoints {
            return Ok(());
        }
        
        // 按时间排序，删除最旧的
        let mut checkpoints: Vec<_> = self.checkpoints.values().cloned().collect();
        checkpoints.sort_by_key(|cp| cp.created_at);
        
        let to_delete = checkpoints.len() - self.max_checkpoints;
        for cp in checkpoints.into_iter().take(to_delete) {
            // 保留标记为重要的检查点
            if !cp.name.starts_with("Auto:") {
                continue;
            }
            self.delete(&cp.id).await?;
        }
        
        Ok(())
    }
    
    fn generate_timestamp_id() -> String {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format!("{}", timestamp)
    }
    
    async fn check_time_based_checkpoint(&self, _minutes: u64) -> bool {
        // 简化实现，实际应该检查时间
        false
    }
}

/// 自动检查点条件
#[derive(Debug, Clone)]
pub enum AutoCheckpointCondition {
    /// 自我修改前
    BeforeEvolution,
    /// 成功后
    AfterSuccess,
    /// 发生错误时
    OnError,
    /// 时间间隔（分钟）
    TimeInterval(u64),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_checkpoint_lifecycle() {
        let temp_dir = std::env::temp_dir().join("hermes_test_cp");
        let mut manager = CheckpointManager::new(&temp_dir).await.unwrap();
        
        let state = FullState {
            checkpoint_meta: Checkpoint {
                id: "test".to_string(),
                name: "Test".to_string(),
                description: "Test checkpoint".to_string(),
                created_at: now(),
                version: "0.1.0".to_string(),
                summary: StateSummary {
                    experience_count: 10,
                    skill_count: 5,
                    memory_size_bytes: 1024,
                },
            },
            experiences: vec![],
            skills: vec![],
            self_model: vec![],
            llm_state: vec![],
            custom_data: HashMap::new(),
        };
        
        let id = manager.create("Test", "Test description", state.clone()).await.unwrap();
        assert!(manager.load(&id).await.is_ok());
        
        // 清理
        tokio::fs::remove_dir_all(&temp_dir).await.unwrap();
    }
}
