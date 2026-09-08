use crate::models::{ProcessInfo, SystemStats};
use std::collections::HashMap;
use std::process::Command;
use sysinfo::{Components, Disks, System};

/// Holds a long-lived sysinfo::System. This matters: CPU (and per-process
/// CPU) usage is only meaningful as a delta between two samples taken some
/// time apart. Creating a fresh System on every call -- what the first
/// version of this module did -- always reports ~0%, since there is no
/// prior sample to diff against. Keeping this struct in Tauri-managed
/// state and refreshing the same instance on each poll fixes that.
pub struct SystemMonitor {
    sys: System,
    components: Components,
}

impl SystemMonitor {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        // Seed a real baseline immediately: sysinfo needs two samples with
        // a gap to compute a meaningful CPU delta. Without this, the very
        // first snapshot() call after startup reports an inflated/garbage
        // number (this was the "still ungenau" symptom) because it's
        // diffing against the single refresh_all() sample above, which
        // itself has no prior sample to diff against.
        std::thread::sleep(std::time::Duration::from_millis(250));
        sys.refresh_cpu_usage();

        Self {
            sys,
            components: Components::new_with_refreshed_list(),
        }
    }

    pub fn snapshot(&mut self) -> SystemStats {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.sys
            .refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        self.components.refresh();

        let cpu_percent = self.sys.global_cpu_usage();
        let ram_percent = if self.sys.total_memory() > 0 {
            (self.sys.used_memory() as f32 / self.sys.total_memory() as f32) * 100.0
        } else {
            0.0
        };

        let disks = Disks::new_with_refreshed_list();
        let disk_free_gb =
            disks.iter().map(|d| d.available_space()).sum::<u64>() as f32 / 1_000_000_000.0;

        // Raw sysinfo labels are things like "gigabyte_wmi temp3",
        // "k10temp Tctl", "amdgpu edge" -- meaningless to a non-technical
        // reader. Group them into the handful of categories people
        // actually care about, keeping the hottest reading per category.
        let mut temps_c: HashMap<String, f32> = HashMap::new();
        for c in self.components.iter() {
            let category = categorize_temp_label(c.label());
            let reading = c.temperature();
            temps_c
                .entry(category.to_string())
                .and_modify(|existing| {
                    if reading > *existing {
                        *existing = reading;
                    }
                })
                .or_insert(reading);
        }

        // "Wie sehr ist das gesamte System ausgelastet" -- the 1-minute
        // unix load average, expressed relative to core count so it reads
        // as a familiar 0-100%+ figure instead of raw load-average units.
        let cpu_count = self.sys.cpus().len().max(1) as f64;
        let load = System::load_average();
        let system_load_percent = ((load.one / cpu_count) * 100.0) as f32;

        let gpu_percent = read_gpu_usage();

        // Battery: sysinfo dropped battery support; wire up
        // `starship-battery` or platform APIs here if the target machine
        // is a laptop.
        let battery_percent: Option<f32> = None;

        // Chromium-based browsers (and some other multi-process apps)
        // spawn one OS process per tab/window, all sharing the same name
        // ("brave", "chromium", ...). Listing each individually both
        // clutters the UI and produced non-unique keys in the frontend's
        // keyed #each, which crashed the whole app's reactivity. Group by
        // name and sum instead -- also just more useful info.
        let mut grouped: HashMap<String, ProcessInfo> = HashMap::new();
        for p in self.sys.processes().values() {
            let name = p.name().to_string_lossy().to_string();
            let entry = grouped.entry(name.clone()).or_insert(ProcessInfo {
                name,
                cpu_percent: 0.0,
                memory_mb: 0.0,
            });
            entry.cpu_percent += p.cpu_usage() / cpu_count as f32;
            entry.memory_mb += p.memory() as f32 / 1_000_000.0;
        }
        let mut top_processes: Vec<ProcessInfo> = grouped.into_values().collect();
        top_processes.sort_by(|a, b| b.cpu_percent.total_cmp(&a.cpu_percent));
        top_processes.truncate(6);

        SystemStats {
            cpu_percent,
            ram_percent,
            gpu_percent,
            system_load_percent,
            disk_free_gb,
            battery_percent,
            temps_c,
            top_processes,
        }
    }
}

/// Maps a raw sysinfo/lm-sensors component label to a human category.
/// Falls back to "Sonstige" for anything unrecognized rather than
/// inventing a category, so odd hardware still shows *something* instead
/// of silently vanishing.
fn categorize_temp_label(label: &str) -> &'static str {
    let l = label.to_lowercase();
    if l.contains("tctl") || l.contains("tdie") || l.contains("k10temp") || l.contains("coretemp") || l.contains("cpu") {
        "CPU"
    } else if l.contains("amdgpu") || l.contains("nvidia") || l.contains("gpu") || l.contains("edge") || l.contains("junction") {
        "GPU"
    } else if l.contains("wmi") || l.contains("acpitz") || l.contains("systin") || l.contains("mobo") || l.contains("pch") || l.contains("gigabyte") {
        "Mainboard"
    } else if l.contains("nvme") || l.contains("ssd") || l.contains("disk") {
        "Datenträger"
    } else {
        "Sonstige"
    }
}

/// Best-effort GPU usage: tries NVIDIA's `nvidia-smi` first (works out of
/// the box wherever NVIDIA drivers are installed, no extra crate needed),
/// then falls back to the AMD `amdgpu` sysfs busy-percent file on Linux.
/// Returns None if neither is available -- there is no single
/// cross-platform crate for this that covers NVIDIA/AMD/Intel uniformly.
fn read_gpu_usage() -> Option<f32> {
    if let Some(pct) = read_nvidia_smi() {
        return Some(pct);
    }
    read_amdgpu_sysfs()
}

fn read_nvidia_smi() -> Option<f32> {
    let output = Command::new("nvidia-smi")
        .args(["--query-gpu=utilization.gpu", "--format=csv,noheader,nounits"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()?
        .trim()
        .parse::<f32>()
        .ok()
}

#[cfg(target_os = "linux")]
fn read_amdgpu_sysfs() -> Option<f32> {
    for entry in std::fs::read_dir("/sys/class/drm").ok()? {
        let entry = entry.ok()?;
        let busy_path = entry.path().join("device/gpu_busy_percent");
        if let Ok(contents) = std::fs::read_to_string(&busy_path) {
            if let Ok(value) = contents.trim().parse::<f32>() {
                return Some(value);
            }
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn read_amdgpu_sysfs() -> Option<f32> {
    None
}
