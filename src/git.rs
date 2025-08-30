use std::io::{self, Write};
use std::process::Command;
use std::path::Path;
use urlencoding::encode;
use once_cell::sync::OnceCell;
pub struct GitOperations;

#[derive(Debug, Clone)]
struct GitCredentials {
    username: String,
    password: String,
}

static CACHED_CREDENTIALS: OnceCell<GitCredentials> = OnceCell::new();

fn get_git_credentials() -> GitCredentials {
    if let Some(cached) = CACHED_CREDENTIALS.get() {
        return cached.clone();
    }
    //
    // print!("Git Username: ");
    // io::stdout().flush().unwrap();
    // let mut username = String::new();
    // io::stdin().read_line(&mut username).unwrap();
    //
    // print!("Git Password: ");
    // io::stdout().flush().unwrap();
    // let mut password = String::new();
    // io::stdin().read_line(&mut password).unwrap();
    //
    let creds = GitCredentials {
        username: "brightsoftware.backup@gmail.com".to_string(),
        password: "brightreadonlybot".to_string(),
    };

    // Store once
    CACHED_CREDENTIALS.set(creds.clone()).ok();
    creds
}

impl GitOperations {

    


pub fn pull(repo_path: &str) -> Result<Vec<String>, String> {
   

    let creds = get_git_credentials();

    // Step 1: Auto-commit any local changes (empty message, optional)
    let _ = Command::new("git")
        .args(&["commit", "-am", "auto-commit before pull"])
        .current_dir(repo_path)
        .output(); // Ignored errors — it might just mean "nothing to commit"

    // Step 2: Get HEAD before pull
    let before = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to get HEAD before pull: {}", e))?;

    if !before.status.success() {
        let err = String::from_utf8_lossy(&before.stderr);
        return Err(format!("Failed to get HEAD before pull: {}", err));
    }

    let before_hash = String::from_utf8_lossy(&before.stdout).trim().to_string();


    //get branch name to pull that git rev-parse --abbrev-ref HEAD
    let branch = Command::new("git")
            .args(&["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(repo_path)
            .output()
            .map_err(|e| format!("Git pull failed: {}", e))?;

        if !branch.status.success() {
            let err = String::from_utf8_lossy(&branch.stderr);
            return Err(format!("Git get branch failed: {}", err));
        }

    let branch_name = String::from_utf8_lossy(&branch.stdout).trim().to_string();


    /*
    |--------------------------------------------------------------------------
    | Set password and username to make sure we dont have to login again and again
    |--------------------------------------------------------------------------
    |
    */
       // Step 1: Get original remote URL
    let output = Command::new("git")
        .args(&["config", "--get", "remote.origin.url"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to get remote URL: {}", e))?;

    if !output.status.success() {
        return Err(format!("Git config error: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let original_url = String::from_utf8_lossy(&output.stdout).trim().to_string();

   
    // Step 3: Encode credentials
    let encoded_user = encode(&creds.username);
    let encoded_pass = encode(&creds.password);


    let temp_url = original_url.replace(
            "https://",
            &format!("https://{}:{}@", encoded_user, encoded_pass),
        );


    Command::new("git")
        .args(&["remote", "set-url", "origin", &temp_url])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to set remote URL: {}", e))?;

    // Step 3: git pull
    let pull = Command::new("git")
        .args(&["pull","origin",&branch_name])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Git pull failed: {}", e))?;

    Command::new("git")
        .args(&["remote", "set-url", "origin", &original_url])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to reset remote URL: {}", e))?;


    if !pull.status.success() {
        let err = String::from_utf8_lossy(&pull.stderr);
        return Err(format!("Git pull failed: {}", err));
    }

    

    // Step 4: Sometimes after pull Git asks for a merge commit
    let _ = Command::new("git")
        .args(&["commit", "-am", "merged"])
        .current_dir(repo_path)
        .output(); // Ignore failure — no merge to commit is okay

    // Step 5: Get HEAD after pull
    let after = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to get HEAD after pull: {}", e))?;

    if !after.status.success() {
        let err = String::from_utf8_lossy(&after.stderr);
        return Err(format!("Failed to get HEAD after pull: {}", err));
    }

    let after_hash = String::from_utf8_lossy(&after.stdout).trim().to_string();

    // Step 6: git diff to get changed files
    let diff = Command::new("git")
        .args(&["diff", "--name-only", &before_hash, &after_hash])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Git diff failed: {}", e))?;

    if !diff.status.success() {
        let err = String::from_utf8_lossy(&diff.stderr);
        return Err(format!("Git diff failed: {}", err));
    }

    let changed_files = String::from_utf8_lossy(&diff.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect();

    Ok(changed_files)
}

}
