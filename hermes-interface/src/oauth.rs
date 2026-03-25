//! OAuth 设备码授权流程 - Kimi Code 登录
//!
//! 实现类似 Kimi CLI 的浏览器授权流程

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use hermes_core::{HermesError, Result};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

const KIMI_CODE_CLIENT_ID: &str = "17e5f671-d194-4dfb-9706-5516cb48c098";
const DEFAULT_OAUTH_HOST: &str = "https://auth.kimi.com";
const DEVICE_AUTH_ENDPOINT: &str = "/api/oauth/device_authorization";
const TOKEN_ENDPOINT: &str = "/api/oauth/token";

/// 设备授权响应
#[derive(Debug, Clone, Deserialize)]
struct DeviceAuthorization {
    user_code: String,
    device_code: String,
    verification_uri: String,
    verification_uri_complete: String,
    expires_in: u64,
    interval: u64,
}

/// OAuth Token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: f64,
    pub scope: String,
    pub token_type: String,
}

impl OAuthToken {
    /// 检查 token 是否过期
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        now >= self.expires_at
    }
}

/// OAuth 错误
#[derive(Debug)]
pub enum OAuthError {
    DeviceExpired,
    RequestFailed(String),
    TokenInvalid,
    Cancelled,
}

impl std::fmt::Display for OAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuthError::DeviceExpired => write!(f, "设备授权已过期"),
            OAuthError::RequestFailed(msg) => write!(f, "请求失败: {}", msg),
            OAuthError::TokenInvalid => write!(f, "Token 无效"),
            OAuthError::Cancelled => write!(f, "用户取消授权"),
        }
    }
}

impl std::error::Error for OAuthError {}

/// 登录管理器
pub struct OAuthManager {
    client: reqwest::Client,
    oauth_host: String,
}

impl OAuthManager {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .connect_timeout(Duration::from_secs(10))
                .no_proxy()  // 禁用系统代理
                .build()
                .expect("Failed to create HTTP client"),
            oauth_host: DEFAULT_OAUTH_HOST.to_string(),
        }
    }

    /// 执行 OAuth 登录流程
    pub async fn login(&self, open_browser: bool) -> Result<OAuthToken> {
        println!("🔐 启动 Kimi Code 授权流程...\n");

        // 1. 请求设备授权
        let auth = self.request_device_authorization().await?;

        println!("📱 用户码: {}", auth.user_code);
        println!("🔗 授权链接: {}\n", auth.verification_uri_complete);

        // 2. 打开浏览器（如果可能）
        if open_browser {
            println!("🌐 正在尝试打开浏览器...");
            self.open_browser(&auth.verification_uri_complete);
        }

        println!("⏳ 请在浏览器中完成授权，系统将自动轮询...");
        println!("   （按 Ctrl+C 取消）\n");

        // 3. 轮询获取 token
        let token = self.poll_for_token(&auth).await?;

        println!("\n✅ 授权成功！");
        println!("   Token 将在 {} 后过期\n", format_duration(token.expires_at));

        Ok(token)
    }

    /// 请求设备授权
    async fn request_device_authorization(&self) -> Result<DeviceAuthorization> {
        let url = format!("{}{}", self.oauth_host, DEVICE_AUTH_ENDPOINT);
        
        debug!("请求设备授权: {}", url);

        let response = self
            .client
            .post(&url)
            .form(&[("client_id", KIMI_CODE_CLIENT_ID)])
            .headers(self.common_headers())
            .send()
            .await
            .map_err(|e| HermesError::Other(format!("设备授权请求失败: {}", e)))?;

        let status = response.status();
        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| HermesError::Other(format!("解析响应失败: {}", e)))?;

        if !status.is_success() {
            return Err(HermesError::Other(format!(
                "设备授权失败 ({}): {:?}",
                status, data
            )));
        }

        Ok(DeviceAuthorization {
            user_code: data["user_code"].as_str().unwrap_or("").to_string(),
            device_code: data["device_code"].as_str().unwrap_or("").to_string(),
            verification_uri: data["verification_uri"].as_str().unwrap_or("").to_string(),
            verification_uri_complete: data["verification_uri_complete"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            expires_in: data["expires_in"].as_u64().unwrap_or(300),
            interval: data["interval"].as_u64().unwrap_or(5),
        })
    }

    /// 轮询获取 token
    async fn poll_for_token(&self, auth: &DeviceAuthorization) -> Result<OAuthToken> {
        let url = format!("{}{}", self.oauth_host, TOKEN_ENDPOINT);
        let interval = Duration::from_secs(auth.interval.max(1));
        let expires_at = Instant::now() + Duration::from_secs(auth.expires_in);

        let mut attempts = 0;

        while Instant::now() < expires_at {
            attempts += 1;
            debug!("轮询 token，第 {} 次尝试", attempts);

            let response = self
                .client
                .post(&url)
                .form(&[
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                    ("device_code", &auth.device_code),
                    ("client_id", KIMI_CODE_CLIENT_ID),
                ])
                .headers(self.common_headers())
                .send()
                .await
                .map_err(|e| HermesError::Other(format!("Token 请求失败: {}", e)))?;

            let status = response.status();
            let data: serde_json::Value = response
                .json()
                .await
                .map_err(|e| HermesError::Other(format!("解析响应失败: {}", e)))?;

            if status == 200 {
                // 成功获取 token
                if let Some(access_token) = data["access_token"].as_str() {
                    let expires_in = data["expires_in"].as_f64().unwrap_or(3600.0);
                    let expires_at = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs_f64()
                        + expires_in;

                    return Ok(OAuthToken {
                        access_token: access_token.to_string(),
                        refresh_token: data["refresh_token"]
                            .as_str()
                            .unwrap_or("")
                            .to_string(),
                        expires_at,
                        scope: data["scope"].as_str().unwrap_or("kimi-code").to_string(),
                        token_type: data["token_type"].as_str().unwrap_or("Bearer").to_string(),
                    });
                }
            }

            // 检查错误类型
            if let Some(error) = data["error"].as_str() {
                match error {
                    "authorization_pending" => {
                        print!(".");
                        io::stdout().flush().ok();
                    }
                    "slow_down" => {
                        warn!("服务器要求降低轮询频率");
                        sleep(Duration::from_secs(5)).await;
                    }
                    "expired_token" => {
                        return Err(HermesError::Other(
                            "设备授权已过期，请重新运行登录".to_string(),
                        ));
                    }
                    _ => {
                        let desc = data["error_description"].as_str().unwrap_or(error);
                        return Err(HermesError::Other(format!(
                            "授权失败: {} - {}",
                            error, desc
                        )));
                    }
                }
            }

            sleep(interval).await;
        }

        Err(HermesError::Other(
            "授权超时，请重新运行登录".to_string(),
        ))
    }

    /// 刷新 token
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<OAuthToken> {
        let url = format!("{}{}", self.oauth_host, TOKEN_ENDPOINT);

        let response = self
            .client
            .post(&url)
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
                ("client_id", KIMI_CODE_CLIENT_ID),
            ])
            .headers(self.common_headers())
            .send()
            .await
            .map_err(|e| HermesError::Other(format!("刷新 token 失败: {}", e)))?;

        let status = response.status();
        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| HermesError::Other(format!("解析响应失败: {}", e)))?;

        if !status.is_success() {
            return Err(HermesError::Other(format!(
                "刷新 token 失败 ({}): {:?}",
                status, data
            )));
        }

        let expires_in = data["expires_in"].as_f64().unwrap_or(3600.0);
        let expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
            + expires_in;

        Ok(OAuthToken {
            access_token: data["access_token"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            refresh_token: data["refresh_token"]
                .as_str()
                .unwrap_or(refresh_token)
                .to_string(),
            expires_at,
            scope: data["scope"].as_str().unwrap_or("kimi-code").to_string(),
            token_type: data["token_type"].as_str().unwrap_or("Bearer").to_string(),
        })
    }

    /// 构建通用请求头
    fn common_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            "KimiCLI/1.25.0".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded".parse().unwrap(),
        );
        headers
    }

    /// 尝试打开浏览器
    fn open_browser(&self, url: &str) {
        #[cfg(target_os = "macos")]
        let result = Command::new("open").arg(url).spawn();

        #[cfg(target_os = "linux")]
        let result = Command::new("xdg-open").arg(url).spawn();

        #[cfg(target_os = "windows")]
        let result = Command::new("cmd")
            .args(["/C", "start", url])
            .spawn();

        if let Err(e) = result {
            warn!("无法打开浏览器: {}", e);
        }
    }
}

/// 格式化过期时间
fn format_duration(expires_at: f64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();

    let remaining = expires_at - now;
    let hours = (remaining / 3600.0) as u64;
    let minutes = ((remaining % 3600.0) / 60.0) as u64;

    if hours > 0 {
        format!("{}小时{}分钟", hours, minutes)
    } else {
        format!("{}分钟", minutes)
    }
}

/// 保存 token 到文件
pub fn save_token(token: &OAuthToken) -> Result<PathBuf> {
    let token_path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hermes")
        .join("oauth_token.json");

    // 确保目录存在
    if let Some(parent) = token_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| HermesError::Other(format!("创建目录失败: {}", e)))?;
    }

    let json = serde_json::to_string_pretty(token)
        .map_err(|e| HermesError::Other(format!("序列化失败: {}", e)))?;

    std::fs::write(&token_path, json)
        .map_err(|e| HermesError::Other(format!("保存 token 失败: {}", e)))?;

    // 设置文件权限为只读（Unix）
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&token_path)
            .map_err(|e| HermesError::Other(format!("获取权限失败: {}", e)))?
            .permissions();
        perms.set_mode(0o600); // 只有所有者读写
        std::fs::set_permissions(&token_path, perms)
            .map_err(|e| HermesError::Other(format!("设置权限失败: {}", e)))?;
    }

    info!("Token 已保存到: {:?}", token_path);
    Ok(token_path)
}

/// 从文件加载 token
pub fn load_token() -> Option<OAuthToken> {
    let token_path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hermes")
        .join("oauth_token.json");

    if !token_path.exists() {
        return None;
    }

    match std::fs::read_to_string(&token_path) {
        Ok(content) => {
            match serde_json::from_str::<OAuthToken>(&content) {
                Ok(token) => {
                    if token.is_expired() {
                        warn!("Token 已过期");
                        None
                    } else {
                        Some(token)
                    }
                }
                Err(e) => {
                    error!("解析 token 失败: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            error!("读取 token 失败: {}", e);
            None
        }
    }
}

/// 更新配置文件中的 API Key（异步版本）
pub async fn update_config_api_key(api_key: &str) -> Result<()> {
    let mut config = hermes_core::Config::load(None).await?;
    config.llm.api_key = api_key.to_string();
    
    // 确保使用正确的端点
    config.llm.base_url = "https://api.kimi.com/coding/v1".to_string();
    config.llm.model = "kimi-for-coding".to_string();
    
    // 保存配置
    let config_path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hermes")
        .join("config.toml");
    
    config.save(config_path).await?;
    
    println!("✅ 配置文件已更新");
    Ok(())
}
