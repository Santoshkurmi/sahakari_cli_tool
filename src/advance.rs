use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

use std::{
    path::{ PathBuf},
    time::{SystemTime, UNIX_EPOCH, Duration},
};


use colored::*;

use crate::{git::get_git_credentials, project::LaravelProject};

/// Run a command quietly, but show output if error
fn run_cmd(dir: &Path, cmd: &str, args: &[&str]) -> Result<(), String> {
    // println!("{}", format!("→ Running `{}`", [cmd, &args.join(" ")].join(" ")).blue());
    let output = Command::new(cmd)
        .args(args)
        .env("NODE_OPTIONS", "--max-old-space-size=1024")
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!("Command `{}` failed:\n{}\n{}", cmd, stdout, stderr));
    }

    Ok(())
}

use urlencoding::encode;

/// Run a git command with temporary credential injection
fn run_git_with_auth(
    repo_path: &str,
    args: &[&str],
) -> Result<(), String> {
    let creds = get_git_credentials();

    // Get original URL
    let output = Command::new("git")
        .args(&["config", "--get", "remote.origin.url"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to get remote URL: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Git config error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let original_url = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Inject creds
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
        .map_err(|e| format!("Failed to set temp remote URL: {}", e))?;

    // Run actual git command
    let result = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Git command failed: {}", e));

    // Restore URL
    Command::new("git")
        .args(&["remote", "set-url", "origin", &original_url])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to restore remote URL: {}", e))?;

    // Return error if git itself failed
    match result {
        Ok(res) if res.status.success() => Ok(()),
        Ok(res) => Err(String::from_utf8_lossy(&res.stderr).to_string()),
        Err(e) => Err(e),
    }
}


/// Get latest commit hash where `resources/js` changed
fn latest_js_commit(project: &Path) -> Result<String, String> {
    // find current branch name
    let branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(project)
        .output()
        .map_err(|e| e.to_string())?;
    if !branch.status.success() {
        return Err("Failed to get branch".into());
    }
    let branch_name = String::from_utf8_lossy(&branch.stdout).trim().to_string();

    // now check origin/<branch>
    let output = Command::new("git")
        .args([
            "log",
            "-n1",
            "--pretty=format:%H",
            &format!("origin/{}", branch_name),
            "--",
            "resources/js",
        ])
        .current_dir(project)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err("Failed to get git log".into());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn remove_any(path: &Path) {
    if path.exists() {
        if path.is_file() || path.is_symlink() {
            let _ = fs::remove_file(path);
        } else if path.is_dir() {
            let _ = fs::remove_dir_all(path);
        }
    }
}
/// Main workflow
pub fn ensure_js_build(project: &Path, parent: &Path) -> Result<(), String> {
    // println!("{}", "⚡ Starting JS build check".yellow().bold());

    let commit_hash = latest_js_commit(project)?;
    // println!("{} {}", "Latest JS commit:".green(), commit_hash);

    let builds_path = parent.join(".builds").join(&commit_hash);
    let project_build_link = project.join("public/build");


    if builds_path.exists() && builds_path.read_dir().map_err(|e| e.to_string())?.next().is_some() {
        // if project_build_link.exists() {
        //     fs::remove_file(&project_build_link).ok();
        // }
        remove_any(&project_build_link);
        
        std::os::unix::fs::symlink(&builds_path, &project_build_link)
            .map_err(|e| e.to_string())?;
        println!("{}", "✅ Build already exists and has files, linked successfully".green());
        return Ok(());
    }

    println!("{}", "⚠️  Build not found in parent, preparing build...".red());

    // Make sure commit exists in parent
    let check_commit = Command::new("git")
        .args(["cat-file", "-t", &commit_hash])
        .current_dir(parent)
        .output()
        .map_err(|e| e.to_string())?;

    if !check_commit.status.success() {
        println!("{}", "⏬ Commit not found in parent, fetching...".blue());
        run_git_with_auth(parent.to_str().unwrap(), &["fetch", "--all"])?;
    }

    // Checkout commit
    // println!("{} {}", "🔀 Checking out commit".blue(), commit_hash);
    run_cmd(parent, "git", &["checkout", &commit_hash])?;

    // Install & build
    println!("{}", "📦 Installing dependencies (pnpm install)...".yellow());
    if let Err(e) = run_cmd(parent, "pnpm", &["install"]) {
        return Err(format!("pnpm install failed: {}", e));
    }

    println!("{}", "🏗️  Building project...".yellow());
    if let Err(e) = run_cmd(parent, "pnpm", &["vite", "build","--outDir",".dist"]) {
        println!("{}", "❌ Build failed!".red());
        return Err(e);
    }

    // Move build output
    let tmp_build = parent.join(".dist"); // vite default output
    if tmp_build.exists() {
        // println!("{}", "📂 Moving build to builds/{hash}".yellow());
        fs::create_dir_all(&builds_path).ok();
        fs::rename(&tmp_build, &builds_path).map_err(|e| e.to_string())?;
    } else {
        return Err("No .dist/ directory produced by build".into());
    }

    // Link into project
    if project_build_link.exists() {
        fs::remove_file(&project_build_link).ok();
    }
    std::os::unix::fs::symlink(&builds_path, &project_build_link)
        .map_err(|e| e.to_string())?;

    println!("{}", "✅ Build linked successfully".green());
    Ok(())
}






pub fn cleanup_unused_parent_builds(projects: &[LaravelProject], parent: &Path) -> Result<(), String> {
    // --- Only run once per day ---
    let parent_builds = parent.join(".builds");
    let lock_file = parent_builds.join(".cleanup_last_run");
    if let Ok(metadata) = fs::metadata(&lock_file) {
        if let Ok(mod_time) = metadata.modified() {
            let now = SystemTime::now();
            if let Ok(elapsed) = now.duration_since(mod_time) {
                if elapsed.as_secs() < 24 * 60 * 60 {
                    println!("{}", "⏱️  Cleanup already ran in the last 24h, skipping.".yellow());
                    return Ok(());
                }
            }
        }
    }

    println!("{}", "🧹 Starting cleanup of unused parent builds...".blue());

    // --- Collect all hashes currently used by projects ---
    let mut used_hashes = Vec::new();
    for project in projects {
        let build_symlink = Path::new(&project.path).join("public/build");
        if build_symlink.exists() {
            if let Ok(target) = fs::read_link(&build_symlink) {
                if let Some(hash_name) = target.file_name().map(|s| s.to_string_lossy().to_string()) {
                    used_hashes.push(hash_name);
                }
            }
        }
    }

    // --- List all folders in parent builds ---
    if !parent_builds.exists() {
        println!("{}", "⚠️ Parent builds folder does not exist, nothing to clean.".yellow());
        return Ok(());
    }

    for entry in fs::read_dir(parent_builds).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            let folder_name = path.file_name().unwrap().to_string_lossy();
            if !used_hashes.contains(&folder_name.to_string()) {
                println!("{}", format!("🗑️ Deleting unused build: {}", folder_name).red());
                fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
            }
        }
    }

    // --- Update the last run timestamp ---
    fs::write(lock_file, b"done").map_err(|e| e.to_string())?;

    println!("{}", "✅ Cleanup completed".green());
    Ok(())
}
