use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use colored::Colorize;

use crate::errors::CliError;

/// ~/.phantom — root directory for all local engine state.
fn phantom_home() -> Result<PathBuf, CliError> {
    dirs::home_dir()
        .map(|h| h.join(".phantom"))
        .ok_or_else(|| CliError::Setup("could not determine home directory".to_string()))
}

fn pid_path() -> Result<PathBuf, CliError> {
    Ok(phantom_home()?.join("phantom.pid"))
}

fn log_path() -> Result<PathBuf, CliError> {
    Ok(phantom_home()?.join("logs").join("server.log"))
}

/// Tries to find the 'phantom' binary.
fn find_server_binary() -> Result<PathBuf, CliError> {
    // 1. Check current exe directory (useful for installed binaries)
    if let Ok(mut path) = std::env::current_exe() {
        path.pop();
        let bin = path.join("phantom");
        if bin.exists() {
            return Ok(bin);
        }
    }

    // 2. Check workspace target/debug (useful for dev)
    let workspace_bin = PathBuf::from("target/debug/phantom");
    if workspace_bin.exists() {
        return Ok(workspace_bin);
    }

    // 3. Check PATH
    if let Ok(path) = which::which("phantom") {
        return Ok(path);
    }

    Err(CliError::Setup(
        "could not find 'phantom' binary. Is it built? Run 'cargo build'".to_string(),
    ))
}

pub fn run_up(background: bool) -> Result<(), CliError> {
    let pid_file = pid_path()?;
    if pid_file.exists() {
        let pid_str = fs::read_to_string(&pid_file)?;
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            // Check if process is actually alive
            if process_exists(pid) {
                return Err(CliError::Setup(format!(
                    "server already appears to be running (PID {}). Run 'ph down' first.",
                    pid
                )));
            }
        }
    }

    let bin = find_server_binary()?;
    println!("{} {}", "starting server from".dimmed(), bin.display());

    let mut cmd = Command::new(bin);

    if background {
        let log_file = log_path()?;
        if let Some(parent) = log_file.parent() {
            fs::create_dir_all(parent)?;
        }
        let output = fs::File::create(&log_file)?;

        let child = cmd
            .stdout(Stdio::from(output.try_clone()?))
            .stderr(Stdio::from(output))
            .spawn()
            .map_err(|e| CliError::Setup(format!("failed to spawn server: {}", e)))?;

        let pid = child.id();
        fs::write(&pid_file, pid.to_string())?;

        println!(
            "{} server is running in background (PID {})",
            "success:".green().bold(),
            pid
        );
        println!("{} tail logs with 'ph logs --follow'", "info:".blue());
    } else {
        // Foreground execution
        let mut child = cmd
            .spawn()
            .map_err(|e| CliError::Setup(format!("failed to spawn server: {}", e)))?;

        let _ = child.wait();
    }

    Ok(())
}

pub fn run_down() -> Result<(), CliError> {
    let pid_file = pid_path()?;
    if !pid_file.exists() {
        println!("{}", "server is not running (no PID file found)".yellow());
        return Ok(());
    }

    let pid_str = fs::read_to_string(&pid_file)?;
    let pid = pid_str
        .trim()
        .parse::<i32>()
        .map_err(|_| CliError::Setup("invalid PID file content".to_string()))?;

    println!("{} stopping server (PID {})...", "info:".blue(), pid);

    if terminate_process(pid) {
        let _ = fs::remove_file(pid_file);
        println!("{}", "success: server stopped".green().bold());
    } else {
        println!("{}", "error: could not stop process".red().bold());
    }

    Ok(())
}

pub fn run_logs(follow: bool) -> Result<(), CliError> {
    let log_file = log_path()?;
    if !log_file.exists() {
        return Err(CliError::Setup("log file not found".to_string()));
    }

    let file = fs::File::open(&log_file)?;
    let mut reader = BufReader::new(file);

    if follow {
        loop {
            let mut line = String::new();
            let len = reader.read_line(&mut line)?;
            if len == 0 {
                // Wait for more data
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            print!("{}", line);
        }
    } else {
        for line in reader.lines() {
            println!("{}", line?);
        }
    }

    Ok(())
}

#[cfg(unix)]
fn process_exists(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

#[cfg(unix)]
fn terminate_process(pid: i32) -> bool {
    unsafe { libc::kill(pid, libc::SIGTERM) == 0 }
}

#[cfg(not(unix))]
fn process_exists(_pid: i32) -> bool {
    false // TODO: Windows support if needed
}

#[cfg(not(unix))]
fn terminate_process(_pid: i32) -> bool {
    false // TODO: Windows support if needed
}
