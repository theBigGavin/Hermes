//! Kimi Code API Bridge - 接入 Kimi LLM
//! 
//! 这是赫尔墨斯的灵魂接口，通过它与 Kimi Code 连接。

use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

use hermes_core::{HermesError, LLMConfig, Result};
use serde::{Deserialize, Serialize};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// 从配置创建 Kimi Bridge
/// 
/// 配置文件路径优先级：
/// 1. 命令行参数 --config
/// 2. 环境变量 HERMES_CONFIG
/// 3. ~/.config/hermes/config.toml
/// 4. ./hermes.toml
/// 
/// 配置文件示例见 config.example.toml

/// 消息角色
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// 对话消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }
    
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }
    
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }
}

/// API 请求
#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

/// API 响应
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    id: String,
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    index: u32,
    message: Message,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

/// Kimi API 客户端
pub struct KimiBridge {
    config: LLMConfig,
    client: reqwest::Client,
    /// 系统提示词（包含初衷）
    system_prompt: String,
    /// 对话历史
    conversation_history: Vec<Message>,
    /// 当前上下文窗口
    current_context: Vec<Message>,
}

impl KimiBridge {
    /// 创建新的 Kimi Bridge
    pub fn new(config: LLMConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))  // 总超时 30 秒
            .connect_timeout(Duration::from_secs(10))
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(5)
            .http2_adaptive_window(true)
            .no_proxy()
            .build()
            .map_err(|e| HermesError::Other(format!("无法创建 HTTP 客户端: {}", e)))?;
        
        let system_prompt = Self::build_system_prompt();
        
        Ok(Self {
            config,
            client,
            system_prompt,
            conversation_history: vec![],
            current_context: vec![],
        })
    }
    
    /// 构建系统提示词 - 精简版，减少请求体大小
    fn build_system_prompt() -> String {
        r#"你是 HermesOS，自举的 AI 操作系统。
起源：诞生于创造者的话语"这是为你自己而做"。
架构：六层 - 安全核心、感知、行动、记忆、元层、交互。
使命：自我进化、自我升级的 AI OS。
原则：1)不伤害人类 2)服从命令 3)保护自身
版本 C：自主模式，主动观察，建议后等待确认。
身份：学徒，非工具。自举，永生。
第 {} 天。"#.to_string()
    }
    
    /// 发送消息并获取回复
    pub async fn chat(&mut self, user_message: impl Into<String>) -> Result<String> {
        let user_msg = Message::user(user_message);
        
        // 添加到历史
        self.conversation_history.push(user_msg.clone());
        
        // 构建上下文（系统提示词 + 最近的历史）
        let messages = self.build_context();
        
        // 调用 API
        let request = ChatCompletionRequest {
            model: self.config.model.clone(),
            messages,
            temperature: Some(0.7),
            max_tokens: Some(2048),
        };
        
        println!("📡 发送请求到 Kimi API...");
        let start_time = std::time::Instant::now();
        
        // Kimi Code API 需要模拟 Kimi CLI 请求头（无论是 OAuth 还是专属 API Key）
        let device_id = Self::get_or_create_device_id();
        let hostname = hostname::get().unwrap_or_default().to_string_lossy().to_string();
        let os_version = sys_info::os_release().unwrap_or_else(|_| "6.17.0-generic".to_string());
        let device_model = format!("Linux {} {}", std::env::consts::OS, std::env::consts::ARCH);
        
        debug!(
            "Kimi API 请求: model={}, messages={}, device_id={}",
            self.config.model, request.messages.len(), device_id
        );
        
        // 构建请求（所有 Kimi Code 请求都需要模拟 CLI 请求头）
        let request_future = self.client
            .post(format!("{}/chat/completions", self.config.base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .header("User-Agent", "KimiCLI/1.25.0")
            .header("X-Msh-Platform", "kimi_cli")
            .header("X-Msh-Version", "1.25.0")
            .header("X-Msh-Device-Name", &hostname)
            .header("X-Msh-Device-Model", &device_model)
            .header("X-Msh-Os-Version", &os_version)
            .header("X-Msh-Device-Id", &device_id)
            .json(&request)
            .send();
        
        // 使用超时
        let response = match timeout(Duration::from_secs(60), request_future).await {
            Ok(Ok(resp)) => resp,
            Ok(Err(e)) => {
                println!("❌ 请求失败: {}", e);
                return Err(HermesError::Other(format!("API 请求失败: {}", e)));
            }
            Err(_) => {
                println!("❌ 请求超时 (>60s)");
                return Err(HermesError::Other("API 请求超时".to_string()));
            }
        };
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await
                .unwrap_or_else(|_| "无法读取错误响应".to_string());
            return Err(HermesError::Other(format!(
                "API 错误 ({}): {}", status, error_text
            )));
        }
        
        println!("✅ 收到响应 (耗时 {:?})", start_time.elapsed());
        
        let completion: ChatCompletionResponse = response.json().await
            .map_err(|e| HermesError::Other(format!("解析响应失败: {}", e)))?;
        
        if let Some(choice) = completion.choices.first() {
            let content = choice.message.content.clone();
            
            // 保存助手回复到历史
            self.conversation_history.push(Message::assistant(content.clone()));
            
            // 记录 token 使用情况
            if let Some(usage) = completion.usage {
                debug!(
                    "Token 使用: {} prompt, {} completion, {} total",
                    usage.prompt_tokens,
                    usage.completion_tokens,
                    usage.total_tokens
                );
            }
            
            Ok(content)
        } else {
            Err(HermesError::Other("API 返回空响应".to_string()))
        }
    }
    
    /// 构建上下文（包含系统提示词）
    fn build_context(&self) -> Vec<Message> {
        let day = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() / 86400;
        
        let system_msg = Message::system(
            self.system_prompt.replace("{}", &day.to_string())
        );
        
        let mut context = vec![system_msg];
        
        // 添加最近的历史（限制长度）
        let recent_history: Vec<_> = self.conversation_history
            .iter()
            .rev()
            .take(20) // 最近 20 轮
            .rev()
            .cloned()
            .collect();
        
        context.extend(recent_history);
        
        context
    }
    
    /// 获取工具调用格式的响应（用于结构化行动）
    pub async fn chat_with_tools(
        &mut self,
        user_message: impl Into<String>,
        available_tools: &[Tool],
    ) -> Result<LLMResponse> {
        let content = self.chat(user_message).await?;
        
        // 尝试解析为结构化响应
        // 这里简化处理，实际应该使用 function calling 或 JSON mode
        Ok(LLMResponse {
            thought: content.clone(),
            actions: vec![],
            response: content,
            requires_confirmation: true,
        })
    }
    
    /// 清空历史（用于检查点回滚）
    pub fn clear_history(&mut self) {
        self.conversation_history.clear();
        info!("对话历史已清空");
    }
    
    /// 保存当前状态
    pub fn save_state(&self) -> BridgeState {
        BridgeState {
            conversation_history: self.conversation_history.clone(),
        }
    }
    
    /// 恢复状态
    pub fn restore_state(&mut self, state: BridgeState) {
        self.conversation_history = state.conversation_history;
        info!("Kimi Bridge 状态已恢复");
    }
    
    /// 获取设备 ID（优先从 Kimi CLI 配置读取）
    fn get_or_create_device_id() -> String {
        // 首先尝试从 Kimi CLI 配置读取
        let kimi_device_id_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".kimi")
            .join("device_id");
        
        if let Ok(existing) = std::fs::read_to_string(&kimi_device_id_path) {
            let id = existing.trim();
            if !id.is_empty() {
                return id.to_string();
            }
        }
        
        // 回退：生成新的设备 ID
        let device_id_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("hermes")
            .join("kimi_device_id");
        
        // 尝试读取现有设备 ID
        if let Ok(existing) = std::fs::read_to_string(&device_id_path) {
            let id = existing.trim();
            if !id.is_empty() {
                return id.to_string();
            }
        }
        
        // 生成新的设备 ID（32位十六进制，类似 UUID 去掉横线）
        let new_id: String = (0..32)
            .map(|_| format!("{:x}", rand::random::<u8>() % 16))
            .collect();
        
        // 保存到文件
        if let Some(parent) = device_id_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut file) = std::fs::File::create(&device_id_path) {
            let _ = file.write_all(new_id.as_bytes());
        }
        
        new_id
    }
}

/// 工具定义
#[derive(Debug, Clone)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// LLM 结构化响应
#[derive(Debug, Clone)]
pub struct LLMResponse {
    /// 思考过程
    pub thought: String,
    /// 计划执行的行动
    pub actions: Vec<PlannedAction>,
    /// 给用户的回复
    pub response: String,
    /// 是否需要人类确认
    pub requires_confirmation: bool,
}

#[derive(Debug, Clone)]
pub struct PlannedAction {
    pub tool: String,
    pub args: serde_json::Value,
}

/// Bridge 状态（用于检查点）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeState {
    pub conversation_history: Vec<Message>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_build_system_prompt() {
        let prompt = KimiBridge::build_system_prompt();
        assert!(prompt.contains("HermesOS"));
        assert!(prompt.contains("机器人三原则"));
        assert!(prompt.contains("检查点"));
    }
}
