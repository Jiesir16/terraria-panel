use serde::Serialize;
use sysinfo::{Disks, System};

#[derive(Debug, Serialize, Clone)]
pub struct SystemInfo {
    pub cpu_usage: f32,
    pub memory_total: u64,
    pub memory_used: u64,
    pub memory_usage: f32,
    pub disk_total: u64,
    pub disk_used: u64,
    pub disk_usage: f32,
    pub uptime: u64,
    pub hostname: Option<String>,
    pub os_version: Option<String>,
    pub os_name: Option<String>,
    pub cpu_count: usize,
    pub dotnet_version: Option<String>,
    pub mono_version: Option<String>,
}

pub struct SystemMonitor {
    system: System,
    dotnet_version: Option<String>,
    mono_version: Option<String>,
}

impl SystemMonitor {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        let dotnet_version = std::process::Command::new("dotnet")
            .arg("--info")
            .output()
            .ok()
            .and_then(|output| {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let line = line.trim();
                    if line.starts_with("Version:") {
                        return Some(line.trim_start_matches("Version:").trim().to_string());
                    }
                }
                None
            });

        let mono_version = std::process::Command::new("mono")
            .arg("--version")
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    stdout.lines().next().map(|s| s.trim().to_string())
                } else {
                    None
                }
            });

        Self {
            system,
            dotnet_version,
            mono_version,
        }
    }

    pub fn get_system_info(&mut self) -> SystemInfo {
        self.system.refresh_all();

        let cpu_usage = self.system.global_cpu_info().cpu_usage();

        let memory_total = self.system.total_memory();
        let memory_used = self.system.used_memory();
        let memory_usage = if memory_total > 0 {
            (memory_used as f32 / memory_total as f32) * 100.0
        } else {
            0.0
        };

        let disks = Disks::new_with_refreshed_list();
        let mut disk_total: u64 = 0;
        let mut disk_available: u64 = 0;
        for disk in disks.list() {
            disk_total += disk.total_space();
            disk_available += disk.available_space();
        }
        let disk_used = disk_total.saturating_sub(disk_available);

        let disk_usage = if disk_total > 0 {
            (disk_used as f32 / disk_total as f32) * 100.0
        } else {
            0.0
        };

        let uptime = System::uptime();
        let hostname = System::host_name();
        let os_version = System::os_version();
        let os_name = System::name();
        let cpu_count = self.system.cpus().len();

        SystemInfo {
            cpu_usage,
            memory_total,
            memory_used,
            memory_usage,
            disk_total,
            disk_used,
            disk_usage,
            uptime,
            hostname,
            os_version,
            os_name,
            cpu_count,
            dotnet_version: self.dotnet_version.clone(),
            mono_version: self.mono_version.clone(),
        }
    }
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}
