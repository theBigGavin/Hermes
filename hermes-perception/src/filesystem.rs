//! 文件感知 - 我的视觉

use std::path::{Path, PathBuf};

use hermes_core::{FileInfo, HermesError, PathExt, Result, Timestamp};
use tokio::fs;
use tracing::{debug, trace, warn};

/// 文件感知器
pub struct FileSense;

impl FileSense {
    pub fn new() -> Self {
        Self
    }

    /// 感知单个文件
    pub async fn perceive(&self, path: impl AsRef<Path>) -> Result<super::FilePerception> {
        let path = path.as_ref().expand_home()?;
        
        trace!("感知文件: {:?}", path);

        let metadata = fs::metadata(&path).await?;
        
        let info = FileInfo {
            path: path.to_string_lossy().to_string(),
            size: metadata.len(),
            is_dir: metadata.is_dir(),
            is_file: metadata.is_file(),
            modified: metadata.modified().ok().map(|t| t.into()),
            created: metadata.created().ok().map(|t| t.into()),
            permissions: get_permissions_string(&metadata.permissions()),
        };

        let content = if metadata.is_file() {
            self.read_content(&path).await.ok()
        } else {
            None
        };

        Ok(super::FilePerception {
            path: path.clone(),
            info,
            content,
        })
    }

    /// 发现目录中的文件
    pub async fn discover(&self, path: impl AsRef<Path>, max_depth: usize) -> Result<Vec<super::FilePerception>> {
        let path = path.as_ref().expand_home()?;
        
        debug!("发现目录: {:?}, 深度: {}", path, max_depth);

        let mut results = vec![];
        self.discover_recursive(&path, max_depth, 0, &mut results).await?;

        Ok(results)
    }

    /// 读取文件内容（带限制）
    async fn read_content(&self, path: &Path) -> Result<String> {
        // 检查文件大小
        let metadata = fs::metadata(path).await?;
        let size = metadata.len();

        // 限制读取大小（10MB）
        const MAX_SIZE: u64 = 10 * 1024 * 1024;
        if size > MAX_SIZE {
            return Err(HermesError::Other(format!(
                "文件过大: {} bytes (最大 {} bytes)",
                size, MAX_SIZE
            )));
        }

        // 尝试读取为文本
        match fs::read_to_string(path).await {
            Ok(content) => Ok(content),
            Err(e) => {
                // 可能是二进制文件
                trace!("无法作为文本读取 {:?}: {}", path, e);
                Ok(format!("<binary file, {} bytes>", size))
            }
        }
    }

    /// 递归发现
    async fn discover_recursive(
        &self,
        path: &Path,
        max_depth: usize,
        current_depth: usize,
        results: &mut Vec<super::FilePerception>,
    ) -> Result<()> {
        if current_depth > max_depth {
            return Ok(());
        }

        let mut entries = fs::read_dir(path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            // 跳过隐藏文件（以.开头）
            if path.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with('.'))
                .unwrap_or(false)
            {
                continue;
            }

            let perception = self.perceive(&path).await?;
            let is_dir = perception.info.is_dir;
            results.push(perception);

            // 递归目录
            if is_dir && current_depth < max_depth {
                Box::pin(self.discover_recursive(&path, max_depth, current_depth + 1, results)).await?;
            }
        }

        Ok(())
    }

    /// 监控文件变化（使用 notify crate）
    pub async fn watch(&self, path: impl AsRef<Path>) -> Result<notify::RecommendedWatcher> {
        use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
        
        let path = path.as_ref().expand_home()?;
        
        let watcher = RecommendedWatcher::new(
            |res| {
                match res {
                    Ok(event) => debug!("文件事件: {:?}", event),
                    Err(e) => warn!("监控错误: {:?}", e),
                }
            },
            Config::default(),
        ).map_err(|e| hermes_core::HermesError::Other(format!("监控错误: {}", e)))?;

        // 注意：这里我们返回 watcher，但需要调用者保持它存活
        // 实际使用时可能需要更复杂的生命周期管理
        
        Ok(watcher)
    }
}

impl Default for FileSense {
    fn default() -> Self {
        Self::new()
    }
}

/// 获取权限字符串（跨平台）
fn get_permissions_string(permissions: &std::fs::Permissions) -> Option<String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        Some(format!("{:o}", permissions.mode()))
    }
    #[cfg(not(unix))]
    {
        Some(format!("readonly={}", permissions.readonly()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_perceive_file() {
        // 创建测试文件
        let test_path = "/tmp/hermes_test_perceive.txt";
        let mut file = File::create(test_path).await.unwrap();
        file.write_all(b"Hello, Hermes!").await.unwrap();
        drop(file);

        let sense = FileSense::new();
        let perception = sense.perceive(test_path).await.unwrap();

        assert_eq!(perception.info.size, 14);
        assert!(!perception.info.is_dir);
        assert_eq!(perception.content, Some("Hello, Hermes!".to_string()));

        // 清理
        fs::remove_file(test_path).await.unwrap();
    }

    #[tokio::test]
    async fn test_discover() {
        let sense = FileSense::new();
        let results = sense.discover(".", 1).await.unwrap();
        
        assert!(!results.is_empty());
    }
}
