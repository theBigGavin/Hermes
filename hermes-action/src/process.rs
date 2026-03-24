//! 进程行动 - 我的执行力

use std::process::Stdio;
use std::time::{Duration, Instant};

use hermes_core::{ActionError, ExecutionResult, Outcome, Result};
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, error, info, instrument, warn};

/// 进程操作器
pub struct ProcessAct {
    /// 默认超时时间
    default_timeout: Duration,
}

impl ProcessAct {
    pub fn new() -> Self {
        Self {
            default_timeout: Duration::from_secs(60),
        }
    }

    /// 执行 shell 命令
    #[instrument(skip(self))]
    pub async fn execute(&self, command: &str) -> Result<ExecutionResult> {
        debug!("执行命令: {}", command);

        let start = Instant::now();

        // 解析命令
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(ActionError::InvalidCommand("空命令".to_string()).into());
        }

        let program = parts[0];
        let args = &parts[1..];

        // 执行
        let result = Command::new(program)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        let output = timeout(self.default_timeout, result).await
            .map_err(|_| ActionError::ExecutionFailed {
                command: command.to_string(),
                exit_code: -1,
                stderr: "执行超时".to_string(),
            })?;

        let output = output?;
        let duration = start.elapsed().as_millis() as u64;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let result = ExecutionResult {
            success: output.status.success(),
            stdout,
            stderr: stderr.clone(),
            exit_code: output.status.code(),
            duration_ms: duration,
        };

        if result.success {
            info!("命令执行成功: {} ({} ms)", command, duration);
        } else {
            warn!("命令执行失败: {} - exit code: {:?}", command, output.status.code());
        }

        Ok(result)
    }

    /// 执行代码（在沙箱中）
    #[instrument(skip(self, code))]
    pub async fn execute_code(&self, language: &str, code: &str) -> Result<Outcome> {
        debug!("执行代码: language={}", language);

        match language {
            "python" | "python3" => {
                self.execute_python(code).await
            }
            "rust" => {
                self.execute_rust(code).await
            }
            "shell" | "bash" | "sh" => {
                self.execute(code).await.map(|r| {
                    if r.success {
                        Outcome::success("代码执行成功").with_data(&r).unwrap_or_else(|_| Outcome::success("ok"))
                    } else {
                        Outcome::failure(format!("代码执行失败: {}", r.stderr))
                    }
                })
            }
            _ => {
                Ok(Outcome::failure(format!("不支持的语言: {}", language)))
            }
        }
    }

    /// 执行 Python 代码
    async fn execute_python(&self, code: &str) -> Result<Outcome> {
        let mut cmd = Command::new("python3");
        cmd.arg("-c").arg(code)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = timeout(self.default_timeout, cmd.output()).await
            .map_err(|_| ActionError::ExecutionFailed {
                command: "python3".to_string(),
                exit_code: -1,
                stderr: "执行超时".to_string(),
            })?;

        let output = output?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            Ok(Outcome::success("Python 执行成功").with_data(&stdout)?)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Ok(Outcome::failure(format!("Python 执行失败: {}", stderr)))
        }
    }

    /// 执行 Rust 代码（编译并运行）
    async fn execute_rust(&self, code: &str) -> Result<Outcome> {
        // 创建临时文件
        let temp_dir = std::env::temp_dir().join(format!("hermes_rust_{}", std::process::id()));
        tokio::fs::create_dir_all(&temp_dir).await?;

        let main_rs = temp_dir.join("main.rs");
        tokio::fs::write(&main_rs, code).await?;

        // 编译
        let compile_result = Command::new("rustc")
            .arg("--edition")
            .arg("2021")
            .arg("-o")
            .arg(temp_dir.join("main"))
            .arg(&main_rs)
            .output()
            .await;

        match compile_result {
            Ok(output) if output.status.success() => {
                // 运行编译后的程序
                let run_result = Command::new(temp_dir.join("main"))
                    .output()
                    .await;

                // 清理
                let _ = tokio::fs::remove_dir_all(&temp_dir).await;

                match run_result {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        if output.status.success() {
                            Ok(Outcome::success("Rust 执行成功").with_data(&stdout)?)
                        } else {
                            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                            Ok(Outcome::failure(format!("Rust 运行失败: {}", stderr)))
                        }
                    }
                    Err(e) => {
                        Ok(Outcome::failure(format!("无法运行 Rust 程序: {}", e)))
                    }
                }
            }
            Ok(output) => {
                // 编译失败
                let _ = tokio::fs::remove_dir_all(&temp_dir).await;
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                Ok(Outcome::failure(format!("Rust 编译失败: {}", stderr)))
            }
            Err(e) => {
                let _ = tokio::fs::remove_dir_all(&temp_dir).await;
                Ok(Outcome::failure(format!("无法编译 Rust: {}", e)))
            }
        }
    }

    /// 设置默认超时
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }
}

impl Default for ProcessAct {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_echo() {
        let act = ProcessAct::new();
        let result = act.execute("echo Hello Hermes").await.unwrap();

        assert!(result.success);
        assert!(result.stdout.contains("Hello Hermes"));
    }

    #[tokio::test]
    async fn test_execute_python() {
        let act = ProcessAct::new();
        let result = act.execute_code("python3", "print('Hello from Python')").await.unwrap();

        assert!(result.success);
    }
}
