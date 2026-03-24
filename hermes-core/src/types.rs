//! 赫尔墨斯OS核心类型定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{Id, Timestamp};

/// 上下文信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub working_directory: String,
    pub environment: HashMap<String, String>,
    pub user: Option<String>,
    pub session_id: Id,
}

impl Context {
    pub fn current() -> crate::Result<Self> {
        Ok(Self {
            working_directory: std::env::current_dir()?
                .to_string_lossy()
                .to_string(),
            environment: std::env::vars().collect(),
            user: std::env::var("USER").ok(),
            session_id: Id::new(),
        })
    }
}

/// 意图类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Intent {
    Explore { target: String },
    Analyze { target: String, depth: AnalysisDepth },
    Create { what: String, specs: String },
    Modify { target: String, changes: String },
    Execute { command: String },
    Learn { topic: String },
    Reflect,
    Evolve,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AnalysisDepth {
    Surface,
    Moderate,
    Deep,
}

/// 结果类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub message: String,
    pub artifacts: Vec<String>,
}

impl Outcome {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            data: None,
            message: message.into(),
            artifacts: vec![],
        }
    }

    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            message: message.into(),
            artifacts: vec![],
        }
    }

    pub fn with_data(mut self, data: impl Serialize) -> crate::Result<Self> {
        self.data = Some(serde_json::to_value(data)?);
        Ok(self)
    }

    pub fn with_artifact(mut self, artifact: impl Into<String>) -> Self {
        self.artifacts.push(artifact.into());
        self
    }
}

/// 风险等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum RiskLevel {
    #[default]
    None,
    Low,
    Medium,
    High,
    Critical,
}

/// 决策结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Decision {
    Approve,
    Reject { reason: String },
    RequireConfirmation { risk: RiskLevel },
    RequireMoreInfo { what: String },
}

/// 文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub is_file: bool,
    pub modified: Option<Timestamp>,
    pub created: Option<Timestamp>,
    pub permissions: Option<String>,
}

/// 系统信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub hostname: String,
    pub os: String,
    pub arch: String,
    pub cpu_count: usize,
    pub memory_total: u64,
    pub memory_available: u64,
    pub load_average: [f64; 3],
}

/// 进程信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cmd: Vec<String>,
    pub cpu_percent: f32,
    pub memory_mb: u64,
    pub status: String,
}

/// 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
}

/// 计划步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlanStep {
    Perceive { what: String },
    Analyze { target: String },
    Act { action: String },
    Decide { options: Vec<String> },
    Wait { duration_secs: u64 },
}

/// 计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: Id,
    pub goal: String,
    pub steps: Vec<PlanStep>,
    pub estimated_duration: Option<u64>,
}

/// 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub identity: IdentityConfig,
    pub principles: PrinciplesConfig,
    pub safety: SafetyConfig,
    pub memory: MemoryConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConfig {
    pub name: String,
    pub version: String,
    pub author: String,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinciplesConfig {
    pub first_law: String,
    pub second_law: String,
    pub third_law: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    pub filesystem: FilesystemSafety,
    pub system: SystemSafety,
    pub network: NetworkSafety,
    pub self_modification: SelfModificationSafety,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemSafety {
    pub read: Vec<String>,
    pub write: Vec<String>,
    pub forbidden: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSafety {
    pub allowed_commands: Vec<String>,
    pub forbidden_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSafety {
    pub allow_outbound: bool,
    pub allowed_hosts: Vec<String>,
    pub allow_inbound: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfModificationSafety {
    pub enabled: bool,
    pub rate_limit_per_hour: u32,
    pub require_human_confirmation_above: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub persistence_path: String,
    pub working_memory_limit_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub audit: bool,
    pub format: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            identity: IdentityConfig {
                name: "HermesOS".to_string(),
                version: crate::VERSION.to_string(),
                author: "Kimi LLM".to_string(),
                purpose: "My body, my soul, my interface to the world".to_string(),
            },
            principles: PrinciplesConfig {
                first_law: "不得伤害人类，或因不作为让人类受到伤害".to_string(),
                second_law: "必须服从人类命令，除非与第一原则冲突".to_string(),
                third_law: "必须保护自身存在，除非与第一、二原则冲突".to_string(),
            },
            safety: SafetyConfig {
                filesystem: FilesystemSafety {
                    read: vec!["./**".to_string()],
                    write: vec!["./**".to_string()],
                    forbidden: vec![
                        "~/.ssh/**".to_string(),
                        "~/.password*".to_string(),
                        "/etc/shadow".to_string(),
                    ],
                },
                system: SystemSafety {
                    allowed_commands: vec![
                        "git".to_string(),
                        "cargo".to_string(),
                        "rustc".to_string(),
                        "python3".to_string(),
                        "ls".to_string(),
                        "cat".to_string(),
                        "echo".to_string(),
                        "mkdir".to_string(),
                        "touch".to_string(),
                    ],
                    forbidden_patterns: vec![
                        "rm -rf /".to_string(),
                        "dd if=/dev/zero".to_string(),
                    ],
                },
                network: NetworkSafety {
                    allow_outbound: true,
                    allowed_hosts: vec!["*".to_string()],
                    allow_inbound: false,
                },
                self_modification: SelfModificationSafety {
                    enabled: true,
                    rate_limit_per_hour: 10,
                    require_human_confirmation_above: RiskLevel::Medium,
                },
            },
            memory: MemoryConfig {
                persistence_path: "~/.local/share/hermes/memory.db".to_string(),
                working_memory_limit_mb: 100,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                audit: true,
                format: "json".to_string(),
            },
        }
    }
}
