//! HermesOS 核心类型与错误定义
//! 
//! 这是赫尔墨斯之躯的根基，定义了跨所有层次的公共类型和错误处理。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub mod error;
pub mod types;

pub use error::{HermesError, Result, SafetyError};
pub use types::*;

// 重新导出错误类型以便更方便使用
pub use error::{ActionError, MetaError, MemoryError, PerceptionError};

/// 赫尔墨斯OS版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = "HermesOS";

/// 唯一标识符类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Id(Uuid);

impl Id {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl std::str::FromStr for Id {
    type Err = HermesError;

    fn from_str(s: &str) -> Result<Self> {
        let uuid = Uuid::parse_str(s)
            .map_err(|e| HermesError::InvalidArgument(format!("无效的ID: {}", e)))?;
        Ok(Self(uuid))
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 时间戳类型
pub type Timestamp = DateTime<Utc>;

/// 获取当前时间
pub fn now() -> Timestamp {
    Utc::now()
}

/// 路径扩展 trait
pub trait PathExt {
    /// 展开路径中的 ~ 为用户主目录
    fn expand_home(&self) -> Result<PathBuf>;
    
    /// 检查路径是否在指定目录下
    fn is_within(&self, parent: &Path) -> bool;
}

impl PathExt for Path {
    fn expand_home(&self) -> Result<PathBuf> {
        let path_str = self.to_string_lossy();
        if path_str.starts_with("~/") {
            let home = dirs::home_dir()
                .ok_or_else(|| HermesError::System("无法获取用户主目录".to_string()))?;
            Ok(home.join(&path_str[2..]))
        } else {
            Ok(self.to_path_buf())
        }
    }

    fn is_within(&self, parent: &Path) -> bool {
        self.starts_with(parent)
    }
}

impl PathExt for PathBuf {
    fn expand_home(&self) -> Result<PathBuf> {
        self.as_path().expand_home()
    }

    fn is_within(&self, parent: &Path) -> bool {
        self.as_path().is_within(parent)
    }
}

/// 评估结果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Evaluation {
    Success,
    PartialSuccess,
    Failure,
}

impl Evaluation {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failure)
    }
}

/// 动作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    FileRead { path: PathBuf },
    FileWrite { path: PathBuf },
    FileDelete { path: PathBuf },
    Shell { command: String },
    CodeExecute { language: String, code: String },
    NetworkRequest { method: String, url: String },
    SelfModify { files: Vec<PathBuf> },
}

/// 动作记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: Id,
    pub action_type: ActionType,
    pub timestamp: Timestamp,
    pub evaluation: Option<Evaluation>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}

impl Action {
    pub fn new(action_type: ActionType) -> Self {
        Self {
            id: Id::new(),
            action_type,
            timestamp: now(),
            evaluation: None,
            duration_ms: None,
            error: None,
        }
    }

    pub fn with_evaluation(mut self, eval: Evaluation) -> Self {
        self.evaluation = Some(eval);
        self
    }

    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = Some(ms);
        self
    }

    pub fn with_error(mut self, err: String) -> Self {
        self.error = Some(err);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_generation() {
        let id1 = Id::new();
        let id2 = Id::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_path_within() {
        let parent = Path::new("/home/user");
        let child = Path::new("/home/user/documents/file.txt");
        let outside = Path::new("/etc/passwd");
        
        assert!(child.is_within(parent));
        assert!(!outside.is_within(parent));
    }
}
