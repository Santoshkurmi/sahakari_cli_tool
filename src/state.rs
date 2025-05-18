use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectState {
    pub project_name: String,
    pub failed_step: String,
    pub timestamp: String,
}

pub struct StateManager {
    state_path: String,
}

impl StateManager {
    pub fn new(state_path: &str) -> Self {
        let path = Path::new(state_path);
        if !path.exists() {
            fs::create_dir_all(path).expect("Failed to create state directory");
        }
        
        StateManager {
            state_path: state_path.to_string(),
        }
    }
    
    pub fn save_state(&self, project_name: &str, failed_step: &str) {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        
        let state = ProjectState {
            project_name: project_name.to_string(),
            failed_step: failed_step.to_string(),
            timestamp: now,
        };
        
        let state_file_path = Path::new(&self.state_path).join(format!("{}.json", project_name));
        
        if let Ok(state_json) = serde_json::to_string_pretty(&state) {
            fs::write(&state_file_path, state_json)
                .expect("Failed to write state file");
        }
    }
    
    pub fn load_state(&self, project_name: &str) -> Option<ProjectState> {
        let state_file_path = Path::new(&self.state_path).join(format!("{}.json", project_name));
        
        if !state_file_path.exists() {
            return None;
        }
        
        match fs::read_to_string(&state_file_path) {
            Ok(content) => {
                match serde_json::from_str::<ProjectState>(&content) {
                    Ok(state) => Some(state),
                    Err(_) => None,
                }
            },
            Err(_) => None,
        }
    }
    
    pub fn clear_state(&self, project_name: &str) {
        let state_file_path = Path::new(&self.state_path).join(format!("{}.json", project_name));
        
        if state_file_path.exists() {
            let _ = fs::remove_file(&state_file_path);
        }
    }
}