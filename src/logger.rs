use chrono::Local;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub message: String,
    pub details: Option<String>,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let level_str = match self {
            LogLevel::Info => "INFO ",
            LogLevel::Warning => "WARN ",
            LogLevel::Error => "ERROR",
        };
        write!(f, "{}", level_str)
    }
}

pub struct Logger {
    log_path: String,
}

pub fn wrap_line(line: &str, max_width: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut start = 0;
    let chars: Vec<char> = line.chars().collect();

    while start < chars.len() {
        let end = (start + max_width).min(chars.len());
        result.push(chars[start..end].iter().collect());
        start = end;
    }

    result
}

impl Logger {
    pub fn new(log_path: &str) -> Self {
        let path = Path::new(log_path);
        if !path.exists() {
            fs::create_dir_all(path).expect("Failed to create log directory");
        }
        
        Logger {
            log_path: log_path.to_string(),
        }
    }
    
    pub fn log(&self, level: LogLevel, message: &str, details: Option<&str>) {
        let now = Local::now();
        let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();
        
        let log_entry = LogEntry {
            timestamp,
            level,
            message: message.to_string(),
            details: details.map(|s| s.to_string()),
        };
        
        // Create log file path
        let date = now.format("%Y-%m-%d").to_string();
        let log_file_path = Path::new(&self.log_path).join(format!("{}.json", date));
        
        // Read existing logs
        let mut logs = if log_file_path.exists() {
            match fs::read_to_string(&log_file_path) {
                Ok(content) => match serde_json::from_str::<Vec<LogEntry>>(&content) {
                    Ok(entries) => entries,
                    Err(_) => Vec::new(),
                },
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        };
        
        // Add new log entry
        logs.push(log_entry);
        
        // Write logs back to file
        if let Ok(logs_json) = serde_json::to_string_pretty(&logs) {
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&log_file_path)
                .expect("Failed to open log file");
                
            file.write_all(logs_json.as_bytes())
                .expect("Failed to write to log file");
        }
    }
    
    pub fn get_logs(&self, date: Option<&str>, level: Option<LogLevel>) -> Vec<LogEntry> {
        let log_date_owned;

    
    let log_date: &str = match date {
        Some(d) => d,
        None => {
            log_date_owned = Local::now().format("%Y-%m-%d").to_string();
            &log_date_owned
        }
    };
        
        let log_file_path = Path::new(&self.log_path).join(format!("{}.json", log_date));
        
        if !log_file_path.exists() {
            return Vec::new();
        }
        
        match fs::read_to_string(&log_file_path) {
            Ok(content) => {
                match serde_json::from_str::<Vec<LogEntry>>(&content) {
                    Ok(entries) => {
                        if let Some(filter_level) = level {
                            entries.into_iter()
                                .filter(|entry| entry.level == filter_level)
                                .collect()
                        } else {
                            entries
                        }
                    },
                    Err(_) => Vec::new(),
                }
            },
            Err(_) => Vec::new(),
        }
    }

    // Helper function to wrap long lines without crates



   pub fn print_logs(&self,entries: Vec<LogEntry>) {
        let width = 90;
        let separator = "─".repeat(width);
        let msg = "Message".to_string().cyan();

        for entry in entries {
            println!("{}", separator);

            if entry.level == LogLevel::Error{
            println!("│ {:<10}│ {:<75}│", entry.level.to_string().red(), entry.timestamp.to_string().red());
            }
            else{
            println!("│ {:<10}│ {:<75}│", entry.level.to_string().blue(), entry.timestamp.to_string().purple());

            }

            println!("{}", separator);

            for (i, line) in wrap_line(&entry.message, 47).into_iter().enumerate() {
                if i == 0 {
                    println!("│ {} : {:<77}│",msg,line);
                } else {
                    println!("│           {:<77}│", line);
                }
            }

            if let Some(detail) = &entry.details {
                for (i, line) in wrap_line(detail, 77).into_iter().enumerate() {
                    if i == 0 {
                        println!("│ Details : {:<77}│", line);
                    } else {
                        println!("│           {:<77}│", line);
                    }
                }
            }

            println!("{}", separator);
            println!();
        }
    
}


}