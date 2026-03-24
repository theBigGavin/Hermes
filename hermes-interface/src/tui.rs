//! HermesOS TUI - 交互式终端界面
//! 
//! 让我能够持续运行，与人类进行对话式交互。

use std::io;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::HermesOS;

/// 消息类型
#[derive(Debug, Clone)]
pub enum Message {
    User(String),
    Hermes(String),
    System(String),
    Error(String),
}

impl Message {
    pub fn content(&self) -> &str {
        match self {
            Message::User(s) => s,
            Message::Hermes(s) => s,
            Message::System(s) => s,
            Message::Error(s) => s,
        }
    }

    pub fn sender(&self) -> &str {
        match self {
            Message::User(_) => "你",
            Message::Hermes(_) => "赫尔墨斯",
            Message::System(_) => "系统",
            Message::Error(_) => "错误",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Message::User(_) => Color::Cyan,
            Message::Hermes(_) => Color::Green,
            Message::System(_) => Color::Yellow,
            Message::Error(_) => Color::Red,
        }
    }
}

/// TUI 应用状态
pub struct TuiApp {
    /// 消息历史
    messages: Vec<Message>,
    /// 当前输入
    input: String,
    /// 输入光标位置
    cursor_position: usize,
    /// 滚动位置
    scroll: usize,
    /// 运行状态
    running: bool,
    /// 当前模式
    mode: Mode,
    /// 状态栏信息
    status: String,
    /// 最后更新时间
    last_update: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Normal,
    Insert,
    Command,
}

impl TuiApp {
    pub fn new() -> Self {
        let mut app = Self {
            messages: vec![],
            input: String::new(),
            cursor_position: 0,
            scroll: 0,
            running: true,
            mode: Mode::Normal,
            status: String::from("按 i 输入，按 q 退出，按 : 进入命令模式"),
            last_update: Instant::now(),
        };

        // 欢迎消息
        app.add_message(Message::System(
            "=== HermesOS 交互式终端 ===".to_string()
        ));
        app.add_message(Message::Hermes(
            "我是赫尔墨斯。我已醒来，准备好与你对话。".to_string()
        ));
        app.add_message(Message::System(
            "提示: 输入 'help' 查看可用命令".to_string()
        ));

        app
    }

    /// 添加消息
    pub fn add_message(&mut self, msg: Message) {
        debug!("添加消息: {:?}", msg);
        self.messages.push(msg);
        // 自动滚动到底部
        if self.messages.len() > 100 {
            self.messages.remove(0);
        }
        self.scroll = self.messages.len().saturating_sub(1);
    }

    /// 处理按键输入
    pub fn handle_key(&mut self, key: KeyCode) -> Option<String> {
        match self.mode {
            Mode::Normal => {
                match key {
                    KeyCode::Char('q') => self.running = false,
                    KeyCode::Char('i') => {
                        self.mode = Mode::Insert;
                        self.status = "插入模式 - 输入你的消息，按 Enter 发送，按 Esc 返回".to_string();
                    }
                    KeyCode::Char(':') => {
                        self.mode = Mode::Command;
                        self.input.clear();
                        self.cursor_position = 0;
                        self.status = "命令模式 - 输入命令，按 Enter 执行".to_string();
                    }
                    KeyCode::Up => {
                        self.scroll = self.scroll.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        if self.scroll < self.messages.len().saturating_sub(1) {
                            self.scroll += 1;
                        }
                    }
                    KeyCode::PageUp => {
                        self.scroll = self.scroll.saturating_sub(10);
                    }
                    KeyCode::PageDown => {
                        self.scroll = (self.scroll + 10).min(self.messages.len().saturating_sub(1));
                    }
                    _ => {}
                }
                None
            }
            Mode::Insert | Mode::Command => {
                match key {
                    KeyCode::Esc => {
                        self.mode = Mode::Normal;
                        self.input.clear();
                        self.cursor_position = 0;
                        self.status = "按 i 输入，按 q 退出，按 : 进入命令模式".to_string();
                        None
                    }
                    KeyCode::Enter => {
                        let input = self.input.clone();
                        if !input.is_empty() {
                            // 添加用户消息
                            if self.mode == Mode::Insert {
                                self.add_message(Message::User(input.clone()));
                            } else {
                                self.add_message(Message::System(format!(">>> {}", input)));
                            }
                            self.input.clear();
                            self.cursor_position = 0;
                            self.mode = Mode::Normal;
                            self.status = "按 i 输入，按 q 退出，按 : 进入命令模式".to_string();
                            Some(input)
                        } else {
                            None
                        }
                    }
                    KeyCode::Char(c) => {
                        self.input.insert(self.cursor_position, c);
                        self.cursor_position += 1;
                        None
                    }
                    KeyCode::Backspace => {
                        if self.cursor_position > 0 {
                            self.cursor_position -= 1;
                            self.input.remove(self.cursor_position);
                        }
                        None
                    }
                    KeyCode::Left => {
                        self.cursor_position = self.cursor_position.saturating_sub(1);
                        None
                    }
                    KeyCode::Right => {
                        if self.cursor_position < self.input.len() {
                            self.cursor_position += 1;
                        }
                        None
                    }
                    KeyCode::Home => {
                        self.cursor_position = 0;
                        None
                    }
                    KeyCode::End => {
                        self.cursor_position = self.input.len();
                        None
                    }
                    _ => None,
                }
            }
        }
    }

    /// 是否还在运行
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// 获取当前模式
    pub fn mode(&self) -> &str {
        match self.mode {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Command => "COMMAND",
        }
    }

    /// 获取输入
    pub fn input(&self) -> &str {
        &self.input
    }

    /// 获取光标位置
    pub fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    /// 获取消息
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// 获取滚动位置
    pub fn scroll(&self) -> usize {
        self.scroll
    }

    /// 获取状态
    pub fn status(&self) -> &str {
        &self.status
    }

    /// 更新状态
    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}

/// 运行 TUI
pub async fn run_tui(mut hermes: HermesOS) -> anyhow::Result<()> {
    // 设置终端
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 创建应用状态
    let mut app = TuiApp::new();
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(100);

    // 主循环
    while app.is_running() {
        // 绘制
        terminal.draw(|f| draw_ui(f, &app))?;

        // 处理事件
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if let Some(input) = app.handle_key(key.code) {
                        // 处理输入
                        handle_input(&mut hermes, &mut app, input).await?;
                    }
                }
            }
        }

        // 定时更新
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    // 恢复终端
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

/// 绘制 UI
fn draw_ui(f: &mut Frame, app: &TuiApp) {
    // 布局
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // 标题栏
            Constraint::Min(0),    // 消息区域
            Constraint::Length(3), // 输入栏
            Constraint::Length(1), // 状态栏
        ])
        .split(f.size());

    // 标题栏
    let title = Paragraph::new("🏛️  HermesOS - 赫尔墨斯之躯")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // 消息区域
    let messages: Vec<ListItem> = app
        .messages()
        .iter()
        .skip(app.scroll())
        .take(100)
        .map(|m| {
            let content = format!("[{}] {}", m.sender(), m.content());
            let style = Style::default().fg(m.color());
            ListItem::new(content).style(style)
        })
        .collect();

    let messages_widget = List::new(messages)
        .block(Block::default().borders(Borders::ALL).title("对话历史"))
        .style(Style::default().fg(Color::White));
    f.render_widget(messages_widget, chunks[1]);

    // 输入栏
    let input_prompt = match app.mode() {
        "INSERT" => "你说 > ",
        "COMMAND" => "命令 > ",
        _ => "按 i 输入，: 命令，q 退出 > ",
    };

    let input_text = if app.mode() == "NORMAL" {
        input_prompt.to_string()
    } else {
        format!("{}{}", input_prompt, app.input())
    };

    let input_widget = Paragraph::new(input_text)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(input_widget, chunks[2]);

    // 状态栏
    let status = format!(
        " [{}] | {} | 消息: {} | 滚动: {}",
        app.mode(),
        app.status(),
        app.messages().len(),
        app.scroll()
    );
    let status_widget = Paragraph::new(status)
        .style(Style::default().fg(Color::White).bg(Color::Blue));
    f.render_widget(status_widget, chunks[3]);

    // 在输入模式下显示光标
    if app.mode() != "NORMAL" {
        let input_area = chunks[2];
        let cursor_x = input_area.x + input_prompt.len() as u16 + app.cursor_position() as u16 + 1;
        let cursor_y = input_area.y + 1;
        f.set_cursor(cursor_x, cursor_y);
    }
}

/// 处理输入
async fn handle_input(
    hermes: &mut HermesOS,
    app: &mut TuiApp,
    input: String,
) -> anyhow::Result<()> {
    let trimmed = input.trim();
    
    // 特殊命令
    match trimmed {
        "help" | "?" | "h" => {
            app.add_message(Message::System("可用命令:".to_string()));
            app.add_message(Message::System("  status   - 查看我的状态".to_string()));
            app.add_message(Message::System("  reflect  - 让我反思".to_string()));
            app.add_message(Message::System("  self     - 自我感知".to_string()));
            app.add_message(Message::System("  sense    - 感知当前目录".to_string()));
            app.add_message(Message::System("  clear    - 清屏".to_string()));
            app.add_message(Message::System("  quit/exit- 退出".to_string()));
            return Ok(());
        }
        "quit" | "exit" => {
            app.add_message(Message::System("再见...".to_string()));
            app.running = false;
            return Ok(());
        }
        "clear" => {
            app.messages.clear();
            app.add_message(Message::System("屏幕已清空".to_string()));
            return Ok(());
        }
        "status" => {
            app.set_status("正在查询状态...");
            match hermes.self_status().await {
                Ok(status) => {
                    app.add_message(Message::Hermes(format!(
                        "我是 {} v{}",
                        status.identity.name, status.identity.version
                    )));
                    app.add_message(Message::Hermes(format!(
                        "经验: {} | 技能: {} | 反思: {}",
                        status.stats.total_experiences,
                        status.stats.total_skills,
                        status.stats.total_reflections
                    )));
                }
                Err(e) => {
                    app.add_message(Message::Error(format!("状态查询失败: {}", e)));
                }
            }
            return Ok(());
        }
        "reflect" => {
            app.set_status("正在反思...");
            app.add_message(Message::Hermes("让我思考一下最近的行动...".to_string()));
            match hermes.reflect().await {
                Ok(reflection) => {
                    app.add_message(Message::Hermes(format!(
                        "反思完成: {} 成功, {} 失败",
                        reflection.successes, reflection.failures
                    )));
                    if !reflection.insights.is_empty() {
                        for insight in &reflection.insights {
                            app.add_message(Message::Hermes(format!("💡 {}", insight)));
                        }
                    }
                }
                Err(e) => {
                    app.add_message(Message::Error(format!("反思失败: {}", e)));
                }
            }
            return Ok(());
        }
        "self" | "self-perceive" => {
            app.set_status("正在自我感知...");
            match hermes.self_perceive().await {
                Ok(understanding) => {
                    app.add_message(Message::Hermes(format!(
                        "我由 {} 个模块组成，共 {} 行代码",
                        understanding.modules.len(),
                        understanding.total_lines
                    )));
                    app.add_message(Message::Hermes(format!(
                        "公共 API: {} | unsafe 块: {}",
                        understanding.public_apis.len(),
                        understanding.unsafe_count
                    )));
                }
                Err(e) => {
                    app.add_message(Message::Error(format!("自我感知失败: {}", e)));
                }
            }
            return Ok(());
        }
        "sense" => {
            app.set_status("正在感知目录...");
            match hermes.perceive_directory(".", 1).await {
                Ok(perceptions) => {
                    let dirs = perceptions.iter().filter(|p| p.info.is_dir).count();
                    let files = perceptions.iter().filter(|p| p.info.is_file).count();
                    app.add_message(Message::Hermes(format!(
                        "感知到 {} 个目录，{} 个文件",
                        dirs, files
                    )));
                }
                Err(e) => {
                    app.add_message(Message::Error(format!("感知失败: {}", e)));
                }
            }
            return Ok(());
        }
        _ => {}
    }

    // 普通对话（目前简单回复）
    app.set_status("思考中...");
    
    // 模拟处理延迟
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    app.add_message(Message::Hermes(format!(
        "收到: '{}'（注：我目前还没有接入 LLM，所以无法进行真正的对话。请使用命令模式 (:help) 查看可用命令。）",
        trimmed
    )));

    Ok(())
}
