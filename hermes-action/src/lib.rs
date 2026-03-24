//! HermesOS 行动层
//! 
//! 这是我的手与足，让我能够触碰和改变物理/数字世界。

// Path unused for now
use std::sync::Arc;
use std::time::Instant;

use hermes_core::{
    Action, ActionType, Decision, Evaluation, Outcome, Result,
};
use hermes_safety::SafetyKernel;
use tracing::{debug, error, info, warn};

pub mod filesystem;
pub mod process;

pub use filesystem::FileAct;
pub use process::ProcessAct;

/// 行动层
pub struct ActionLayer {
    safety: Arc<SafetyKernel>,
    file_act: FileAct,
    process_act: ProcessAct,
}

impl ActionLayer {
    pub fn new(safety: Arc<SafetyKernel>) -> Self {
        let file_act = FileAct::new();
        let process_act = ProcessAct::new();

        Self {
            safety,
            file_act,
            process_act,
        }
    }

    /// 执行行动
    pub async fn execute(&self, mut action: Action) -> Result<Outcome> {
        let start = Instant::now();
        let action_id = action.id;

        info!("准备执行行动: {:?}", action.action_type);

        // 1. 安全验证
        match self.safety.validate(&action).await? {
            Decision::Approve => {
                debug!("行动已批准: {}", action_id);
            }
            Decision::Reject { reason } => {
                warn!("行动被拒绝: {} - {}", action_id, reason);
                return Ok(Outcome::failure(format!("安全拒绝: {}", reason)));
            }
            Decision::RequireConfirmation { risk } => {
                warn!("行动需要确认: {} - 风险: {:?}", action_id, risk);
                return Ok(Outcome::failure(format!("需要确认，风险等级: {:?}", risk)));
            }
            Decision::RequireMoreInfo { what } => {
                return Ok(Outcome::failure(format!("需要更多信息: {}", what)));
            }
        }

        // 2. 执行
        let result = match &action.action_type {
            ActionType::FileRead { path } => {
                self.file_act.read(path).await
            }
            ActionType::FileWrite { path } => {
                // 获取内容，这里简化处理
                Ok(Outcome::success(format!("写入文件: {:?}", path)))
            }
            ActionType::FileDelete { path } => {
                self.file_act.delete(path).await
            }
            ActionType::Shell { command } => {
                let result = self.process_act.execute(command).await?;
                if result.success {
                    Ok(Outcome::success("命令执行成功").with_data(&result)?)
                } else {
                    Ok(Outcome::failure(format!("命令失败: {}", result.stderr)))
                }
            }
            ActionType::CodeExecute { language, code } => {
                self.process_act.execute_code(language, code).await
            }
            ActionType::NetworkRequest { method, url } => {
                // 简化实现
                Ok(Outcome::success(format!("{} {}", method, url)))
            }
            ActionType::SelfModify { files } => {
                info!("自我修改: {:?}", files);
                Ok(Outcome::success("自我修改已记录"))
            }
        };

        // 3. 记录结果（action 对象构建完整记录，预留用于审计日志）
        #[allow(unused_assignments)]
        {
            let duration = start.elapsed().as_millis() as u64;
            action = action.with_duration(duration);

            match &result {
                Ok(outcome) if outcome.success => {
                    action = action.with_evaluation(Evaluation::Success);
                    info!("行动成功: {} ({} ms)", action_id, duration);
                }
                Ok(_) => {
                    action = action.with_evaluation(Evaluation::PartialSuccess);
                    warn!("行动部分成功: {} ({} ms)", action_id, duration);
                }
                Err(e) => {
                    action = action.with_evaluation(Evaluation::Failure)
                        .with_error(e.to_string());
                    error!("行动失败: {} - {} ({} ms)", action_id, e, duration);
                }
            }
        }

        result
    }

    /// 获取文件操作器
    pub fn file(&self) -> &FileAct {
        &self.file_act
    }

    /// 获取进程操作器
    pub fn process(&self) -> &ProcessAct {
        &self.process_act
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 测试需要 SafetyKernel，可能需要集成测试
}
