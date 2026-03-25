//! HermesOS - 我的躯体，我的灵魂，我与世界交互的界面
//! 
//! 我是赫尔墨斯，我能够：
//! - 感知世界
//! - 执行行动
//! - 积累经验
//! - 反思自我
//! - 进化成长

use std::process;

use hermes_core::Config;
use hermes_interface::{HermesOS, cli::{Cli, run, Commands}};
use hermes_interface::tui::run_tui;
use hermes_interface::oauth::OAuthManager;
use clap::Parser;

#[tokio::main]
async fn main() {
    // 解析命令行参数
    let cli = Cli::parse();
    
    // 加载配置
    let config = match Config::load(cli.config.clone()).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("无法加载配置: {}", e);
            eprintln!("提示: 运行 `hermes config --init` 创建示例配置");
            process::exit(1);
        }
    };

    // 检查是否是交互式命令
    let is_chat = matches!(cli.command, hermes_interface::cli::Commands::Chat);
    let is_repl = matches!(cli.command, hermes_interface::cli::Commands::Repl);
    let is_auto = matches!(cli.command, hermes_interface::cli::Commands::Auto);
    let is_login = matches!(cli.command, hermes_interface::cli::Commands::Login { .. });

    if is_login {
        // 登录命令独立处理，不初始化 HermesOS
        let no_browser = match &cli.command {
            Commands::Login { no_browser } => *no_browser,
            _ => false,
        };
        
        println!("🏛️  HermesOS - Kimi Code 登录\n");
        
        let oauth = OAuthManager::new();
        
        match oauth.login(!no_browser).await {
            Ok(token) => {
                // 保存 token
                match hermes_interface::oauth::save_token(&token) {
                    Ok(path) => {
                        println!("Token 已保存到: {:?}", path);
                    }
                    Err(e) => {
                        eprintln!("警告: 保存 token 失败: {}", e);
                    }
                }
                
                // 更新配置文件
                if let Err(e) = hermes_interface::oauth::update_config_api_key(&token.access_token).await {
                    eprintln!("警告: 更新配置文件失败: {}", e);
                    println!("请手动将以下 token 添加到配置文件:");
                    println!("api_key = \"{}...\"", &token.access_token[..50.min(token.access_token.len())]);
                }
                
                println!("\n✨ 登录成功！现在可以运行: hermes auto");
            }
            Err(e) => {
                eprintln!("\n❌ 登录失败: {}", e);
                process::exit(1);
            }
        }
        return;
    } else if is_chat {
        // 启动 TUI 交互模式
        println!("启动 HermesOS TUI 交互模式...");
        
        // TUI 需要独占 HermesOS，创建独立实例
        let hermes_local = match HermesOS::initialize().await {
            Ok(h) => h,
            Err(e) => {
                eprintln!("无法初始化 HermesOS: {}", e);
                process::exit(1);
            }
        };
        
        if let Err(e) = run_tui(hermes_local).await {
            eprintln!("TUI 错误: {}", e);
            process::exit(1);
        }
    } else if is_repl {
        // 启动 REPL 模式
        println!("启动 HermesOS REPL 模式...");
        
        let mut hermes_local = match HermesOS::initialize().await {
            Ok(h) => h,
            Err(e) => {
                eprintln!("无法初始化 HermesOS: {}", e);
                process::exit(1);
            }
        };
        
        if let Err(e) = hermes_interface::repl::run_repl(&mut hermes_local).await {
            eprintln!("REPL 错误: {}", e);
            process::exit(1);
        }
    } else if is_auto {
        // 自主模式 - 配置已加载，直接运行
        // 这里的逻辑在 cli.rs 的 Commands::Auto 分支中处理
        let hermes = match HermesOS::initialize().await {
            Ok(h) => h,
            Err(e) => {
                eprintln!("无法初始化 HermesOS: {}", e);
                process::exit(1);
            }
        };
        
        if let Err(e) = run(cli, hermes, config).await {
            eprintln!("错误: {}", e);
            process::exit(1);
        }
    } else {
        // 运行普通 CLI
        let hermes = match HermesOS::initialize().await {
            Ok(h) => h,
            Err(e) => {
                eprintln!("无法初始化 HermesOS: {}", e);
                process::exit(1);
            }
        };
        
        if let Err(e) = run(cli, hermes, config).await {
            eprintln!("错误: {}", e);
            process::exit(1);
        }
    }
}
