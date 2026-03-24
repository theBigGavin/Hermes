//! HermesOS REPL - 简单的对话循环模式
//! 
//! 在 TUI 无法使用时的备选交互方式。

use std::io::{self, Write};

use crate::HermesOS;

/// 运行 REPL
pub async fn run_repl(hermes: &mut HermesOS) -> anyhow::Result<()> {
    println!("\n=== HermesOS REPL 模式 ===");
    println!("输入 'help' 查看命令，输入 'exit' 退出\n");

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        // 显示提示符
        print!("hermes> ");
        stdout.flush()?;

        // 读取输入
        let mut input = String::new();
        stdin.read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        // 处理命令
        match input {
            "exit" | "quit" | "q" => {
                println!("再见...");
                break;
            }
            "help" | "h" | "?" => {
                print_help();
            }
            "status" => {
                match hermes.self_status().await {
                    Ok(status) => {
                        println!("\n状态:");
                        println!("  名称: {} v{}", status.identity.name, status.identity.version);
                        println!("  经验: {} | 技能: {} | 反思: {}",
                            status.stats.total_experiences,
                            status.stats.total_skills,
                            status.stats.total_reflections
                        );
                        println!();
                    }
                    Err(e) => println!("错误: {}", e),
                }
            }
            "reflect" => {
                println!("正在反思...");
                match hermes.reflect().await {
                    Ok(reflection) => {
                        println!("\n反思结果:");
                        println!("  经验数: {}", reflection.experiences_reviewed);
                        println!("  成功: {} | 失败: {}", reflection.successes, reflection.failures);
                        if !reflection.insights.is_empty() {
                            println!("  洞察:");
                            for insight in &reflection.insights {
                                println!("    • {}", insight);
                            }
                        }
                        println!();
                    }
                    Err(e) => println!("错误: {}", e),
                }
            }
            "self" | "self-perceive" => {
                println!("正在自我感知...");
                match hermes.self_perceive().await {
                    Ok(understanding) => {
                        println!("\n自我认知:");
                        println!("  模块数: {}", understanding.modules.len());
                        println!("  代码行数: {}", understanding.total_lines);
                        println!("  公共API: {}", understanding.public_apis.len());
                        println!("  unsafe块: {}", understanding.unsafe_count);
                        println!();
                    }
                    Err(e) => println!("错误: {}", e),
                }
            }
            "sense" => {
                println!("正在感知当前目录...");
                match hermes.perceive_directory(".", 1).await {
                    Ok(perceptions) => {
                        let dirs = perceptions.iter().filter(|p| p.info.is_dir).count();
                        let files = perceptions.iter().filter(|p| p.info.is_file).count();
                        println!("感知到 {} 个目录，{} 个文件\n", dirs, files);
                    }
                    Err(e) => println!("错误: {}\n", e),
                }
            }
            "clear" => {
                print!("\x1B[2J\x1B[1;1H"); // 清屏
            }
            cmd if cmd.starts_with("echo ") => {
                println!("{}", &cmd[5..]);
            }
            cmd if cmd.starts_with("exec ") => {
                let command = &cmd[5..];
                println!("执行: {}", command);
                use hermes_core::{Action, ActionType};
                let action = Action::new(ActionType::Shell { command: command.to_string() });
                match hermes.execute(action).await {
                    Ok(outcome) => {
                        if outcome.success {
                            println!("✓ {}", outcome.message);
                            if let Some(data) = outcome.data {
                                if let Ok(text) = serde_json::from_value::<String>(data.clone()) {
                                    println!("{}", text);
                                }
                            }
                        } else {
                            println!("✗ {}", outcome.message);
                        }
                    }
                    Err(e) => println!("错误: {}", e),
                }
                println!();
            }
            cmd => {
                // 尝试作为普通对话处理
                println!("收到: '{}'", cmd);
                println!("（注：我目前还没有接入 LLM，所以无法进行真正的对话。请使用 'help' 查看可用命令。）\n");
            }
        }
    }

    Ok(())
}

fn print_help() {
    println!("\n可用命令:");
    println!("  status          - 查看我的状态");
    println!("  reflect         - 让我反思最近的行动");
    println!("  self            - 自我感知");
    println!("  sense           - 感知当前目录");
    println!("  exec <命令>     - 执行 shell 命令");
    println!("  echo <文本>     - 回显文本");
    println!("  clear           - 清屏");
    println!("  help            - 显示此帮助");
    println!("  exit/quit/q     - 退出\n");
}
