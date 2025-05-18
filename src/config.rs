use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Clone, Deserialize, Serialize)]
pub struct Config {
    pub projects_root: String,
    pub log_path: String,
    pub state_path: String,
}

impl Config {
    pub fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        let config_dir = home.join(".sahakari");
        
        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).expect("Failed to create config directory");
        }
        
        Config {
            projects_root: "/var/www/html".to_string(),
            log_path: config_dir.join("logs").to_string_lossy().to_string(),
            state_path: config_dir.join("state").to_string_lossy().to_string(),
        }
    }
    
    pub fn load() -> Result<Self, io::Error> {
        let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        let config_path = home.join(".sahakari/config.json");
        
        if !config_path.exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Config file not found"));
        }
        
        let config_str = fs::read_to_string(config_path)?;
        serde_json::from_str(&config_str).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
    
    pub fn save(&self) -> Result<(), io::Error> {
        let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        let config_dir = home.join(".sahakari");
        let config_path = config_dir.join("config.json");
        println!("{}",config_path.to_string_lossy());
        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }
        
        // Create log directory if it doesn't exist
        let log_dir = Path::new(&self.log_path);
        if !log_dir.exists() {
            fs::create_dir_all(log_dir)?;
        }
        
        // Create state directory if it doesn't exist
        let state_dir = Path::new(&self.state_path);
        if !state_dir.exists() {
            fs::create_dir_all(state_dir)?;
        }
        
        let config_str = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            
        fs::write(config_path, config_str)
    }
}