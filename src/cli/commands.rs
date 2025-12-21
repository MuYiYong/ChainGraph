//! 控制台命令处理
//!
//! 处理以 : 开头的控制台命令

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::graph::Graph;

/// 控制台命令执行结果
pub enum CommandResult {
    /// 继续运行
    Continue,
    /// 退出程序
    Exit,
    /// 显示消息
    Message(String),
    /// 错误
    Error(String),
}

/// 控制台状态
pub struct ConsoleState {
    /// 输出到文件
    pub tee_file: Option<File>,
    /// 分页器命令
    pub pager_command: Option<String>,
    /// 分页器行数限制
    pub pager_limit: usize,
    /// 查询超时（秒）
    pub timeout: Option<u64>,
}

impl Default for ConsoleState {
    fn default() -> Self {
        Self {
            tee_file: None,
            pager_command: None,
            pager_limit: 200,
            timeout: None,
        }
    }
}

impl ConsoleState {
    pub fn new() -> Self {
        Self::default()
    }

    /// 写入输出（同时写入 stdout 和 tee 文件）
    pub fn write_output(&mut self, content: &str) {
        print!("{}", content);
        if let Some(ref mut file) = self.tee_file {
            let _ = file.write_all(content.as_bytes());
        }
    }

    /// 使用分页器显示内容
    pub fn paginate(&self, content: &str, row_count: usize) -> bool {
        if let Some(ref pager) = self.pager_command {
            if row_count > self.pager_limit {
                if let Ok(mut child) = Command::new(pager)
                    .stdin(Stdio::piped())
                    .spawn()
                {
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = stdin.write_all(content.as_bytes());
                    }
                    let _ = child.wait();
                    return true;
                }
            }
        }
        false
    }
}

/// 解析并执行控制台命令
pub fn execute_console_command(input: &str, state: &mut ConsoleState, graph: &Arc<Graph>) -> CommandResult {
    let input = input.trim();
    
    // 移除开头的冒号
    let cmd_line = if input.starts_with(':') {
        &input[1..]
    } else {
        input
    };

    let parts: Vec<&str> = cmd_line.splitn(2, ' ').collect();
    let cmd = parts[0].to_lowercase();
    let args = parts.get(1).copied().unwrap_or("");

    match cmd.as_str() {
        "help" | "h" => CommandResult::Message(get_help_text()),
        
        "quit" | "q" | "exit" | "e" => CommandResult::Exit,
        
        "sleep" => {
            if let Ok(secs) = args.parse::<u64>() {
                thread::sleep(Duration::from_secs(secs));
                CommandResult::Message(format!("Slept for {} seconds", secs))
            } else {
                CommandResult::Error("Usage: :sleep <seconds>".to_string())
            }
        }
        
        "tee" => {
            let args_parts: Vec<&str> = args.split_whitespace().collect();
            let (overwrite, filename) = if args_parts.first() == Some(&"-o") {
                (true, args_parts.get(1).copied())
            } else {
                (false, args_parts.first().copied())
            };

            if let Some(filename) = filename {
                let path = PathBuf::from(filename);
                let file = if overwrite {
                    File::create(&path)
                } else {
                    File::options().create(true).append(true).open(&path)
                };

                match file {
                    Ok(f) => {
                        state.tee_file = Some(f);
                        CommandResult::Message(format!("Logging to {}", filename))
                    }
                    Err(e) => CommandResult::Error(format!("Cannot open file: {}", e)),
                }
            } else {
                CommandResult::Error("Usage: :tee [-o] <filename>".to_string())
            }
        }
        
        "notee" => {
            if let Some(file) = state.tee_file.take() {
                drop(file);
                CommandResult::Message("Stopped logging".to_string())
            } else {
                CommandResult::Message("No active logging".to_string())
            }
        }
        
        "pager" => {
            let args_parts: Vec<&str> = args.split_whitespace().collect();
            if args_parts.len() >= 2 {
                let cmd = args_parts[0].to_string();
                if let Ok(limit) = args_parts[1].parse::<usize>() {
                    state.pager_command = Some(cmd.clone());
                    state.pager_limit = limit;
                    CommandResult::Message(format!("Pager set to {} with row limit {}", cmd, limit))
                } else {
                    CommandResult::Error("Usage: :pager <command> <row_limit>".to_string())
                }
            } else {
                CommandResult::Error("Usage: :pager <command> <row_limit>".to_string())
            }
        }
        
        "nopager" => {
            state.pager_command = None;
            CommandResult::Message("Pager disabled".to_string())
        }
        
        "timeout" => {
            if args.is_empty() || args == "0" {
                state.timeout = None;
                CommandResult::Message("Timeout disabled".to_string())
            } else if let Ok(secs) = args.parse::<u64>() {
                state.timeout = Some(secs);
                CommandResult::Message(format!("Timeout set to {} seconds", secs))
            } else {
                CommandResult::Error("Usage: :timeout <seconds>".to_string())
            }
        }
        
        "clear" => {
            print!("\x1B[2J\x1B[1;1H");
            CommandResult::Continue
        }

        "flush" | "save" => {
            match graph.flush() {
                Ok(_) => CommandResult::Message("数据已保存到磁盘".to_string()),
                Err(e) => CommandResult::Error(format!("保存失败: {}", e)),
            }
        }

        _ => CommandResult::Error(format!("Unknown command: {}. Type :help for help.", cmd)),
    }
}

/// 检查输入是否是控制台命令
pub fn is_console_command(input: &str) -> bool {
    input.trim().starts_with(':')
}

fn get_help_text() -> String {
    r#"
╔═══════════════════════════════════════════════════════════════╗
║                    Console Commands                           ║
╠═══════════════════════════════════════════════════════════════╣
║ :help, :h                  Show this help                     ║
║ :quit, :q, :exit, :e       Exit the program                   ║
║ :sleep <N>                 Sleep for N seconds                ║
║ :tee [-o] <filename>       Log output to file (-o: overwrite) ║
║ :notee                     Stop logging to file               ║
║ :pager <cmd> <limit>       Set pager (e.g., :pager less 100)  ║
║ :nopager                   Disable pager                      ║
║ :timeout <seconds>         Set query timeout (0 to disable)   ║
║ :flush, :save              Flush data to disk                  ║
║ :clear                     Clear the screen                   ║
╠═══════════════════════════════════════════════════════════════╣
║ Tip: Use \G at end of query for vertical result display       ║
╚═══════════════════════════════════════════════════════════════╝
"#.to_string()
}
