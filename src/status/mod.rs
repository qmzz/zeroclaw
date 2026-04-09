//! Status reporting module (P0-2 enhancement)
//! 
//! Provides enhanced status reporting with multiple output formats and watch mode.

use crate::config::Config;
use crate::memory;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Status report structure
#[derive(Debug, Clone, serde::Serialize)]
pub struct StatusReport {
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub workspace: String,
    pub config_path: String,
    pub system: SystemStatus,
    pub provider: ProviderStatus,
    pub memory: MemoryStatus,
    pub channels: ChannelStatus,
    pub resources: ResourceStatus,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SystemStatus {
    pub uptime: String,
    pub service_running: bool,
    pub observability: String,
    pub autonomy_level: String,
    pub runtime: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProviderStatus {
    pub default_provider: String,
    pub default_model: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MemoryStatus {
    pub backend: String,
    pub auto_save: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChannelStatus {
    pub cli: bool,
    pub configured_channels: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ResourceStatus {
    pub memory_usage_mb: Option<u64>,
    pub disk_available_gb: Option<f64>,
}

/// Run status report
pub fn run_status(config: &Config, output_format: &str, output_file: Option<&str>) -> Result<()> {
    let report = generate_status_report(config);
    
    let output = match output_format {
        "json" => {
            serde_json::to_string_pretty(&report)?
        }
        "brief" => {
            format_brief_status(&report)
        }
        _ => {
            format_human_status(&report)
        }
    };
    
    // Output to file or stdout
    if let Some(file_path) = output_file {
        std::fs::write(file_path, &output)?;
        eprintln!("Status report written to: {}", file_path);
    } else {
        println!("{}", output);
    }
    
    Ok(())
}

/// Run watch mode
pub async fn run_watch(
    config: &Config, 
    output_format: &str, 
    interval: u64,
    output_file: Option<&str>,
) -> Result<()> {
    use tokio::time::{sleep, Duration};
    
    println!("📊 ZeroClaw Status Watch (interval: {}s)", interval);
    println!("Press Ctrl+C to stop\n");
    
    loop {
        // Clear screen (ANSI escape code)
        print!("\x1b[2J\x1b[1;1H");
        
        let report = generate_status_report(config);
        
        let output = match output_format {
            "json" => serde_json::to_string_pretty(&report)?,
            "brief" => format_brief_status(&report),
            _ => format_human_status(&report),
        };
        
        println!("{}", output);
        
        // Output to file if specified
        if let Some(file_path) = output_file {
            std::fs::write(file_path, &output).ok();
        }
        
        sleep(Duration::from_secs(interval)).await;
    }
}

/// Generate status report from config
fn generate_status_report(config: &Config) -> StatusReport {
    StatusReport {
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        workspace: config.workspace_dir.display().to_string(),
        config_path: config.config_path.display().to_string(),
        system: SystemStatus {
            uptime: get_uptime(),
            service_running: crate::service::is_running(),
            observability: config.observability.backend.clone(),
            autonomy_level: format!("{:?}", config.autonomy.level),
            runtime: config.runtime.kind.clone(),
        },
        provider: ProviderStatus {
            default_provider: config.default_provider.clone().unwrap_or_else(|| "openrouter".into()),
            default_model: config.default_model.clone().unwrap_or_else(|| "(default)".into()),
        },
        memory: MemoryStatus {
            backend: memory::effective_memory_backend_name(
                &config.memory.backend,
                Some(&config.storage.provider.config),
            ),
            auto_save: config.memory.auto_save,
        },
        channels: ChannelStatus {
            cli: true,
            configured_channels: config.channels_config.channels()
                .into_iter()
                .filter(|(_, configured)| *configured)
                .map(|(channel, _)| channel.name().to_string())
                .collect(),
        },
        resources: ResourceStatus {
            memory_usage_mb: get_memory_usage(),
            disk_available_gb: get_disk_available(),
        },
    }
}

/// Format status as human-readable text
fn format_human_status(report: &StatusReport) -> String {
    let mut output = String::new();
    
    output.push_str("🦀 ZeroClaw Status\n\n");
    output.push_str(&format!("Version:     {}\n", report.version));
    output.push_str(&format!("Workspace:   {}\n", report.workspace));
    output.push_str(&format!("Config:      {}\n", report.config_path));
    output.push_str(&format!("Timestamp:   {}\n", report.timestamp.format("%Y-%m-%d %H:%M:%S UTC")));
    output.push('\n');
    
    output.push_str(&format!("🤖 Provider:      {}\n", report.provider.default_provider));
    output.push_str(&format!("   Model:         {}\n", report.provider.default_model));
    output.push_str(&format!("📊 Observability:  {}\n", report.system.observability));
    output.push_str(&format!("🛡️  Autonomy:      {}\n", report.system.autonomy_level));
    output.push_str(&format!("⚙️  Runtime:       {}\n", report.system.runtime));
    output.push_str(&format!(
        "🟢 Service:       {}\n",
        if report.system.service_running { "running" } else { "stopped" }
    ));
    output.push_str(&format!("⏱️  Uptime:        {}\n", report.system.uptime));
    output.push_str(&format!("🧠 Memory:         {} (auto-save: {})\n", 
        report.memory.backend,
        if report.memory.auto_save { "on" } else { "off" }
    ));
    
    output.push('\n');
    output.push_str("Channels:\n");
    output.push_str("  CLI:      ✅ always\n");
    for channel in &report.channels.configured_channels {
        output.push_str(&format!("  {:9} ✅ configured\n", channel));
    }
    
    output.push('\n');
    output.push_str("Resources:\n");
    if let Some(mem) = report.resources.memory_usage_mb {
        output.push_str(&format!("  Memory:   {} MB\n", mem));
    }
    if let Some(disk) = report.resources.disk_available_gb {
        output.push_str(&format!("  Disk:     {:.1} GB available\n", disk));
    }
    
    output
}

/// Format status as brief text
fn format_brief_status(report: &StatusReport) -> String {
    format!(
        "ZeroClaw v{} | {} | Provider: {} | Memory: {} | Service: {}",
        report.version,
        report.timestamp.format("%H:%M:%S"),
        report.provider.default_provider,
        report.memory.backend,
        if report.system.service_running { "✅" } else { "❌" }
    )
}

/// Get system uptime
fn get_uptime() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(uptime_content) = std::fs::read_to_string("/proc/uptime") {
            if let Some(uptime_secs) = uptime_content.split_whitespace().next() {
                if let Ok(secs) = uptime_secs.parse::<u64>() {
                    let days = secs / 86400;
                    let hours = (secs % 86400) / 3600;
                    let mins = (secs % 3600) / 60;
                    
                    if days > 0 {
                        return format!("{}d {}h {}m", days, hours, mins);
                    } else if hours > 0 {
                        return format!("{}h {}m", hours, mins);
                    } else {
                        return format!("{}m {}s", mins, secs % 60);
                    }
                }
            }
        }
    }
    
    "unknown".to_string()
}

/// Get memory usage in MB
fn get_memory_usage() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<u64>() {
                            return Some(kb / 1024);
                        }
                    }
                }
            }
        }
    }
    
    None
}

/// Get disk available space in GB
fn get_disk_available() -> Option<f64> {
    if let Ok(cwd) = std::env::current_dir() {
        #[cfg(unix)]
        {
            use std::ffi::CString;
            use std::os::unix::ffi::OsStrExt;
            
            if let Ok(path_c) = CString::new(cwd.as_os_str().as_bytes()) {
                unsafe {
                    let mut stat: libc::statvfs = std::mem::zeroed();
                    
                    if libc::statvfs(path_c.as_ptr(), &mut stat) == 0 {
                        let available_bytes = stat.f_bavail as u64 * stat.f_frsize as u64;
                        return Some(available_bytes as f64 / 1024.0 / 1024.0 / 1024.0);
                    }
                }
            }
        }
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_config() -> Config {
        let temp_dir = TempDir::new().unwrap();
        Config {
            workspace_dir: temp_dir.path().to_path_buf(),
            config_path: temp_dir.path().join("config.toml"),
            default_provider: Some("anthropic".to_string()),
            default_model: Some("claude-sonnet".to_string()),
            autonomy: crate::autonomy::AutonomyConfig::default(),
            observability: crate::observability::ObservabilityConfig::default(),
            memory: crate::memory::MemoryConfig::default(),
            storage: crate::storage::StorageConfig::default(),
            channels_config: crate::channels::ChannelsConfig::default(),
            runtime: crate::runtime::RuntimeConfig::default(),
            tunnel: crate::tunnel::TunnelConfig::default(),
            skills: crate::skills::SkillsConfig::default(),
            cron: crate::cron::CronConfig::default(),
            security: crate::security::SecurityConfig::default(),
        }
    }

    #[test]
    fn test_generate_status_report() {
        let config = test_config();
        let report = generate_status_report(&config);
        
        assert_eq!(report.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(report.provider.default_provider, "anthropic");
        assert_eq!(report.provider.default_model, "claude-sonnet");
        assert!(report.workspace.contains("zeroclaw"));
    }

    #[test]
    fn test_format_human_status() {
        let config = test_config();
        let report = generate_status_report(&config);
        let output = format_human_status(&report);
        
        assert!(output.contains("ZeroClaw Status"));
        assert!(output.contains("Provider:"));
        assert!(output.contains("Memory:"));
    }

    #[test]
    fn test_format_brief_status() {
        let config = test_config();
        let report = generate_status_report(&config);
        let output = format_brief_status(&report);
        
        assert!(output.contains("ZeroClaw v"));
        assert!(output.contains("Provider:"));
        assert!(output.contains("Memory:"));
        assert!(output.contains("Service:"));
    }

    #[test]
    fn test_get_uptime() {
        let uptime = get_uptime();
        // Should return either a formatted string or "unknown"
        assert!(!uptime.is_empty());
    }

    #[test]
    fn test_get_memory_usage() {
        let mem = get_memory_usage();
        // May be None on non-Linux or Some(value) on Linux
        #[cfg(target_os = "linux")]
        assert!(mem.is_some());
    }

    #[test]
    fn test_get_disk_available() {
        let disk = get_disk_available();
        // May be None or Some(value)
        #[cfg(unix)]
        assert!(disk.is_some());
    }

    #[test]
    fn test_run_status_json() {
        let config = test_config();
        let result = run_status(&config, "json", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_status_brief() {
        let config = test_config();
        let result = run_status(&config, "brief", None);
        assert!(result.is_ok());
    }
}
