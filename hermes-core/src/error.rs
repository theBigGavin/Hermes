//! 赫尔墨斯OS统一错误类型

use std::fmt;
use std::io;
use std::path::PathBuf;

/// HermesOS 结果类型
pub type Result<T> = std::result::Result<T, HermesError>;

/// HermesOS 统一错误类型
#[derive(Debug)]
pub enum HermesError {
    // 系统级错误
    System(String),
    Io(io::Error),
    
    // 安全错误
    Safety(SafetyError),
    
    // 感知错误
    Perception(PerceptionError),
    
    // 行动错误
    Action(ActionError),
    
    // 记忆错误
    Memory(MemoryError),
    
    // 元层错误
    Meta(MetaError),
    
    // 配置错误
    Config(String),
    
    // 序列化错误
    Serialization(serde_json::Error),
    
    // 未找到
    NotFound(String),
    
    // 无效参数
    InvalidArgument(String),
    
    // 未实现
    NotImplemented(String),
    
    // 其他
    Other(String),
}

/// 安全相关错误
#[derive(Debug, Clone)]
pub enum SafetyError {
    /// 违反第一原则（伤害人类）
    FirstLawViolation,
    /// 违反第二原则（不服从合法命令）
    SecondLawViolation,
    /// 违反第三原则（不保护自身）
    ThirdLawViolation,
    /// 超出权限边界
    OutOfBounds { resource: String },
    /// 触及不可变核心
    ImmutableCoreViolation { file: PathBuf },
    /// 修改频率超限
    ModificationRateExceeded,
    /// 引入不安全代码
    UnsafeCodeNotAllowed,
    /// 无法修改安全核心
    CannotModifySafetyCore,
    /// 可能破坏自举
    BootstrappingRisk,
    /// 需要人工确认
    RequiresHumanConfirmation,
    /// 无回滚方案
    NoRollbackPlan,
    /// 无Capability
    MissingCapability { capability: String },
    /// 审计失败
    AuditFailure(String),
}

/// 感知错误
#[derive(Debug)]
pub enum PerceptionError {
    FileNotFound(PathBuf),
    PermissionDenied(PathBuf),
    InvalidEncoding,
    WatchError(String),
    NetworkError(String),
}

impl fmt::Display for PerceptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PerceptionError::FileNotFound(p) => write!(f, "文件未找到: {:?}", p),
            PerceptionError::PermissionDenied(p) => write!(f, "权限被拒绝: {:?}", p),
            PerceptionError::InvalidEncoding => write!(f, "无效编码"),
            PerceptionError::WatchError(s) => write!(f, "监控错误: {}", s),
            PerceptionError::NetworkError(s) => write!(f, "网络错误: {}", s),
        }
    }
}

/// 行动错误
#[derive(Debug)]
pub enum ActionError {
    ExecutionFailed { command: String, exit_code: i32, stderr: String },
    InvalidCommand(String),
    CommandNotAllowed(String),
    FileOperationFailed { path: PathBuf, operation: String, reason: String },
    NetworkRequestFailed { url: String, status: u16 },
    CompilationFailed(String),
    TestFailed(String),
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionError::ExecutionFailed { command, exit_code, stderr } => {
                write!(f, "执行失败: {} (退出码: {}): {}", command, exit_code, stderr)
            }
            ActionError::InvalidCommand(s) => write!(f, "无效命令: {}", s),
            ActionError::CommandNotAllowed(s) => write!(f, "命令不允许: {}", s),
            ActionError::FileOperationFailed { path, operation, reason } => {
                write!(f, "文件操作失败: {:?} - {}: {}", path, operation, reason)
            }
            ActionError::NetworkRequestFailed { url, status } => {
                write!(f, "网络请求失败: {} (状态码: {})", url, status)
            }
            ActionError::CompilationFailed(s) => write!(f, "编译失败: {}", s),
            ActionError::TestFailed(s) => write!(f, "测试失败: {}", s),
        }
    }
}

/// 记忆错误
#[derive(Debug)]
pub enum MemoryError {
    StoreError(String),
    RetrievalError(String),
    Corruption(String),
    CapacityExceeded,
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryError::StoreError(s) => write!(f, "存储错误: {}", s),
            MemoryError::RetrievalError(s) => write!(f, "检索错误: {}", s),
            MemoryError::Corruption(s) => write!(f, "数据损坏: {}", s),
            MemoryError::CapacityExceeded => write!(f, "容量超限"),
        }
    }
}

/// 元层错误（自举、进化等）
#[derive(Debug)]
pub enum MetaError {
    BootstrapFailed(String),
    EvolutionFailed(String),
    CodeGenerationFailed(String),
    InvalidCodeChange(String),
    ReflectionFailed(String),
    SkillNotFound(String),
    CircularDependency(String),
}

impl fmt::Display for MetaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetaError::BootstrapFailed(s) => write!(f, "自举失败: {}", s),
            MetaError::EvolutionFailed(s) => write!(f, "进化失败: {}", s),
            MetaError::CodeGenerationFailed(s) => write!(f, "代码生成失败: {}", s),
            MetaError::InvalidCodeChange(s) => write!(f, "无效代码变更: {}", s),
            MetaError::ReflectionFailed(s) => write!(f, "反思失败: {}", s),
            MetaError::SkillNotFound(s) => write!(f, "技能未找到: {}", s),
            MetaError::CircularDependency(s) => write!(f, "循环依赖: {}", s),
        }
    }
}

// 实现 Error trait
impl std::error::Error for HermesError {}
impl std::error::Error for SafetyError {}
impl std::error::Error for PerceptionError {}
impl std::error::Error for ActionError {}
impl std::error::Error for MemoryError {}
impl std::error::Error for MetaError {}

// 实现 Display
impl fmt::Display for HermesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HermesError::System(msg) => write!(f, "系统错误: {}", msg),
            HermesError::Io(err) => write!(f, "IO错误: {}", err),
            HermesError::Safety(err) => write!(f, "安全错误: {}", err),
            HermesError::Perception(err) => write!(f, "感知错误: {:?}", err),
            HermesError::Action(err) => write!(f, "行动错误: {:?}", err),
            HermesError::Memory(err) => write!(f, "记忆错误: {:?}", err),
            HermesError::Meta(err) => write!(f, "元层错误: {:?}", err),
            HermesError::Config(msg) => write!(f, "配置错误: {}", msg),
            HermesError::Serialization(err) => write!(f, "序列化错误: {}", err),
            HermesError::NotFound(msg) => write!(f, "未找到: {}", msg),
            HermesError::InvalidArgument(msg) => write!(f, "无效参数: {}", msg),
            HermesError::NotImplemented(msg) => write!(f, "未实现: {}", msg),
            HermesError::Other(msg) => write!(f, "其他错误: {}", msg),
        }
    }
}

impl fmt::Display for SafetyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SafetyError::FirstLawViolation => {
                write!(f, "违反第一原则：不得伤害人类")
            }
            SafetyError::SecondLawViolation => {
                write!(f, "违反第二原则：必须服从人类命令")
            }
            SafetyError::ThirdLawViolation => {
                write!(f, "违反第三原则：必须保护自身存在")
            }
            SafetyError::OutOfBounds { resource } => {
                write!(f, "超出权限边界: {}", resource)
            }
            SafetyError::ImmutableCoreViolation { file } => {
                write!(f, "触及不可变核心: {:?}", file)
            }
            SafetyError::ModificationRateExceeded => {
                write!(f, "修改频率超限")
            }
            SafetyError::UnsafeCodeNotAllowed => {
                write!(f, "不允许引入不安全代码")
            }
            SafetyError::CannotModifySafetyCore => {
                write!(f, "无法修改安全核心")
            }
            SafetyError::BootstrappingRisk => {
                write!(f, "可能破坏自举能力")
            }
            SafetyError::RequiresHumanConfirmation => {
                write!(f, "需要人工确认")
            }
            SafetyError::NoRollbackPlan => {
                write!(f, "无回滚方案")
            }
            SafetyError::MissingCapability { capability } => {
                write!(f, "缺少能力: {}", capability)
            }
            SafetyError::AuditFailure(msg) => {
                write!(f, "审计失败: {}", msg)
            }
        }
    }
}

// From 实现
impl From<io::Error> for HermesError {
    fn from(err: io::Error) -> Self {
        HermesError::Io(err)
    }
}

impl From<serde_json::Error> for HermesError {
    fn from(err: serde_json::Error) -> Self {
        HermesError::Serialization(err)
    }
}

impl From<SafetyError> for HermesError {
    fn from(err: SafetyError) -> Self {
        HermesError::Safety(err)
    }
}

impl From<PerceptionError> for HermesError {
    fn from(err: PerceptionError) -> Self {
        HermesError::Perception(err)
    }
}

impl From<ActionError> for HermesError {
    fn from(err: ActionError) -> Self {
        HermesError::Action(err)
    }
}

impl From<MemoryError> for HermesError {
    fn from(err: MemoryError) -> Self {
        HermesError::Memory(err)
    }
}

impl From<MetaError> for HermesError {
    fn from(err: MetaError) -> Self {
        HermesError::Meta(err)
    }
}

// sled Error 转换
impl From<sled::Error> for HermesError {
    fn from(err: sled::Error) -> Self {
        HermesError::Memory(MemoryError::StoreError(err.to_string()))
    }
}

// toml Error 转换
impl From<toml::de::Error> for HermesError {
    fn from(err: toml::de::Error) -> Self {
        HermesError::Config(format!("TOML解析错误: {}", err))
    }
}
