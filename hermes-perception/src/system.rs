//! 系统感知 - 我的本体感觉

use hermes_core::{ProcessInfo, Result, SystemInfo};
use sysinfo::System;
use tracing::debug;

/// 系统感知器
pub struct SystemSense;

impl SystemSense {
    pub fn new() -> Self {
        Self
    }

    /// 感知系统状态
    pub async fn perceive(&self) -> Result<super::SystemPerception> {
        // 创建一个新的System实例获取当前状态
        let sys = System::new_all();

        let info = SystemInfo {
            hostname: System::host_name().unwrap_or_else(|| "unknown".to_string()),
            os: format!("{} {}", 
                System::name().unwrap_or_default(), 
                System::os_version().unwrap_or_default()),
            arch: System::cpu_arch().unwrap_or_else(|| "unknown".to_string()),
            cpu_count: sys.cpus().len(),
            memory_total: sys.total_memory(),
            memory_available: sys.available_memory(),
            load_average: [
                System::load_average().one,
                System::load_average().five,
                System::load_average().fifteen,
            ],
        };

        let processes: Vec<ProcessInfo> = sys.processes()
            .iter()
            .take(50) // 只取前50个进程
            .map(|(pid, process)| ProcessInfo {
                pid: pid.as_u32(),
                name: process.name().to_string(),
                cmd: process.cmd().to_vec(),
                cpu_percent: process.cpu_usage(),
                memory_mb: process.memory() / 1024,
                status: format!("{:?}", process.status()),
            })
            .collect();

        debug!("系统感知完成: {} CPUs, {} MB 内存", info.cpu_count, info.memory_total / 1024);

        Ok(super::SystemPerception {
            info,
            processes,
        })
    }

    /// 获取系统负载
    pub fn load_average() -> [f64; 3] {
        [
            System::load_average().one,
            System::load_average().five,
            System::load_average().fifteen,
        ]
    }

    /// 获取内存使用情况
    pub fn memory_usage() -> (u64, u64) {
        let sys = System::new_all();
        (sys.total_memory(), sys.used_memory())
    }

    /// 获取进程数量
    pub fn process_count() -> usize {
        let sys = System::new_all();
        sys.processes().len()
    }
}

impl Default for SystemSense {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_system_sense() {
        let sense = SystemSense::new();
        let perception = sense.perceive().await.unwrap();
        
        assert!(!perception.info.hostname.is_empty());
    }
}
