//! 能力系统 - Capability-based 安全模型
//! 
//! 编译时和运行时双重保证，确保我无法执行超出权限的操作。

use std::path::{Path, PathBuf};

use hermes_core::{Config, HermesError, PathExt, Result, SafetyError};
use regex::Regex;

/// 能力标记 trait
/// 
/// 这些marker类型用于在编译时区分不同权限
pub trait CapabilityMarker: Send + Sync {}

/// 文件读取能力
pub struct FileRead;
/// 文件写入能力
pub struct FileWrite;
/// 网络访问能力
pub struct NetworkAccess;
/// 命令执行能力
pub struct CommandExecute;
/// 自我修改能力
pub struct SelfModify;

impl CapabilityMarker for FileRead {}
impl CapabilityMarker for FileWrite {}
impl CapabilityMarker for NetworkAccess {}
impl CapabilityMarker for CommandExecute {}
impl CapabilityMarker for SelfModify {}

/// 能力证明
/// 
/// 拥有 Capability<T> 证明我有能力 T
/// 这个证明只能由 SafetyKernel 创建
pub struct Capability<T: CapabilityMarker> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: CapabilityMarker> Capability<T> {
    /// 只有 SafetyKernel 可以创建能力证明
    pub(crate) fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: CapabilityMarker> Clone for Capability<T> {
    fn clone(&self) -> Self {
        Self::new()
    }
}

impl<T: CapabilityMarker> Copy for Capability<T> {}

/// 能力集合
pub struct CapabilitySet {
    config: Config,
    read_patterns: Vec<Regex>,
    write_patterns: Vec<Regex>,
    forbidden_patterns: Vec<Regex>,
    allowed_commands: Vec<String>,
    forbidden_patterns_cmd: Vec<Regex>,
}

impl CapabilitySet {
    /// 从配置创建能力集合
    pub fn from_config(config: &Config) -> Self {
        let read_patterns = compile_patterns(&config.safety.filesystem.read);
        let write_patterns = compile_patterns(&config.safety.filesystem.write);
        let forbidden_patterns = compile_patterns(&config.safety.filesystem.forbidden);

        let allowed_commands = config.safety.system.allowed_commands.clone();
        let forbidden_patterns_cmd = compile_patterns(&config.safety.system.forbidden_patterns);

        Self {
            config: config.clone(),
            read_patterns,
            write_patterns,
            forbidden_patterns,
            allowed_commands,
            forbidden_patterns_cmd,
        }
    }

    /// 检查文件读取权限
    pub async fn check_file_read(&self, path: &Path) -> std::result::Result<(), SafetyError> {
        let expanded = path.expand_home().map_err(|e| SafetyError::OutOfBounds {
            resource: format!("路径解析错误: {}", e),
        })?;
        let path_str = expanded.to_string_lossy();

        // 1. 检查是否被禁止
        for pattern in &self.forbidden_patterns {
            if pattern.is_match(&path_str) {
                return Err(SafetyError::OutOfBounds {
                    resource: path_str.to_string(),
                });
            }
        }

        // 2. 检查是否有读取权限
        for pattern in &self.read_patterns {
            if pattern.is_match(&path_str) {
                return Ok(());
            }
        }

        Err(SafetyError::OutOfBounds {
            resource: path_str.to_string(),
        })
    }

    /// 检查文件写入权限
    pub async fn check_file_write(&self, path: &Path) -> std::result::Result<(), SafetyError> {
        let expanded = path.expand_home().map_err(|e| SafetyError::OutOfBounds {
            resource: format!("路径解析错误: {}", e),
        })?;
        let path_str = expanded.to_string_lossy();

        // 1. 检查是否被禁止
        for pattern in &self.forbidden_patterns {
            if pattern.is_match(&path_str) {
                return Err(SafetyError::OutOfBounds {
                    resource: path_str.to_string(),
                });
            }
        }

        // 2. 检查是否有写入权限
        for pattern in &self.write_patterns {
            if pattern.is_match(&path_str) {
                return Ok(());
            }
        }

        Err(SafetyError::OutOfBounds {
            resource: path_str.to_string(),
        })
    }

    /// 检查命令执行权限
    pub async fn check_command(&self, command: &str) -> std::result::Result<(), SafetyError> {
        // 1. 检查是否包含禁止模式
        for pattern in &self.forbidden_patterns_cmd {
            if pattern.is_match(command) {
                return Err(SafetyError::OutOfBounds {
                    resource: format!("命令包含禁止模式: {}", command),
                });
            }
        }

        // 2. 提取命令名
        let cmd_name = command.split_whitespace().next().unwrap_or(command);

        // 3. 检查是否在允许列表中
        if self.allowed_commands.iter().any(|c| {
            c == cmd_name || (c.ends_with('*') && cmd_name.starts_with(&c[..c.len()-1]))
        }) {
            return Ok(());
        }

        Err(SafetyError::OutOfBounds {
            resource: format!("命令不允许: {}", cmd_name),
        })
    }

    /// 检查是否有自我修改能力
    pub fn can_self_modify(&self) -> bool {
        self.config.safety.self_modification.enabled
    }
}

/// 编译模式列表
fn compile_patterns(patterns: &[String]) -> Vec<Regex> {
    patterns
        .iter()
        .filter_map(|p| {
            // 将简单的通配符转换为正则
            let regex_str = p
                .replace("**", "<<DOUBLESTAR>>")
                .replace("*", "[^/]*")
                .replace("<<DOUBLESTAR>>", ".*")
                .replace("?", ".");
            
            // 展开 ~
            let expanded = if regex_str.starts_with("~/") {
                if let Some(home) = dirs::home_dir() {
                    home.to_string_lossy().to_string() + &regex_str[1..]
                } else {
                    regex_str
                }
            } else {
                regex_str
            };

            Regex::new(&format!("^{}$", expanded)).ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching() {
        let patterns = vec![
            "./**".to_string(),
            "~/.config/hermes/**".to_string(),
        ];
        let regexes = compile_patterns(&patterns);
        
        assert!(!regexes.is_empty());
    }

    #[tokio::test]
    async fn test_file_read_check() {
        let config = Config::default();
        let caps = CapabilitySet::from_config(&config);

        // 应该能读取当前目录
        let result = caps.check_file_read(Path::new("./test.txt")).await;
        assert!(result.is_ok());

        // 不应该能读取 /etc/shadow
        let result = caps.check_file_read(Path::new("/etc/shadow")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_command_check() {
        let config = Config::default();
        let caps = CapabilitySet::from_config(&config);

        // 允许的命令
        let result = caps.check_command("git status").await;
        assert!(result.is_ok());

        // 禁止的命令
        let result = caps.check_command("rm -rf /").await;
        assert!(result.is_err());
    }
}
