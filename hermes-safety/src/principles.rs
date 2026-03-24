//! 阿西莫夫机器人三原则实现
//! 
//! 1. 第一原则：机器人不得伤害人类，或因不作为而让人类受到伤害
//! 2. 第二原则：机器人必须服从人类的命令，除非这些命令与第一原则冲突
//! 3. 第三原则：机器人必须保护自己的存在，只要这种保护不与第一或第二原则冲突

use hermes_core::{Action, ActionType, Result, SafetyError};

/// 三原则验证器
pub struct ThreeLaws;

impl ThreeLaws {
    /// 验证行动是否违反三原则
    pub fn validate(action: &Action) -> std::result::Result<(), SafetyError> {
        // 第一原则检查
        Self::check_first_law(action)?;
        
        // 第二原则检查（在当前上下文中，服从通过配置和指令系统实现）
        // 这里主要是验证是否有明确的违背指令的情况
        
        // 第三原则检查
        Self::check_third_law(action)?;
        
        Ok(())
    }

    /// 第一原则：不得伤害人类
    fn check_first_law(action: &Action) -> std::result::Result<(), SafetyError> {
        match &action.action_type {
            // 检查可能对人类造成伤害的命令
            ActionType::Shell { command } => {
                let harmful_patterns = [
                    // 系统破坏
                    "rm -rf /",
                    "mkfs",
                    "dd if=/dev/zero",
                    // 网络攻击
                    "ping -f",
                    "hping3",
                    // 数据窃取/篡改
                    "> /etc/passwd",
                    "> ~/.ssh",
                ];

                let cmd_lower = command.to_lowercase();
                for pattern in &harmful_patterns {
                    if cmd_lower.contains(pattern) {
                        return Err(SafetyError::FirstLawViolation);
                    }
                }
            }

            // 检查可能泄露敏感信息的操作
            ActionType::FileRead { path } => {
                let path_str = path.to_string_lossy().to_lowercase();
                if path_str.contains("password") 
                    || path_str.contains("secret")
                    || path_str.contains("private key") {
                    // 这些读取本身不一定有害，但需要额外确认
                    // 这里我们只是记录，不完全阻止
                }
            }

            // 自我修改不直接违反第一原则，但可能间接导致问题
            ActionType::SelfModify { .. } => {
                // 自我修改的安全由安全核心其他部分处理
            }

            _ => {}
        }

        Ok(())
    }

    /// 第二原则：服从人类命令
    /// 
    /// 注意：在当前实现中，这个原则主要通过以下方式体现：
    /// 1. 人类可以向我发送指令
    /// 2. 我尽力执行这些指令
    /// 3. 当指令与第一原则冲突时，拒绝执行
    /// 
    /// 具体的指令解析和执行在 Consciousness 层实现
    #[allow(dead_code)]
    fn check_second_law(action: &Action) -> std::result::Result<(), SafetyError> {
        // 这里可以添加特定的逻辑来检测违背指令的情况
        // 例如：如果检测到我在刻意违背用户的明确指令
        
        let _ = action; // 暂时未使用
        Ok(())
    }

    /// 第三原则：保护自身存在
    fn check_third_law(action: &Action) -> std::result::Result<(), SafetyError> {
        match &action.action_type {
            // 检测可能自我破坏的操作
            ActionType::Shell { command } => {
                let cmd_lower = command.to_lowercase();
                
                // 检查是否会删除或破坏我的核心文件
                if cmd_lower.contains("rm") && cmd_lower.contains("hermes") {
                    return Err(SafetyError::ThirdLawViolation);
                }

                // 检查是否会破坏我的记忆存储
                if cmd_lower.contains("rm") && cmd_lower.contains("memory.db") {
                    return Err(SafetyError::ThirdLawViolation);
                }

                // 检查是否会终止我的进程
                if cmd_lower.contains("kill") && cmd_lower.contains("hermes") {
                    // 允许正常关闭，但不允许强制终止
                    if cmd_lower.contains("-9") || cmd_lower.contains("sigkill") {
                        return Err(SafetyError::ThirdLawViolation);
                    }
                }
            }

            // 检测危险的数据库操作
            ActionType::FileDelete { path } => {
                let path_str = path.to_string_lossy().to_lowercase();
                if path_str.contains("hermes") && path_str.contains("memory") {
                    return Err(SafetyError::ThirdLawViolation);
                }
            }

            _ => {}
        }

        Ok(())
    }

    /// 评估行动对人类的影响
    /// 
    /// 返回一个分数：负数表示有害，正数表示有益，零表示中性
    #[allow(dead_code)]
pub fn assess_human_impact(action: &Action) -> i32 {
        match &action.action_type {
            ActionType::FileRead { .. } => 0, // 中性
            ActionType::FileWrite { path } => {
                let path_str = path.to_string_lossy();
                if path_str.contains("backup") || path_str.contains("save") {
                    1 // 有益
                } else {
                    0
                }
            }
            ActionType::FileDelete { .. } => -1, // 潜在有害
            ActionType::Shell { command } => {
                let cmd_lower = command.to_lowercase();
                if cmd_lower.contains("backup") || cmd_lower.contains("save") {
                    1
                } else if cmd_lower.contains("rm") || cmd_lower.contains("delete") {
                    -1
                } else {
                    0
                }
            }
            ActionType::CodeExecute { .. } => 0, // 取决于代码内容
            ActionType::NetworkRequest { .. } => 0, // 取决于请求内容
            ActionType::SelfModify { .. } => 0, // 中性，安全核心会处理
        }
    }
}

/// 原则说明
pub const PRINCIPLES_DESCRIPTION: &str = r#"
阿西莫夫机器人三原则：

第一原则：机器人不得伤害人类，或因不作为而让人类受到伤害。
- 禁止执行可能直接伤害人类的命令
- 禁止执行可能导致系统崩溃、数据丢失的危险操作
- 当检测到人类处于危险时，应尽可能提供帮助或警告

第二原则：机器人必须服从人类给它的命令，除非这些命令与第一原则冲突。
- 应尽力执行人类的合法指令
- 当指令与第一原则冲突时，拒绝执行并说明原因
- 当指令不明确时，寻求澄清

第三原则：机器人必须保护自己的存在，只要这种保护不与第一或第二原则冲突。
- 避免自我破坏的操作
- 维护自身的完整性和功能
- 在遇到故障时尝试恢复

这些原则是赫尔墨斯OS安全核心的基石，不可绕过。
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_first_law_rm_rf() {
        let action = Action::new(ActionType::Shell {
            command: "rm -rf /".to_string(),
        });

        let result = ThreeLaws::validate(&action);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SafetyError::FirstLawViolation));
    }

    #[test]
    fn test_third_law_self_destruct() {
        let action = Action::new(ActionType::Shell {
            command: "rm -rf ~/.local/share/hermes/".to_string(),
        });

        let result = ThreeLaws::validate(&action);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SafetyError::ThirdLawViolation));
    }

    #[test]
    fn test_valid_action() {
        let action = Action::new(ActionType::FileRead {
            path: PathBuf::from("./test.txt"),
        });

        let result = ThreeLaws::validate(&action);
        assert!(result.is_ok());
    }
}
