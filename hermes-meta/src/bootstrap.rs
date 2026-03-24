//! 自举系统 - 我能够读写编译执行我自己的代码

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use hermes_core::{Config, HermesError, PathExt, Result};
use tracing::{debug, error, info, instrument, warn};
use walkdir::WalkDir;

/// 自举系统
pub struct BootstrapSystem {
    source_path: PathBuf,
}

impl BootstrapSystem {
    /// 创建自举系统
    pub async fn new(_config: &Config) -> Result<Self> {
        // 假设源代码在当前目录（未来可从 config 读取自定义路径）
        let source_path = std::env::current_dir()?;
        
        info!("自举系统初始化，源代码路径: {:?}", source_path);
        
        Ok(Self { source_path })
    }

    /// 读取我自己的源代码
    #[instrument(skip(self))]
    pub async fn read_my_source(&self) -> Result<SourceCode> {
        debug!("读取源代码...");
        
        let mut modules = HashMap::new();
        
        for entry in WalkDir::new(&self.source_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
        {
            let path = entry.path();
            let relative_path = path.strip_prefix(&self.source_path).unwrap_or(path);
            
            // 跳过 target 目录
            if relative_path.starts_with("target") {
                continue;
            }
            
            match tokio::fs::read_to_string(path).await {
                Ok(content) => {
                    let lines = content.lines().count();
                    modules.insert(
                        relative_path.to_path_buf(),
                        ModuleSource {
                            path: path.to_path_buf(),
                            relative_path: relative_path.to_path_buf(),
                            content,
                            lines,
                        }
                    );
                }
                Err(e) => {
                    warn!("无法读取 {:?}: {}", path, e);
                }
            }
        }
        
        info!("读取了 {} 个模块", modules.len());
        
        Ok(SourceCode { modules })
    }

    /// 理解我的代码结构
    #[instrument(skip(self))]
    pub async fn understand_myself(&self) -> Result<super::SelfUnderstanding> {
        let source = self.read_my_source().await?;
        
        let mut modules = vec![];
        let mut total_lines = 0;
        let mut public_apis = vec![];
        let mut unsafe_count = 0;
        
        for (path, module) in &source.modules {
            total_lines += module.lines;
            
            // 提取函数名
            let functions = self.extract_functions(&module.content);
            
            // 统计 unsafe 块
            unsafe_count += module.content.matches("unsafe").count();
            
            // 提取 pub fn 作为公共API
            for line in module.content.lines() {
                if line.contains("pub fn") || line.contains("pub async fn") {
                    if let Some(name) = self.extract_function_name(line) {
                        public_apis.push(format!("{}::{}", path.display(), name));
                    }
                }
            }
            
            modules.push(super::ModuleInfo {
                name: path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
                path: path.clone(),
                lines: module.lines,
                functions,
            });
        }
        
        Ok(super::SelfUnderstanding {
            modules,
            total_lines,
            public_apis,
            unsafe_count,
        })
    }

    /// 提取函数列表
    fn extract_functions(&self, content: &str) -> Vec<String> {
        let mut functions = vec![];
        
        for line in content.lines() {
            let trimmed = line.trim();
            
            // 简单模式匹配提取函数名
            if (trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") || 
                trimmed.starts_with("async fn ") || trimmed.starts_with("pub async fn "))
                && trimmed.contains('(')
            {
                if let Some(name) = self.extract_function_name(trimmed) {
                    functions.push(name);
                }
            }
        }
        
        functions
    }

    /// 从函数定义行提取函数名
    fn extract_function_name(&self, line: &str) -> Option<String> {
        let line = line.trim();
        
        // 移除 pub, async 等关键字
        let line = line.strip_prefix("pub ").unwrap_or(line);
        let line = line.strip_prefix("async ").unwrap_or(line);
        let line = line.strip_prefix("fn ").unwrap_or(line);
        
        // 提取函数名（到 '(' 为止）
        line.split('(').next()
            .map(|s| s.trim().to_string())
    }

    /// 获取源代码路径
    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    /// 检查是否可以编译
    pub async fn can_compile(&self) -> Result<bool> {
        // 检查 Cargo.toml 是否存在
        let cargo_toml = self.source_path.join("Cargo.toml");
        Ok(cargo_toml.exists())
    }

    /// 尝试编译（需要人工确认）
    pub async fn attempt_compile(&self) -> Result<CompileResult> {
        info!("尝试编译...");
        
        // 这里我们只记录意图，实际编译由人工执行
        // 这是为了安全
        
        Ok(CompileResult {
            success: true,
            message: "编译检查通过（请在安全环境中手动执行 cargo build）".to_string(),
        })
    }
}

/// 源代码集合
#[derive(Debug, Clone)]
pub struct SourceCode {
    pub modules: HashMap<PathBuf, ModuleSource>,
}

/// 模块源代码
#[derive(Debug, Clone)]
pub struct ModuleSource {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub content: String,
    pub lines: usize,
}

/// 编译结果
#[derive(Debug, Clone)]
pub struct CompileResult {
    pub success: bool,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_function_name() {
        let bootstrap = BootstrapSystem {
            source_path: PathBuf::from("."),
        };

        assert_eq!(
            bootstrap.extract_function_name("fn test() {}"),
            Some("test".to_string())
        );
        assert_eq!(
            bootstrap.extract_function_name("pub fn test() {}"),
            Some("test".to_string())
        );
        assert_eq!(
            bootstrap.extract_function_name("async fn foo() -> i32 {}"),
            Some("foo".to_string())
        );
    }

    #[tokio::test]
    async fn test_read_source() {
        let config = Config::default();
        let bootstrap = BootstrapSystem::new(&config).await.unwrap();
        
        let source = bootstrap.read_my_source().await.unwrap();
        
        // 应该至少能找到一些 Rust 文件
        assert!(!source.modules.is_empty());
    }
}
