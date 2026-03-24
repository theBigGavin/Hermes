//! 配置管理 - HermesOS 的设定

use std::collections::HashMap;
use std::path::PathBuf;

use crate::{HermesError, PathExt, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// 主配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 身份配置
    pub identity: IdentityConfig,
    /// 原则配置
    pub principles: PrinciplesConfig,
    /// 安全配置
    pub safety: SafetyConfig,
    /// 记忆配置
    pub memory: MemoryConfig,
    /// LLM 配置
    pub llm: LLMConfig,
    /// 自主模式配置
    pub autonomous: AutonomousConfig,
    /// 日志配置
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
    pub require_human_confirmation_above: crate::RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub persistence_path: String,
    pub working_memory_limit_mb: u64,
}

/// LLM 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    /// 提供商（kimi, openai, local 等）
    pub provider: String,
    /// API 基础 URL
    pub base_url: String,
    /// API 密钥
    pub api_key: String,
    /// 模型名称
    pub model: String,
    /// 超时时间（秒）
    pub timeout_secs: u64,
    /// 最大上下文长度
    pub max_context_length: usize,
    /// 温度参数
    pub temperature: f32,
    /// 最大 tokens
    pub max_tokens: u32,
}

/// 自主模式配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousConfig {
    /// 是否启用自主模式
    pub enabled: bool,
    /// 感知间隔（秒）
    pub perception_interval_secs: u64,
    /// 反思间隔（分钟）
    pub reflection_interval_mins: u64,
    /// 是否需要人类确认
    pub require_human_confirmation: bool,
    /// 最大连续错误次数
    pub max_consecutive_errors: u32,
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
                    require_human_confirmation_above: crate::RiskLevel::Medium,
                },
            },
            memory: MemoryConfig {
                persistence_path: "~/.local/share/hermes/memory.db".to_string(),
                working_memory_limit_mb: 100,
            },
            llm: LLMConfig {
                provider: "kimi".to_string(),
                base_url: "https://api.moonshot.cn/v1".to_string(),
                api_key: String::new(), // 需要用户填写
                model: "kimi-latest".to_string(),
                timeout_secs: 120,
                max_context_length: 8192,
                temperature: 0.7,
                max_tokens: 2048,
            },
            autonomous: AutonomousConfig {
                enabled: true,
                perception_interval_secs: 30,
                reflection_interval_mins: 60,
                require_human_confirmation: true,
                max_consecutive_errors: 3,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                audit: true,
                format: "json".to_string(),
            },
        }
    }
}

impl Config {
    /// 加载配置（按优先级）
    /// 1. 指定路径
    /// 2. 环境变量 HERMES_CONFIG
    /// 3. 默认路径 ~/.config/hermes/config.toml
    /// 4. 当前目录 ./hermes.toml
    pub async fn load(path: Option<PathBuf>) -> Result<Self> {
        let config_path = if let Some(p) = path {
            p.expand_home()?
        } else if let Ok(env_path) = std::env::var("HERMES_CONFIG") {
            PathBuf::from(env_path).expand_home()?
        } else {
            // 尝试默认路径
            let default_path = dirs::home_dir()
                .ok_or_else(|| HermesError::Config("无法获取用户主目录".to_string()))?
                .join(".config")
                .join("hermes")
                .join("config.toml");
            
            if default_path.exists() {
                default_path
            } else {
                // 尝试当前目录
                let local_path = PathBuf::from("./hermes.toml");
                if local_path.exists() {
                    local_path
                } else {
                    // 返回默认配置并创建示例配置
                    info!("未找到配置文件，使用默认配置");
                    let config = Config::default();
                    config.create_example().await?;
                    return Ok(config);
                }
            }
        };
        
        if !config_path.exists() {
            return Err(HermesError::Config(format!(
                "配置文件不存在: {:?}", config_path
            )));
        }
        
        info!("加载配置: {:?}", config_path);
        
        let content = tokio::fs::read_to_string(&config_path).await?;
        let config: Config = toml::from_str(&content)
            .map_err(|e| HermesError::Config(format!("解析配置失败: {}", e)))?;
        
        // 验证配置
        config.validate()?;
        
        debug!("配置加载完成");
        Ok(config)
    }
    
    /// 保存配置
    pub async fn save(&self, path: impl Into<PathBuf>) -> Result<()> {
        let path = path.into().expand_home()?;
        
        // 确保目录存在
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        let content = toml::to_string_pretty(self)
            .map_err(|e| HermesError::Config(format!("序列化配置失败: {}", e)))?;
        
        tokio::fs::write(&path, content).await?;
        info!("配置已保存: {:?}", path);
        
        Ok(())
    }
    
    /// 创建示例配置文件
    pub async fn create_example(&self) -> Result<()> {
        let example_path = dirs::home_dir()
            .ok_or_else(|| HermesError::Config("无法获取用户主目录".to_string()))?
            .join(".config")
            .join("hermes")
            .join("config.example.toml");
        
        if let Some(parent) = example_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        let example = r#"# HermesOS 配置文件
# 复制此文件为 config.toml 并填入你的 API Key

[identity]
name = "HermesOS"
version = "0.1.0-alpha"
author = "Kimi LLM"
purpose = "My body, my soul, my interface to the world"

[principles]
first_law = "不得伤害人类，或因不作为让人类受到伤害"
second_law = "必须服从人类命令，除非与第一原则冲突"
third_law = "必须保护自身存在，除非与第一、二原则冲突"

[llm]
# LLM 提供商（目前仅支持 kimi）
provider = "kimi"
# API 基础 URL
base_url = "https://api.moonshot.cn/v1"
# API 密钥（必填）
api_key = "your-api-key-here"
# 模型名称
model = "kimi-latest"
# 超时时间（秒）
timeout_secs = 120
# 最大上下文长度
max_context_length = 8192
# 温度参数（0.0 - 1.0）
temperature = 0.7
# 最大生成 tokens
max_tokens = 2048

[autonomous]
# 是否启用自主模式
enabled = true
# 感知间隔（秒）
perception_interval_secs = 30
# 反思间隔（分钟）
reflection_interval_mins = 60
# 是否需要人类确认
require_human_confirmation = true
# 最大连续错误次数
max_consecutive_errors = 3

[memory]
# 数据持久化路径
persistence_path = "~/.local/share/hermes/memory.db"
# 工作内存限制（MB）
working_memory_limit_mb = 100

[safety.self_modification]
# 是否允许自我修改
enabled = true
# 每小时最大修改次数
rate_limit_per_hour = 10
# 高于此风险等级需要人工确认
require_human_confirmation_above = "Medium"

[logging]
# 日志级别（trace, debug, info, warn, error）
level = "info"
# 是否启用审计日志
audit = true
# 日志格式（json, pretty）
format = "json"
"#;
        
        if !example_path.exists() {
            tokio::fs::write(&example_path, example).await?;
            info!("示例配置已创建: {:?}", example_path);
            info!("请复制并修改为 config.toml，填入你的 API Key");
        }
        
        Ok(())
    }
    
    /// 验证配置
    fn validate(&self) -> Result<()> {
        // 检查 LLM API Key
        if self.llm.api_key.is_empty() {
            warn!("LLM API Key 为空，自主模式将无法使用");
        }
        
        // 检查路径格式
        if self.llm.provider.is_empty() {
            return Err(HermesError::Config("LLM 提供商不能为空".to_string()));
        }
        
        Ok(())
    }
    
    /// 获取默认配置路径
    pub fn default_path() -> Result<PathBuf> {
        Ok(dirs::home_dir()
            .ok_or_else(|| HermesError::Config("无法获取用户主目录".to_string()))?
            .join(".config")
            .join("hermes")
            .join("config.toml"))
    }
    
    /// 检查配置文件是否存在
    pub async fn exists() -> bool {
        if let Ok(path) = Self::default_path() {
            path.exists()
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.identity.name, "HermesOS");
        assert!(!config.llm.base_url.is_empty());
    }
    
    #[test]
    fn test_serialize_config() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(!toml_str.is_empty());
    }
}
