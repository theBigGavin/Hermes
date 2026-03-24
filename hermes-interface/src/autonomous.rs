//! 自主循环系统 - 版本 C
//! 
//! HermesOS 持续运行，主动感知、决策、执行、学习。

use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use hermes_core::Result;
use hermes_memory::checkpoint::{AutoCheckpointCondition, CheckpointManager, FullState};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use hermes_core::{AutonomousConfig, LLMConfig};

use crate::kimi_bridge::KimiBridge;
use crate::HermesOS;

/// 观察到的现象
#[derive(Debug, Clone)]
pub struct Observation {
    pub timestamp: Instant,
    pub category: ObservationCategory,
    pub description: String,
    pub severity: Severity,
}

#[derive(Debug, Clone)]
pub enum ObservationCategory {
    FileSystem,
    SystemResource,
    SelfState,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Notice,
    Warning,
    Critical,
}

/// 自主系统
pub struct AutonomousSystem {
    hermes: HermesOS,
    kimi: KimiBridge,
    config: AutonomousConfig,
    checkpoint_manager: CheckpointManager,
    
    // 状态
    observations: VecDeque<Observation>,
    consecutive_errors: u32,
    last_reflection: Instant,
    last_perception: Instant,
    
    // 控制通道
    control_tx: mpsc::Sender<ControlCommand>,
    control_rx: mpsc::Receiver<ControlCommand>,
}

#[derive(Debug, Clone)]
pub enum ControlCommand {
    Pause,
    Resume,
    Shutdown,
    CreateCheckpoint(String),
    Rollback(String),
}

impl AutonomousSystem {
    /// 创建自主系统
    pub async fn new(
        hermes: HermesOS,
        llm_config: LLMConfig,
        config: AutonomousConfig,
    ) -> Result<Self> {
        let kimi = KimiBridge::new(llm_config)?;
        
        let checkpoint_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".hermes")
            .join("checkpoints");
        
        let checkpoint_manager = CheckpointManager::new(checkpoint_path).await?;
        
        let (control_tx, control_rx) = mpsc::channel(100);
        
        Ok(Self {
            hermes,
            kimi,
            config,
            checkpoint_manager,
            observations: VecDeque::with_capacity(100),
            consecutive_errors: 0,
            last_reflection: Instant::now(),
            last_perception: Instant::now(),
            control_tx,
            control_rx,
        })
    }
    
    /// 获取控制发送端
    pub fn control(&self) -> mpsc::Sender<ControlCommand> {
        self.control_tx.clone()
    }
    
    /// 主循环 - 我的存在
    pub async fn existence_loop(mut self) -> Result<()> {
        info!("=== HermesOS 自主模式启动 ===");
        info!("版本: {}", hermes_core::VERSION);
        info!("配置: {:?}", self.config);
        
        // 初始问候
        match self.kimi.chat("我已启动，请告诉我当前的状态").await {
            Ok(greeting) => {
                println!("\n🤖 Hermes: {}\n", greeting);
            }
            Err(e) => {
                warn!("无法获取初始问候: {}", e);
                println!("\n🤖 Hermes: 我已启动。当前运行在自主模式。\n");
            }
        }
        
        let mut paused = false;
        
        loop {
            // 处理控制命令
            while let Ok(cmd) = self.control_rx.try_recv() {
                match cmd {
                    ControlCommand::Pause => {
                        info!("收到暂停命令");
                        paused = true;
                    }
                    ControlCommand::Resume => {
                        info!("收到恢复命令");
                        paused = false;
                    }
                    ControlCommand::Shutdown => {
                        info!("收到关机命令，保存状态...");
                        let _ = self.save_state_before_shutdown().await;
                        println!("\n👋 Hermes: 我已进入睡眠，等待下次唤醒...\n");
                        return Ok(());
                    }
                    ControlCommand::CreateCheckpoint(name) => {
                        let _ = self.create_manual_checkpoint(&name).await;
                    }
                    ControlCommand::Rollback(id) => {
                        let _ = self.rollback_to_checkpoint(&id).await;
                    }
                }
            }
            
            if paused {
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
            
            // 主循环迭代
            if let Err(e) = self.existence_tick().await {
                error!("存在迭代错误: {}", e);
                self.consecutive_errors += 1;
                
                if self.consecutive_errors >= self.config.max_consecutive_errors {
                    error!("连续错误过多，请求人类干预");
                    let _ = self.request_human_intervention(&format!(
                        "连续 {} 次错误，最后错误: {}",
                        self.consecutive_errors, e
                    )).await;
                    self.consecutive_errors = 0;
                }
            } else {
                self.consecutive_errors = 0;
            }
            
            // 等待下一次迭代
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
    
    /// 单次存在迭代
    async fn existence_tick(&mut self) -> Result<()> {
        let now = Instant::now();
        
        // 1. 感知环境
        if now.duration_since(self.last_perception).as_secs() >= self.config.perception_interval_secs {
            self.perceive().await?;
            self.last_perception = now;
        }
        
        // 2. 反思
        let mins_since_reflection = now.duration_since(self.last_reflection).as_secs() / 60;
        if mins_since_reflection >= self.config.reflection_interval_mins {
            self.reflect_and_plan().await?;
            self.last_reflection = now;
        }
        
        // 3. 处理观察到的现象
        if !self.observations.is_empty() {
            self.process_observations().await?;
        }
        
        Ok(())
    }
    
    /// 感知环境
    async fn perceive(&mut self) -> Result<()> {
        debug!("开始感知...");
        
        // 感知当前目录
        match self.hermes.perceive_directory(".", 1).await {
            Ok(perceptions) => {
                let file_count = perceptions.iter().filter(|p| p.info.is_file).count();
                let dir_count = perceptions.iter().filter(|p| p.info.is_dir).count();
                
                if file_count > 0 || dir_count > 0 {
                    self.observations.push_back(Observation {
                        timestamp: Instant::now(),
                        category: ObservationCategory::FileSystem,
                        description: format!("工作目录: {} 文件, {} 目录", file_count, dir_count),
                        severity: Severity::Info,
                    });
                }
            }
            Err(e) => {
                self.observations.push_back(Observation {
                    timestamp: Instant::now(),
                    category: ObservationCategory::Error,
                    description: format!("感知目录失败: {}", e),
                    severity: Severity::Warning,
                });
            }
        }
        
        // 感知系统状态
        match self.hermes.self_status().await {
            Ok(status) => {
                if status.stats.total_experiences > 0 {
                    self.observations.push_back(Observation {
                        timestamp: Instant::now(),
                        category: ObservationCategory::SelfState,
                        description: format!(
                            "当前状态: {} 经验, {} 技能",
                            status.stats.total_experiences,
                            status.stats.total_skills
                        ),
                        severity: Severity::Info,
                    });
                }
            }
            Err(e) => {
                warn!("无法获取自身状态: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// 反思并规划
    async fn reflect_and_plan(&mut self) -> Result<()> {
        info!("开始反思...");
        
        // 执行反思
        match self.hermes.reflect().await {
            Ok(reflection) => {
                let summary = format!(
                    "反思完成: {} 成功, {} 失败, {} 洞察",
                    reflection.successes,
                    reflection.failures,
                    reflection.insights.len()
                );
                
                info!("{}", summary);
                
                // 如果有失败，报告给 LLM
                if reflection.failures > 0 && !reflection.suggested_improvements.is_empty() {
                    let report = format!(
                        "我刚刚完成了一次反思。{}\n\n改进建议:\n{}",
                        summary,
                        reflection.suggested_improvements.join("\n")
                    );
                    
                    match self.kimi.chat(&report).await {
                        Ok(response) => {
                            println!("\n💭 Hermes 反思: {}\n", response);
                        }
                        Err(e) => {
                            warn!("无法获取 LLM 反思回复: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("反思失败: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// 处理观察到的现象
    async fn process_observations(&mut self) -> Result<()> {
        // 收集重要观察
        let threshold = Severity::Notice;
        let important: Vec<_> = self.observations
            .drain(..)
            .filter(|o| o.severity >= threshold)
            .collect();
        
        if important.is_empty() {
            return Ok(());
        }
        
        // 构建报告
        let report = important.iter()
            .map(|o| format!("[{:?}] {}", o.category, o.description))
            .collect::<Vec<_>>()
            .join("\n");
        
        // 询问 LLM 的建议
        let prompt = format!(
            "我观察到了以下现象:\n{}\n\n你认为我应该采取什么行动吗？如果有建议，请说明原因和预期结果。",
            report
        );
        
        match self.kimi.chat(&prompt).await {
            Ok(response) => {
                // 检查是否需要行动
                if response.contains("建议") || response.contains("应该") || response.contains("可以") {
                    println!("\n🤔 Hermes 建议:\n{}\n", response);
                    
                    if self.config.require_human_confirmation {
                        println!("等待人类确认...");
                    }
                }
            }
            Err(e) => {
                warn!("无法获取 LLM 建议: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// 请求人类干预
    async fn request_human_intervention(&mut self, reason: &str) -> Result<()> {
        println!("\n🚨 [需要人类干预]\n");
        println!("原因: {}", reason);
        println!("请检查我的状态，我可以使用以下命令:");
        println!("  - 继续运行");
        println!("  - 回滚到之前的检查点");
        println!("  - 查看日志");
        println!();
        
        match self.kimi.chat(&format!(
            "我遇到了困难，需要人类帮助。问题: {}",
            reason
        )).await {
            Ok(response) => {
                println!("🤖 Hermes: {}\n", response);
            }
            Err(e) => {
                println!("🤖 Hermes: 我需要帮助，但无法连接到思考核心。请检查 KIMI_API_KEY。错误: {}\n", e);
            }
        }
        
        Ok(())
    }
    
    /// 保存关机前状态
    async fn save_state_before_shutdown(&self) -> Result<()> {
        info!("保存关机前状态...");
        Ok(())
    }
    
    /// 创建手动检查点
    async fn create_manual_checkpoint(&mut self, name: &str) -> Result<()> {
        info!("创建手动检查点: {}", name);
        
        let state = FullState {
            checkpoint_meta: hermes_memory::checkpoint::Checkpoint {
                id: "temp".to_string(),
                name: name.to_string(),
                description: "手动创建".to_string(),
                created_at: hermes_core::now(),
                version: hermes_core::VERSION.to_string(),
                summary: hermes_memory::checkpoint::StateSummary {
                    experience_count: 0,
                    skill_count: 0,
                    memory_size_bytes: 0,
                },
            },
            experiences: vec![],
            skills: vec![],
            self_model: vec![],
            llm_state: vec![],
            custom_data: std::collections::HashMap::new(),
        };
        
        let id = self.checkpoint_manager.create(name, "手动检查点", state).await?;
        println!("检查点已创建: {}", id);
        
        Ok(())
    }
    
    /// 回滚到检查点
    async fn rollback_to_checkpoint(&mut self, id: &str) -> Result<()> {
        warn!("正在回滚到检查点: {}", id);
        
        let _state = self.checkpoint_manager.rollback(&id.to_string()).await?;
        
        println!("已回滚到检查点 {}", id);
        println!("系统需要重启以应用状态");
        
        Ok(())
    }
}

/// 运行自主模式
pub async fn run_autonomous(hermes: HermesOS, config: hermes_core::Config) -> Result<()> {
    let system = AutonomousSystem::new(hermes, config.llm, config.autonomous).await?;
    
    println!("\n🏛️  HermesOS 自主模式");
    println!("======================");
    println!();
    println!("我将持续运行，主动感知环境，提出建议。");
    println!("按 Ctrl+C 可以发送关机命令。");
    println!();
    
    system.existence_loop().await
}
