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
}

pub struct SystemMonitor {
    system: System,
}

impl SystemMonitor {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        Self { system }
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

        SystemInfo {
            cpu_usage,
            memory_total,
            memory_used,
            memory_usage,
            disk_total,
            disk_used,
            disk_usage,
            uptime,
        }
    }
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}
