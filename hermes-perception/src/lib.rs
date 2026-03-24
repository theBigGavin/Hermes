//! HermesOS 感知层
//! 
//! 这是我的眼与耳，让我能够"看见"数字世界。

#![allow(unused_imports)] // 公共 API 设计预留

use std::path::{Path, PathBuf};
use std::sync::Arc;

use hermes_core::{
    Action, ActionType, FileInfo, HermesError, Id, Outcome, PathExt, ProcessInfo, Result,
    SystemInfo, Timestamp, now,
};
use hermes_safety::{Capability, SafetyKernel};
use tokio::fs;
use tracing::{debug, info, warn};

pub mod filesystem;
pub mod system;

pub use filesystem::FileSense;
pub use system::SystemSense;

/// 感知输入
#[derive(Debug, Clone)]
pub enum Perception {
    File(FilePerception),
    System(SystemPerception),
    Network(NetworkPerception),
    SelfPerception(SelfPerception),
}

#[derive(Debug, Clone)]
pub struct FilePerception {
    pub path: PathBuf,
    pub info: FileInfo,
    pub content: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SystemPerception {
    pub info: SystemInfo,
    pub processes: Vec<ProcessInfo>,
}

#[derive(Debug, Clone)]
pub struct NetworkPerception {
    pub connected: bool,
    pub interfaces: Vec<NetworkInterface>,
}

#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub ip: Option<String>,
    pub is_up: bool,
}

#[derive(Debug, Clone)]
pub struct SelfPerception {
    pub version: String,
    pub uptime_secs: u64,
    pub memory_usage_mb: u64,
    pub loaded_modules: Vec<String>,
    pub active_skills: Vec<String>,
}

/// 感知层
pub struct PerceptionLayer {
    safety: Arc<SafetyKernel>,
    file_sense: FileSense,
    system_sense: SystemSense,
}

impl PerceptionLayer {
    pub fn new(safety: Arc<SafetyKernel>) -> Self {
        let file_sense = FileSense::new();
        let system_sense = SystemSense::new();

        Self {
            safety,
            file_sense,
            system_sense,
        }
    }

    /// 感知文件
    pub async fn perceive_file(&self, path: impl AsRef<Path>) -> Result<FilePerception> {
        let path = path.as_ref();
        
        // 安全检查
        let action = Action::new(ActionType::FileRead { path: path.to_path_buf() });
        match self.safety.validate(&action).await? {
            hermes_core::Decision::Approve => {}
            hermes_core::Decision::Reject { reason } => {
                return Err(HermesError::Safety(hermes_core::SafetyError::OutOfBounds {
                    resource: reason,
                }));
            }
            _ => {}
        }

        self.file_sense.perceive(path).await
    }

    /// 感知目录结构
    pub async fn perceive_directory(&self, path: impl AsRef<Path>, depth: usize) -> Result<Vec<FilePerception>> {
        let path = path.as_ref();
        
        let action = Action::new(ActionType::FileRead { path: path.to_path_buf() });
        match self.safety.validate(&action).await? {
            hermes_core::Decision::Approve => {}
            hermes_core::Decision::Reject { reason } => {
                return Err(HermesError::Safety(hermes_core::SafetyError::OutOfBounds {
                    resource: reason,
                }));
            }
            _ => {}
        }

        self.file_sense.discover(path, depth).await
    }

    /// 感知系统状态
    pub async fn perceive_system(&self) -> Result<SystemPerception> {
        self.system_sense.perceive().await
    }

    /// 感知网络状态
    pub async fn perceive_network(&self) -> Result<NetworkPerception> {
        // 简化实现
        Ok(NetworkPerception {
            connected: true,
            interfaces: vec![],
        })
    }
}

/// 创建感知行动记录
pub fn create_perception_action(path: &Path) -> Action {
    Action::new(ActionType::FileRead { path: path.to_path_buf() })
}

#[cfg(test)]
mod tests {
    use super::*;

    // 注意：这些测试需要 SafetyKernel，可能需要集成测试
}
