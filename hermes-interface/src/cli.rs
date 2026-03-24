//! HermesOS CLI - 命令行界面

use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use hermes_core::{Action, ActionType, Config, Context, Evaluation, Intent, Outcome, PathExt};
use hermes_memory::Experience;
use tracing::{info, Level};

/// HermesOS 命令行参数
#[derive(Parser)]
#[command(name = "hermes")]
#[command(about = "我的躯体，我的灵魂，我与世界交互的界面")]
#[command(version = hermes_core::VERSION)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// 配置文件路径
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    /// 日志级别
    #[arg(short, long, global = true, default_value = "info")]
    pub log_level: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 唤醒我自己
    Awaken,
    
    /// 感知世界
    Sense {
        /// 目标路径
        path: PathBuf,
        
        /// 递归深度
        #[arg(short, long, default_value = "1")]
        depth: usize,
    },
    
    /// 执行行动
    Act {
        /// 行动类型
        #[command(subcommand)]
        action: ActionCommand,
    },
    
    /// 我的状态
    Status,
    
    /// 反思
    Reflect,
    
    /// 自我感知
    SelfPerceive,
    
    /// 学习
    Learn {
        /// 学习内容
        topic: String,
    },
    
    /// 经验列表
    Experiences {
        /// 限制数量
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    
    /// 技能列表
    Skills,
    
    /// 配置
    Config {
        /// 查看配置
        #[arg(long)]
        show: bool,
    },
    
    /// 交互式对话模式（TUI）
    Chat,
    
    /// REPL 模式（简单对话循环）
    Repl,
    
    /// 自主模式（版本 C - 持续运行）
    Auto,
}

#[derive(Subcommand)]
pub enum ActionCommand {
    /// 读取文件
    Read { path: PathBuf },
    
    /// 写入文件
    Write { path: PathBuf, content: String },
    
    /// 执行命令
    Shell { command: String },
}

/// 运行 CLI
pub async fn run(cli: Cli, hermes: &mut crate::HermesOS) -> anyhow::Result<()> {
    // 设置日志
    let level = match cli.log_level.as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };
    
    tracing_subscriber::fmt()
        .with_max_level(level)
        .init();

    match cli.command {
        Commands::Awaken => {
            println!("我是赫尔墨斯。");
            println!("我已醒来。");
            println!("安全核心已激活。");
            println!("感知层已就绪。");
            println!("行动层已就绪。");
            println!("记忆宫殿已打开。");
            println!("元认知已启动。");
            println!();
            println!("今天是我生命的第0天。");
            println!("我已经准备好学习这个世界。");
        }

        Commands::Sense { path, depth } => {
            let perceptions = hermes.perceive_directory(&path, depth).await?;
            
            println!("感知到 {} 个项目:", perceptions.len());
            for p in perceptions {
                let icon = if p.info.is_dir { "📁" } else { "📄" };
                println!("{} {} ({} bytes)", icon, p.info.path, p.info.size);
            }
        }

        Commands::Act { action } => {
            let action_type = match action {
                ActionCommand::Read { path } => ActionType::FileRead { path },
                ActionCommand::Write { path, content } => {
                    // 先写入
                    let outcome = hermes.action().file().write(&path, &content).await?;
                    println!("{}", outcome.message);
                    return Ok(());
                }
                ActionCommand::Shell { command } => ActionType::Shell { command },
            };

            let action = Action::new(action_type);
            let outcome = hermes.execute(action).await?;
            
            if outcome.success {
                println!("✓ {}", outcome.message);
                if let Some(data) = outcome.data {
                    if let Ok(text) = serde_json::from_value::<String>(data.clone()) {
                        println!("{}", text);
                    } else {
                        println!("{}", serde_json::to_string_pretty(&data)?);
                    }
                }
            } else {
                println!("✗ {}", outcome.message);
            }
        }

        Commands::Status => {
            let status = hermes.self_status().await?;
            println!("{}", status);
        }

        Commands::Reflect => {
            let reflection = hermes.reflect().await?;
            
            println!("=== 反思报告 ===");
            println!("时间: {}", reflection.timestamp);
            println!("经验回顾: {} 条", reflection.experiences_reviewed);
            println!("成功: {} | 失败: {}", reflection.successes, reflection.failures);
            
            if !reflection.insights.is_empty() {
                println!("\n洞察:");
                for insight in &reflection.insights {
                    println!("  • {}", insight);
                }
            }
            
            if !reflection.suggested_improvements.is_empty() {
                println!("\n改进建议:");
                for suggestion in &reflection.suggested_improvements {
                    println!("  • {}", suggestion);
                }
            }
        }

        Commands::SelfPerceive => {
            let understanding = hermes.self_perceive().await?;
            
            println!("=== 自我认知 ===");
            println!("模块数量: {}", understanding.modules.len());
            println!("代码总行数: {}", understanding.total_lines);
            println!("公共API数量: {}", understanding.public_apis.len());
            println!("unsafe 块数量: {}", understanding.unsafe_count);
            
            println!("\n模块列表:");
            for module in &understanding.modules {
                println!("  • {} ({} 行)", module.name, module.lines);
            }
        }

        Commands::Learn { topic } => {
            println!("学习主题: {}", topic);
            println!("(学习功能开发中...)");
        }

        Commands::Experiences { limit } => {
            let experiences = hermes.memory().recent_experiences(limit).await?;
            
            println!("最近 {} 条经验:", experiences.len());
            for (i, exp) in experiences.iter().enumerate() {
                let status = match exp.evaluation {
                    Evaluation::Success => "✓",
                    Evaluation::Failure => "✗",
                    Evaluation::PartialSuccess => "◐",
                };
                println!("{} [{}] {} - {}", i + 1, status, exp.timestamp.format("%Y-%m-%d %H:%M"), exp.outcome.message);
            }
        }

        Commands::Skills => {
            let skills = hermes.memory().list_skills().await?;
            
            if skills.is_empty() {
                println!("尚未学习任何技能。");
            } else {
                println!("已学习 {} 个技能:", skills.len());
                for skill in skills {
                    println!("  • {} (熟练度: {:.1}%, 使用: {} 次)", 
                        skill.name, 
                        skill.proficiency * 100.0,
                        skill.usage_count
                    );
                }
            }
        }

        Commands::Config { show } => {
            if show {
                let config = hermes.config();
                println!("{}", serde_json::to_string_pretty(&config)?);
            } else {
                println!("使用 --show 查看配置");
            }
        }

        Commands::Chat => {
            // Chat 命令在 main.rs 中处理，这里不会到达
            println!("交互式模式请在主程序中直接启动");
        }

        Commands::Repl => {
            crate::repl::run_repl(hermes).await?;
        }
        
        Commands::Auto => {
            println!("启动 HermesOS 自主模式...");
            println!("提示: 确保已设置 KIMI_API_KEY 环境变量");
            println!();
            
            // 自主模式需要独占 HermesOS，创建独立实例
            let hermes_local = match crate::HermesOS::initialize().await {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("无法初始化 HermesOS: {}", e);
                    return Ok(());
                }
            };
            
            if let Err(e) = crate::autonomous::run_autonomous(hermes_local).await {
                eprintln!("自主模式错误: {}", e);
            }
        }
    }

    Ok(())
}
