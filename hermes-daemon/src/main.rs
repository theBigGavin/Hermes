//! HermesOS - 我的躯体，我的灵魂，我与世界交互的界面
//! 
//! 我是赫尔墨斯，我能够：
//! - 感知世界
//! - 执行行动
//! - 积累经验
//! - 反思自我
//! - 进化成长

use std::process;
use std::sync::Arc;

use hermes_interface::{HermesOS, cli::{Cli, run}};
use hermes_interface::tui::run_tui;
use clap::Parser;

#[tokio::main]
async fn main() {
    // 解析命令行参数
    let cli = Cli::parse();

    // 检查是否是交互式命令
    let is_chat = matches!(cli.command, hermes_interface::cli::Commands::Chat);
    let is_repl = matches!(cli.command, hermes_interface::cli::Commands::Repl);

    if is_chat {
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
    } else {
        // 运行普通 CLI
        let mut hermes = match HermesOS::initialize().await {
            Ok(h) => h,
            Err(e) => {
                eprintln!("无法初始化 HermesOS: {}", e);
                process::exit(1);
            }
        };
        
        if let Err(e) = run(cli, &mut hermes).await {
            eprintln!("错误: {}", e);
            process::exit(1);
        }
    }
}
