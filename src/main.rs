// Laravel Project Manager CLI Tool
// Main entry point for the application

use clap::{error, Parser, Subcommand};
use colored::*;
use inquire::{Confirm, MultiSelect, Select};
use std::{env, os::unix::thread, path::PathBuf, process::{Command, Output}};
use walkdir::WalkDir;
use sysinfo::{CpuExt, CpuRefreshKind, DiskExt, RefreshKind, System, SystemExt};
mod config;
mod git;
mod logger;
mod project;
mod state;
use std::thread::sleep;
use std::time::Duration;

use config::Config;
use git::GitOperations;
use logger::{Logger, LogLevel};
use project::{LaravelProject, ProjectStatus};
use state::StateManager;
#[derive(Parser)]
#[clap(author, version, about)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]


enum Commands {
    /// Update all or selected Laravel projects
    Update {
        /// Process all projects without prompting
        #[clap(long,short='a')]
        all: bool,
        
        /// Process only specific project(s) Not working
        #[clap(long)]
        only: Option<Vec<String>>,
        
        /// Show all projects with errors to update
        #[clap(long,short='e')]
        errors: bool,
        
        /// Show what would be done without executing
        #[clap(long)]
        dry_run: bool,
        
        /// Show detailed output
        #[clap(long,short='v')]
        verbose: bool,

        /// Force update to run all commands even if no change
        #[clap(long,short='f')]
        force: bool,

       
        /// Update current directory,use dot(.) to update all projects
        #[clap(value_parser)]
        path: Option<String>,
    },

    /// Check system health like cpu, memory, disk usage
    Health{
        
    },
    
    /// Show logs of previous operations
    Logs {
        /// Export logs to file
        #[clap(long)]
        export: Option<String>,
        
        /// Show only errors
        #[clap(long,short='e')]
        errors: bool,
    },
    
    /// Configure the tool
    Config {
        /// Set projects root path
        #[clap(long)]
        root: Option<String>,
        
        /// Show current configuration
        #[clap(long,short='s')]
        show: bool,
    },

}


fn system_details() {
    let mut sys = System::new();

    sys.refresh_cpu();
    sys.refresh_memory();
    sys.refresh_disks_list();
    sys.refresh_disks();

    // Sleep for a moment so usage can be measured
    sleep(Duration::from_millis(1000));

    // Second refresh to get CPU usage diff
    sys.refresh_cpu();
    let mut total_usage = 0.0;


    println!("┌────────────┬────────────────────────────────────────┐");
    println!("│ CPU        │ Usage (%)                              │");
    println!("├────────────┼────────────────────────────────────────┤");
    for (i, cpu) in sys.cpus().iter().enumerate() {
        total_usage += cpu.cpu_usage();
        println!("│ CPU {:<6} │ {:>36.2} % │", i, cpu.cpu_usage());
    }

        println!("│ Avg {:<6} │ {:>36.2} % │", "", total_usage/ sys.cpus().len() as f32);


    println!("└────────────┴────────────────────────────────────────┘\n");
    // println!("\n🧠 Average CPU Usage: {:.2}%", average);

    let total_mem_gb = sys.total_memory() as f64 / (1024.0 * 1024.0*1024.0);
    let used_mem_gb = sys.used_memory() as f64 / (1024.0 * 1024.0*1024.0);
    let mem_percent = (used_mem_gb / total_mem_gb) * 100.0;

    println!("┌────────────┬─────────────────────────────────────────────────────────────┐");
    println!("│ RAM        │                                                             │");
    println!("├────────────┼──────────────────────────────────────────────────────── ────│");
    println!("│ Used       │ {:>6.2} GB / {:<6.2} GB ({:.1}%) │", used_mem_gb, total_mem_gb, mem_percent);
    println!("└────────────┴─────────────────────────────────────────────────────────────\n");

    println!("┌────────────┬──────────────────────────────────────────────┐");
    println!("│ Disk       │ Total (GB)    | Available (GB)               │");
    println!("├────────────┼──────────────────────────────────────────────┤");
    for disk in sys.disks() {
        let total = disk.total_space() as f64 / (1024.0 * 1024.0 * 1024.0);
        let available = disk.available_space() as f64 / (1024.0 * 1024.0 * 1024.0);
        let name = disk.mount_point().to_string_lossy();
        println!(
            "│ {:<10} │ {:>10.2}     | {:>10.2}                  │",
            name, total, available
        );
    }
    println!("└────────────┴──────────────────────────────────────────────┘");
}


#[allow(unused)]
fn main() {
    // Initialize configuration

    
    let config = Config::load().unwrap_or_else(|_| {
        println!("{}", "No configuration found. Creating default configuration.".yellow());
        let default_config = Config::default();
        default_config.save().expect("Failed to save default configuration");
        default_config
    });
    
    // Initialize logger
    let logger = Logger::new(&config.log_path);
    
    // Initialize state manager
    let state_manager = StateManager::new(&config.state_path);
    
    // Parse command line arguments
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Update { all, only, errors, dry_run, verbose,path ,force} => {
            update_projects(&config, &logger, &state_manager, all, only, errors, dry_run, verbose,path,force);
        },
        Commands::Logs { export, errors } => {
            show_logs(&logger, export, errors);
        },
        Commands::Config { root, show } => {
            configure(&config, root, show);
        },
        Commands::Health {  } => {
            system_details();
        },
    }
}

fn update_projects(
    config: &Config,
    logger: &Logger,
    state_manager: &StateManager,
    all: bool,
    only: Option<Vec<String>>,
    errors: bool,
    dry_run: bool,
    verbose: bool,
    path:Option<String>,
    force:bool
) {

  
    logger.log(LogLevel::Info, "Starting Laravel project update process", None);
    Command::new("clear")
        .status() // or use `spawn()` for async
        .expect("Failed to clear terminal");
    let mut projects_to_process = Vec::new();
    let mut total_count = 0;

    let is_current_path = path.as_deref() == Some(".");
    if   !is_current_path{
        // Scan for Laravel projects
        let projects = scan_for_projects(&config.projects_root);
        
        if projects.is_empty() {
            println!("{}", "No Laravel projects found.".red());
            logger.log(LogLevel::Error, "No Laravel projects found", None);
            return;
        }

        
        if !errors{
        println!("{} {} {}", "\nFound".green(), projects.len(), "Laravel projects".green());
        total_count = projects.len();
        }
        
        // Filter projects if 'only' is specified
         projects_to_process = if let Some(project_names) = only {
            projects.into_iter()
                .filter(|p: &LaravelProject| project_names.contains(&p.name))
                .collect::<Vec<_>>()
        } else if !all {

             let projects = if errors{
            projects.iter()
                .filter(|p| state_manager.load_state(&p.name).is_some())
                .cloned()
                .collect::<Vec<_>>()
            }
            else {
                projects.clone()
            };
            // Interactive selection if not 'all'
            let project_names = projects.iter()
                .map(|p| p.name.clone())
                .collect::<Vec<_>>();
        
           
        
        if errors{
            total_count = projects.len();

            println!("{} {} {}", "\nFound".green(), projects.len(), "Laravel projects with errors".red());
        }
                
            let selected = MultiSelect::new("\nSelect projects to update:", project_names)
                .prompt()
                .unwrap_or_else(|_| Vec::new());

            total_count = selected.len() ;

            projects.into_iter()
                .filter(|p| selected.contains(&p.name))
                .collect::<Vec<_>>()

            
        
                
            
        } else {
            projects
        };
        
        if projects_to_process.is_empty() {
            println!("{}", "No projects selected for processing.".yellow());
            logger.log(LogLevel::Info, "No projects selected for processing", None);
            return;
        }
    } //currents
    else{

        let current_path: PathBuf = env::current_dir().expect("Failed to get current directory");

    // Save folder name (last component) to a variable
        let folder_name: String = current_path
            .file_name()
            .map(|os_str| os_str.to_string_lossy().into_owned())
            .unwrap_or_else(|| String::from(""));

        projects_to_process.push(LaravelProject {
                    name:folder_name,
                    path: current_path.to_string_lossy().to_string(),
                    status: ProjectStatus::NotProcessed,
                });
    
    }
    
    // Process each project
    let mut current_count = 1;
    'outer:for  project in projects_to_process {
        println!("\n{}. {} {} {}/{}\n", current_count , "Processing project:".blue(),project.name.green(),current_count.to_string().purple(),total_count.to_string().cyan());
        logger.log(LogLevel::Info, &format!("Processing project: {}", project.name), None);
        current_count = current_count + 1;
        // Check if we should resume from a failed state
        
        
        // Git operations
        if !dry_run {

            loop{


            match GitOperations::pull(&project.path) {
                Ok(changes) => {
                    if changes.is_empty() && !force {
                        println!("  {} {}", "✓".green(), "No changes detected".green());
                        logger.log(LogLevel::Info, &format!("No changes detected in {}", project.name), None);
                        continue 'outer;
                    }
                    
                    println!("  {} {} {}", "✓".green(), "Pulled changes:".green(), changes.len());
                    
                    // Process changes
                    // let has_php_changes = changes.iter().any(|f| f.ends_with(".php"));
                    let has_migration_changes = changes.iter().any(|f| f.contains("migrations"));
                    let has_js_changes = changes.iter().any(|f| f.ends_with(".js") || f.ends_with(".jsx") || f.ends_with(".ts") || f.ends_with(".tsx"));

                    let has_composer_change = changes.iter().any(|f| f.contains("composer.json"));
                    let has_package_change = changes.iter().any(|f| f.ends_with("package.json"));
                    
                    loop{

                    // Run appropriate commands based on changes
                    if has_composer_change {
                        println!("  {} {}", "→".blue(), "Composer.json file changed, running composer install");
                        if verbose {
                            println!("    {}", "Executing: composer install".cyan());
                        }
                        
                        if !dry_run {
                            match run_command(&project.path, "composer", &["install"]) {
                                Ok(_) => {
                                    println!("    {} {}", "✓".green(), "Composer install completed".green());
                                    logger.log(LogLevel::Info, &format!("Composer install completed for {}", project.name), None);
                                },
                                Err(e) => {
                                    println!("    {} {}: {}", "✗".red(), "Composer install failed".red(), e);
                                    logger.log(LogLevel::Error, &format!("Composer install failed for {}: {}", project.name, e), None);
                                    
                                    // Save state for resuming later
                                    state_manager.save_state(&project.name, "composer_install_failed");
                                    
                                    // Ask user what to do
                                    let action = Select::new("What would you like to do?", vec!["Skip this project", "Retry command", "Abort all"])
                                        .prompt()
                                        .unwrap_or("Abort all");
                                        
                                    match action {
                                        "Skip this project" => continue 'outer,
                                        "Retry command" => {
                                            // TODO: Implement retry logic
                                            println!("  {} {}", "→".blue(), "Retrying composer install");
                                            continue;
                                        },
                                        _ => {
                                            println!("{}", "Aborting all operations.".red());
                                            logger.log(LogLevel::Error, "User aborted all operations", None);
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    if has_js_changes || force {
                        println!("  {} {}", "→".blue(), "JS files changed, running npm install && npm run build");
                        if verbose {
                            println!("    {}", "Executing: npm install".cyan());
                        }
                        
                        if !dry_run {

                            if has_package_change ||force{
                            match run_command(&project.path, "npm", &["install"]) {
                                Ok(_) => {
                                    println!("    {} {}", "✓".green(), "npm install completed".green());
                                    logger.log(LogLevel::Info, &format!("npm install completed for {}", project.name), None);
                                    
                                    if verbose {
                                        println!("    {}", "Executing: npm run build".cyan());
                                    }
                                    
                                 
                                },
                                Err(e) => {
                                    println!("    {} {}: {}", "✗".red(), "npm install failed".red(), e);
                                    logger.log(LogLevel::Error, &format!("npm install failed for {}: {}", project.name, e), None);
                                    
                                    // Save state for resuming later
                                    state_manager.save_state(&project.name, "npm_install_failed");
                                    
                                    // Ask user what to do
                                    let action = Select::new("What would you like to do?", vec!["Skip this project", "Retry command", "Abort all"])
                                        .prompt()
                                        .unwrap_or("Abort all");
                                        
                                    match action {
                                        "Skip this project" => continue 'outer,
                                        "Retry command" => {
                                            // TODO: Implement retry logic
                                            println!("  {} {}", "→".blue(), "Retrying npm install");
                                            continue;
                                        },
                                        _ => {
                                            println!("{}", "Aborting all operations.".red());
                                            logger.log(LogLevel::Error, "User aborted all operations", None);
                                            return;
                                        }
                                    }
                                }
                            }//running command

                           

                            }//has package change

                                   match run_command(&project.path, "npm", &["run", "dev"]) {
                                        Ok(_) => {
                                            println!("    {} {}", "✓".green(), "npm run dev completed".green());
                                            logger.log(LogLevel::Info, &format!("npm run build completed for {}", project.name), None);
                                        },
                                        Err(e) => {
                                            println!("    {} {}: {}", "✗".red(), "npm run dev failed".red(), e);
                                            logger.log(LogLevel::Error, &format!("npm run dev failed for {}: {}", project.name, e), None);
                                            
                                            // Save state for resuming later
                                            state_manager.save_state(&project.name, "npm_dev_failed");
                                            
                                            // Ask user what to do
                                            let action = Select::new("What would you like to do?", vec!["Skip this project", "Retry command", "Abort all"])
                                                .prompt()
                                                .unwrap_or("Abort all");
                                                
                                            match action {
                                                "Skip this project" => continue 'outer,
                                                "Retry command" => {
                                                    // TODO: Implement retry logic
                                                    println!("  {} {}", "→".blue(), "Retrying npm run dev");
                                                    continue;
                                                },
                                                _ => {
                                                    println!("{}", "Aborting all operations.".red());
                                                    logger.log(LogLevel::Error, "User aborted all operations", None);
                                                    return;
                                                }
                                            }
                                        }
                                    }

                        }//dry run
                    }
                    
                    // Ask if user wants to run migrations
                    if has_migration_changes || force
                    {
                        println!("  {} {}", "→".blue(), "Running database migrations");
                        if verbose {
                            println!("    {}", "Executing: php artisan migrate".cyan());
                        }
                        
                        if !dry_run {
                            match run_command(&project.path, "php", &["artisan", "migrate","--force"]) {
                                Ok(_) => {
                                    println!("    {} {}", "✓".green(), "Migrations completed".green());
                                    logger.log(LogLevel::Info, &format!("Migrations completed for {}", project.name), None);
                                },
                                Err(e) => {
                                    println!("    {} {}: {}", "✗".red(), "Migrations failed".red(), e);
                                    logger.log(LogLevel::Error, &format!("Migrations failed for {}: {}", project.name, e), None);

                                    let action = Select::new("What would you like to do?", vec!["Skip this project", "Retry command", "Abort all"])
                                                .prompt()
                                                .unwrap_or("Abort all");
                                                
                                            match action {
                                                "Skip this project" => continue 'outer,
                                                "Retry command" => {
                                                    // TODO: Implement retry logic
                                                    println!("  {} {}", "→".blue(), "Retrying php artisan migrate");
                                                    continue;
                                                },
                                                _ => {
                                                    println!("{}", "Aborting all operations.".red());
                                                    logger.log(LogLevel::Error, "User aborted all operations", None);
                                                    return;
                                                }
                                            }
                                }
                            }
                        }
                    }
                    
                    // Ask if user wants to optimize
                    // if Confirm::new("Run artisan optimize?")
                    //     .with_default(true)
                    //     .prompt()
                    //     .unwrap_or(true) 
                    // {
                    //     println!("  {} {}", "→".blue(), "Running artisan optimize");
                    //     if verbose {
                    //         println!("    {}", "Executing: php artisan optimize".cyan());
                    //     }
                        
                    //     if !dry_run {
                    //         match run_command(&project.path, "php", &["artisan", "optimize"]) {
                    //             Ok(_) => {
                    //                 println!("    {} {}", "✓".green(), "Optimization completed".green());
                    //                 logger.log(LogLevel::Info, &format!("Optimization completed for {}", project.name), None);
                    //             },
                    //             Err(e) => {
                    //                 println!("    {} {}: {}", "✗".red(), "Optimization failed".red(), e);
                    //                 logger.log(LogLevel::Error, &format!("Optimization failed for {}: {}", project.name, e), None);
                    //             }
                    //         }
                    //     }
                    // }
                    break;
                    }//loop
               
                },
                Err(e) => {
                    println!("  {} {}: {}", "✗".red(), "Git pull failed".red(), e);
                    logger.log(LogLevel::Error, &format!("Git pull failed for {}: {}", project.name, e), None);
                    
                    // Save state for resuming later
                    state_manager.save_state(&project.name, "git_pull_failed");
                    
                    // Ask user what to do
                    let action = Select::new("What would you like to do?", vec!["Skip this project", "Retry command", "Abort all"])
                        .prompt()
                        .unwrap_or("Abort all");
                        
                    match action {
                        "Skip this project" => continue 'outer,
                        "Retry command" => {
                            // TODO: Implement retry logic
                            println!("  {} {}", "→".blue(), "Retrying git pull");
                            continue;
                        },
                        _ => {
                            println!("{}", "Aborting all operations.".red());
                            logger.log(LogLevel::Error, "User aborted all operations", None);
                            return;
                        }
                    }
                }
            }
           break;
        
            }//git outer
        } else {
            println!("  {} {}", "→".yellow(), "Dry run: would pull git changes");
        }


        
        // Clear state after successful completion
        state_manager.clear_state(&project.name);
    }
    
    println!("\n{}", "All projects processed successfully.".green());
    logger.log(LogLevel::Info, "All projects processed successfully", None);
}

fn scan_for_projects(root_path: &str) -> Vec<LaravelProject> {
    let mut projects = Vec::new();
    
    for entry in WalkDir::new(root_path)
        .max_depth(2)  // Only scan immediate subdirectories
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        
        // Check if this is a directory
        if path.is_dir() {
            // Check if artisan file exists (Laravel project indicator)
            let artisan_path = path.join("artisan");
            let dot_git = path.join(".git");
           
            if artisan_path.exists() && dot_git.is_dir() {
                // This is a Laravel project
                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                    
                projects.push(LaravelProject {
                    name,
                    path: path.to_string_lossy().to_string(),
                    status: ProjectStatus::NotProcessed,
                });
            }
        }
    }
    
    projects
}

fn run_command(working_dir: &str, command: &str, args: &[&str]) -> Result<Output, String> {
     let output = Command::new(command)
        .args(args)
        .current_dir(working_dir)
        .output()
        .map_err(|e| format!("Failed to launch command: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Command failed:\n{}", stderr));
    }

    Ok(output)

}

fn show_logs(logger: &Logger, export: Option<String>, errors_only: bool) {
    // TODO: Implement log viewing functionality
    println!("Showing logs...");

    let entries = logger.get_logs(None,if errors_only { Option::Some((LogLevel::Error)) } else {None});
    logger.print_logs(entries);
    
}

fn configure(config: &Config, root: Option<String>, show: bool) {
    if show {
        println!("Current configuration:");
        println!("  Projects root: {}", config.projects_root);
        println!("  Log path: {}", config.log_path);
        println!("  State path: {}", config.state_path);
        return;
    }
    
    if let Some(new_root) = root {
        let mut updated_config = config.clone();
        updated_config.projects_root = new_root;
        
        match updated_config.save() {
            Ok(_) => println!("{}", "Configuration updated successfully.".green()),
            Err(e) => println!("{} {}", "Failed to update configuration:".red(), e),
        }
    }
}