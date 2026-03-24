//! 文件行动 - 我的手

#![allow(unused_imports)] // 公共 API 设计预留

use std::path::{Path, PathBuf};

use hermes_core::{HermesError, Outcome, PathExt, Result};
use tokio::fs;
use tracing::{debug, error, info, warn};

/// 文件操作器
pub struct FileAct;

impl FileAct {
    pub fn new() -> Self {
        Self
    }

    /// 读取文件
    pub async fn read(&self, path: impl AsRef<Path>) -> Result<Outcome> {
        let path = path.as_ref().expand_home()?;
        
        debug!("读取文件: {:?}", path);

        match fs::read_to_string(&path).await {
            Ok(content) => {
                info!("文件读取成功: {:?}", path);
                Outcome::success("文件读取成功").with_data(&content)
            }
            Err(e) => {
                error!("文件读取失败: {:?} - {}", path, e);
                Ok(Outcome::failure(format!("无法读取文件: {}", e)))
            }
        }
    }

    /// 写入文件
    pub async fn write(&self, path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> Result<Outcome> {
        let path = path.as_ref().expand_home()?;
        
        debug!("写入文件: {:?}", path);

        // 确保目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        match fs::write(&path, content.as_ref()).await {
            Ok(()) => {
                info!("文件写入成功: {:?}", path);
                Ok(Outcome::success("文件写入成功"))
            }
            Err(e) => {
                error!("文件写入失败: {:?} - {}", path, e);
                Ok(Outcome::failure(format!("无法写入文件: {}", e)))
            }
        }
    }

    /// 追加到文件
    pub async fn append(&self, path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> Result<Outcome> {
        let path = path.as_ref().expand_home()?;
        
        debug!("追加文件: {:?}", path);

        use tokio::io::AsyncWriteExt;

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;

        match file.write_all(content.as_ref()).await {
            Ok(()) => {
                file.flush().await?;
                info!("文件追加成功: {:?}", path);
                Ok(Outcome::success("文件追加成功"))
            }
            Err(e) => {
                error!("文件追加失败: {:?} - {}", path, e);
                Ok(Outcome::failure(format!("无法追加文件: {}", e)))
            }
        }
    }

    /// 删除文件
    pub async fn delete(&self, path: impl AsRef<Path>) -> Result<Outcome> {
        let path = path.as_ref().expand_home()?;
        
        debug!("删除文件: {:?}", path);

        match fs::remove_file(&path).await {
            Ok(()) => {
                info!("文件删除成功: {:?}", path);
                Ok(Outcome::success("文件删除成功"))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                warn!("文件不存在: {:?}", path);
                Ok(Outcome::success("文件不存在，无需删除"))
            }
            Err(e) => {
                error!("文件删除失败: {:?} - {}", path, e);
                Ok(Outcome::failure(format!("无法删除文件: {}", e)))
            }
        }
    }

    /// 创建目录
    pub async fn mkdir(&self, path: impl AsRef<Path>) -> Result<Outcome> {
        let path = path.as_ref().expand_home()?;
        
        debug!("创建目录: {:?}", path);

        match fs::create_dir_all(&path).await {
            Ok(()) => {
                info!("目录创建成功: {:?}", path);
                Ok(Outcome::success("目录创建成功"))
            }
            Err(e) => {
                error!("目录创建失败: {:?} - {}", path, e);
                Ok(Outcome::failure(format!("无法创建目录: {}", e)))
            }
        }
    }

    /// 复制文件
    pub async fn copy(&self, from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<Outcome> {
        let from = from.as_ref().expand_home()?;
        let to = to.as_ref().expand_home()?;
        
        debug!("复制文件: {:?} -> {:?}", from, to);

        // 确保目标目录存在
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).await?;
        }

        match fs::copy(&from, &to).await {
            Ok(bytes) => {
                info!("文件复制成功: {:?} -> {:?} ({} bytes)", from, to, bytes);
                Ok(Outcome::success(format!("复制了 {} bytes", bytes)))
            }
            Err(e) => {
                error!("文件复制失败: {:?} -> {:?} - {}", from, to, e);
                Ok(Outcome::failure(format!("无法复制文件: {}", e)))
            }
        }
    }

    /// 移动/重命名文件
    pub async fn rename(&self, from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<Outcome> {
        let from = from.as_ref().expand_home()?;
        let to = to.as_ref().expand_home()?;
        
        debug!("重命名文件: {:?} -> {:?}", from, to);

        match fs::rename(&from, &to).await {
            Ok(()) => {
                info!("文件重命名成功: {:?} -> {:?}", from, to);
                Ok(Outcome::success("文件重命名成功"))
            }
            Err(e) => {
                error!("文件重命名失败: {:?} -> {:?} - {}", from, to, e);
                Ok(Outcome::failure(format!("无法重命名文件: {}", e)))
            }
        }
    }

    /// 检查文件是否存在
    pub async fn exists(&self, path: impl AsRef<Path>) -> bool {
        match path.as_ref().expand_home() {
            Ok(p) => p.exists(),
            Err(_) => false,
        }
    }
}

impl Default for FileAct {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_file_operations() {
        let act = FileAct::new();
        let test_file = "/tmp/hermes_test_action.txt";

        // 写入
        let result = act.write(test_file, "Hello, Hermes!").await.unwrap();
        assert!(result.success);

        // 读取
        let result = act.read(test_file).await.unwrap();
        assert!(result.success);

        // 删除
        let result = act.delete(test_file).await.unwrap();
        assert!(result.success);

        // 确认删除
        assert!(!act.exists(test_file).await);
    }
}
