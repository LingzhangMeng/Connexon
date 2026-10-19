//! Hardware probe and consent-gated install planner.
//!
//! The probe is read-only. The planner is a pure function over the profile;
//! `cnx install` prints the plan and writes it, but Phase 0 does not download
//! anything. That boundary is deliberate: an installer must never silently
//! install a driver or a GPU stack the hardware cannot use.

use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct OsInfo {
    pub name: String,
    pub version: String,
    pub kernel: String,
    pub arch: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CpuInfo {
    pub model: String,
    pub physical_cores: usize,
    pub logical_threads: usize,
    pub features: Vec<String>,
    pub numa_nodes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryInfo {
    pub total_gib: f64,
    pub available_gib: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuInfo {
    pub vendor: String,
    pub model: String,
    pub arch: Option<String>,
    pub vram_gib: Option<f64>,
    pub driver: Option<String>,
    pub compute_supported: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Toolchains {
    pub r: Option<String>,
    pub python: Option<String>,
    pub cuda: Option<String>,
    pub rocm: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Profile {
    pub os: OsInfo,
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub gpus: Vec<GpuInfo>,
    pub toolchains: Toolchains,
}

#[derive(Debug, Clone, Serialize)]
pub struct Component {
    pub id: String,
    pub kind: String,
    pub version: String,
    pub size_download_mb: f64,
    pub size_install_mb: f64,
    pub license: String,
    pub reason: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallPlan {
    pub generated_at_unix: u64,
    pub profile: Profile,
    pub profile_kind: String,
    pub components: Vec<Component>,
    pub system_commands: Vec<String>,
    pub notes: Vec<String>,
}

pub fn probe() -> Profile {
    let (os_name, os_version) = read_os_release();
    let kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|text| text.trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let (model, physical_cores, logical_threads, features) = read_cpu();
    let (total_gib, available_gib) = read_memory();
    let gpus = read_gpus();
    let toolchains = Toolchains {
        r: command_version("Rscript", &["--version"]),
        python: command_version("python3", &["--version"]),
        cuda: command_version("nvcc", &["--version"]),
        rocm: command_version("hipcc", &["--version"]),
    };
    Profile {
        os: OsInfo {
            name: os_name,
            version: os_version,
            kernel,
            arch: std::env::consts::ARCH.to_string(),
        },
        cpu: CpuInfo {
            model,
            physical_cores,
            logical_threads,
            features,
            numa_nodes: numa_nodes(),
        },
        memory: MemoryInfo {
            total_gib,
            available_gib,
        },
        gpus,
        toolchains,
    }
}

fn read_os_release() -> (String, String) {
    let text = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let mut name = std::env::consts::OS.to_string();
    let mut version = String::new();
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("NAME=") {
            name = value.trim_matches('"').to_string();
        }
        if let Some(value) = line.strip_prefix("VERSION_ID=") {
            version = value.trim_matches('"').to_string();
        }
    }
    (name, version)
}

fn read_cpu() -> (String, usize, usize, Vec<String>) {
    let text = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let model = text
        .lines()
        .find(|line| line.starts_with("model name") || line.starts_with("Model"))
        .and_then(|line| line.split(':').nth(1))
        .map(|value| value.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let logical_threads = text
        .lines()
        .filter(|line| line.starts_with("processor"))
        .count()
        .max(1);
    let mut pairs = std::collections::HashSet::new();
    let mut physical_id: Option<String> = None;
    let mut core_id: Option<String> = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("physical id") {
            physical_id = value.split(':').nth(1).map(|s| s.trim().to_string());
        }
        if let Some(value) = line.strip_prefix("core id") {
            core_id = value.split(':').nth(1).map(|s| s.trim().to_string());
        }
        if let (Some(physical), Some(core)) = (&physical_id, &core_id) {
            pairs.insert((physical.clone(), core.clone()));
        }
    }
    let physical_cores = if pairs.is_empty() {
        (logical_threads / 2).max(1)
    } else {
        pairs.len()
    };
    let flags = text
        .lines()
        .find(|line| line.starts_with("flags") || line.starts_with("Features"))
        .and_then(|line| line.split(':').nth(1))
        .unwrap_or("")
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let interesting = [
        "sse4_2", "avx", "avx2", "avx512f", "fma", "neon", "sve", "sve2", "dotprod",
    ];
    let features = interesting
        .iter()
        .filter(|feature| flags.iter().any(|flag| flag == *feature))
        .map(|feature| feature.to_string())
        .collect();
    (model, physical_cores, logical_threads, features)
}

fn read_memory() -> (f64, f64) {
    let text = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut total_kb = 0u64;
    let mut available_kb = 0u64;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("MemTotal:") {
            total_kb = value
                .split_whitespace()
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
        }
        if let Some(value) = line.strip_prefix("MemAvailable:") {
            available_kb = value
                .split_whitespace()
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
        }
    }
    (
        total_kb as f64 / 1024.0 / 1024.0,
        available_kb as f64 / 1024.0 / 1024.0,
    )
}

fn numa_nodes() -> usize {
    std::fs::read_dir("/sys/devices/system/node")
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.file_name().to_string_lossy().starts_with("node"))
                .count()
        })
        .unwrap_or(0)
        .max(1)
}

fn read_gpus() -> Vec<GpuInfo> {
    let output = Command::new("lspci").arg("-nn").output();
    let text = output
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
        .unwrap_or_default();
    let nvidia = nvidia_info();
    let rocm_arch = rocm_arch();
    let amd_vram = amd_vram_gib();
    let mut gpus = Vec::new();
    for line in text.lines() {
        if !(line.contains("VGA compatible controller")
            || line.contains("3D controller")
            || line.contains("Display controller"))
        {
            continue;
        }
        let vendor = if line.contains("NVIDIA") {
            "nvidia"
        } else if line.contains("AMD") || line.contains("ATI") {
            "amd"
        } else if line.contains("Intel") {
            "intel"
        } else {
            "unknown"
        };
        let model = if let Some(position) = line.find("]: ") {
            line[position + 3..]
                .split(" (rev")
                .next()
                .unwrap_or("")
                .to_string()
        } else {
            line.to_string()
        };
        let mut gpu = GpuInfo {
            vendor: vendor.to_string(),
            model: model.trim().to_string(),
            arch: None,
            vram_gib: None,
            driver: None,
            compute_supported: false,
        };
        match vendor {
            "nvidia" => {
                if let Some((name, vram, driver)) = &nvidia {
                    gpu.model = name.clone();
                    gpu.vram_gib = *vram;
                    gpu.driver = driver.clone();
                    gpu.compute_supported = true;
                }
            }
            "amd" => {
                gpu.arch = rocm_arch.clone();
                gpu.vram_gib = amd_vram;
                if gpu.arch.is_none() && (line.contains("WX 3200") || line.contains("6981")) {
                    gpu.arch = Some("gfx803".to_string());
                }
                gpu.compute_supported = gpu
                    .arch
                    .as_deref()
                    .map(|arch| !arch.starts_with("gfx803"))
                    .unwrap_or(false);
            }
            _ => {}
        }
        gpus.push(gpu);
    }
    gpus
}

fn nvidia_info() -> Option<(String, Option<f64>, Option<String>)> {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total,driver_version",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().next()?;
    let parts: Vec<&str> = line.split(',').map(str::trim).collect();
    let name = parts.first()?.to_string();
    let vram = parts
        .get(1)
        .and_then(|value| value.parse::<f64>().ok())
        .map(|mib| mib / 1024.0);
    let driver = parts.get(2).map(|value| value.to_string());
    Some((name, vram, driver))
}

fn rocm_arch() -> Option<String> {
    let output = Command::new("rocminfo").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    for token in text.split_whitespace() {
        if token.starts_with("gfx") {
            return Some(token.trim_end_matches(',').to_string());
        }
    }
    None
}

fn amd_vram_gib() -> Option<f64> {
    let entries = std::fs::read_dir("/sys/class/drm").ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("card") || name.contains('-') {
            continue;
        }
        let path = entry.path().join("device/mem_info_vram_total");
        if let Ok(text) = std::fs::read_to_string(path) {
            if let Ok(bytes) = text.trim().parse::<u64>() {
                return Some(bytes as f64 / 1024.0 / 1024.0 / 1024.0);
            }
        }
    }
    None
}

fn command_version(program: &str, arguments: &[&str]) -> Option<String> {
    let output = Command::new(program).args(arguments).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    if text.trim().is_empty() {
        text = String::from_utf8_lossy(&output.stderr).to_string();
    }
    text.lines().next().map(|line| line.trim().to_string())
}

pub fn build_plan(profile: &Profile) -> InstallPlan {
    let has_nvidia = profile
        .gpus
        .iter()
        .any(|gpu| gpu.vendor == "nvidia" && gpu.compute_supported);
    let has_rocm = profile
        .gpus
        .iter()
        .any(|gpu| gpu.vendor == "amd" && gpu.compute_supported);
    let has_intel = profile.gpus.iter().any(|gpu| gpu.vendor == "intel");
    let profile_kind = if has_nvidia {
        "cuda"
    } else if has_rocm {
        "rocm"
    } else {
        "cpu-only"
    };

    let mut components = vec![
        component(
            "openblas",
            "cpu.blas",
            "0.3.28",
            12.0,
            58.0,
            "BSD-3-Clause",
            "optimised GEMM; pin to 1 thread per worker",
            true,
        ),
        component(
            "openmp-runtime",
            "cpu.threads",
            "18.1",
            2.0,
            6.0,
            "Apache-2.0",
            "thread pool interop",
            true,
        ),
        component(
            "hwloc",
            "cpu.topology",
            "2.11",
            3.0,
            9.0,
            "BSD-3-Clause",
            "NUMA and topology detection",
            true,
        ),
        component(
            "mimalloc",
            "cpu.allocator",
            "3.1",
            1.0,
            4.0,
            "MIT",
            "allocator for many small objects",
            true,
        ),
    ];
    if has_nvidia {
        components.push(component(
            "cuda-runtime",
            "gpu.cuda",
            "12.8",
            180.0,
            520.0,
            "NVIDIA EULA",
            "CUDA runtime and driver interface",
            true,
        ));
        components.push(component(
            "cublas",
            "gpu.cuda",
            "12.8",
            90.0,
            260.0,
            "NVIDIA EULA",
            "dense linear algebra on NVIDIA GPUs",
            true,
        ));
        if profile
            .gpus
            .iter()
            .filter(|gpu| gpu.vendor == "nvidia")
            .count()
            > 1
        {
            components.push(component(
                "nccl",
                "gpu.cuda",
                "2.24",
                15.0,
                45.0,
                "NVIDIA EULA",
                "multi-GPU collectives",
                true,
            ));
        }
        components.push(component(
            "cudnn",
            "gpu.cuda",
            "9.6",
            400.0,
            1200.0,
            "NVIDIA EULA",
            "deep-learning primitives (optional)",
            false,
        ));
    }
    if has_rocm {
        components.push(component(
            "rocm-runtime",
            "gpu.rocm",
            "6.4",
            220.0,
            700.0,
            "AMD license",
            "HIP runtime and ROCm libraries",
            true,
        ));
        components.push(component(
            "rocblas",
            "gpu.rocm",
            "6.4",
            120.0,
            380.0,
            "BSD-3-Clause",
            "dense linear algebra on AMD GPUs",
            true,
        ));
        if profile
            .gpus
            .iter()
            .filter(|gpu| gpu.vendor == "amd")
            .count()
            > 1
        {
            components.push(component(
                "rccl",
                "gpu.rocm",
                "6.4",
                18.0,
                52.0,
                "BSD-3-Clause",
                "multi-GPU collectives",
                true,
            ));
        }
    }
    if has_intel {
        components.push(component(
            "oneapi-level-zero",
            "gpu.intel",
            "1.18",
            35.0,
            120.0,
            "MIT",
            "Intel GPU compute (optional)",
            false,
        ));
    }

    let mut notes = Vec::new();
    if profile.gpus.is_empty() {
        notes.push("No GPU detected; the CPU-only profile is selected.".to_string());
    }
    if profile
        .gpus
        .iter()
        .any(|gpu| gpu.vendor == "amd" && !gpu.compute_supported)
    {
        notes.push("AMD GPU detected but not supported by modern ROCm (e.g. gfx803); no ROCm components are planned.".to_string());
    }
    if profile.memory.total_gib < 16.0 {
        notes.push(
            "Less than 16 GiB RAM; reduce batch sizes and keep the working set small.".to_string(),
        );
    }
    if profile.cpu.physical_cores <= 2 {
        notes.push("Few physical cores; parallelism will be limited.".to_string());
    }
    notes.push(
        "Phase 0: this is a validated plan. No downloads or system changes are performed."
            .to_string(),
    );

    let system_commands = vec![
        "# optional system-level packages (require sudo; never run automatically):".to_string(),
        "sudo apt install libnuma-dev hwloc".to_string(),
    ];

    InstallPlan {
        generated_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0),
        profile: profile.clone(),
        profile_kind: profile_kind.to_string(),
        components,
        system_commands,
        notes,
    }
}

#[allow(clippy::too_many_arguments)]
fn component(
    id: &str,
    kind: &str,
    version: &str,
    size_download_mb: f64,
    size_install_mb: f64,
    license: &str,
    reason: &str,
    required: bool,
) -> Component {
    Component {
        id: id.to_string(),
        kind: kind.to_string(),
        version: version.to_string(),
        size_download_mb,
        size_install_mb,
        license: license.to_string(),
        reason: reason.to_string(),
        required,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_and_plan_are_consistent() {
        let profile = probe();
        assert!(profile.cpu.logical_threads >= 1);
        assert!(profile.memory.total_gib >= 0.0);
        let plan = build_plan(&profile);
        assert!(!plan.components.is_empty());
        assert!(matches!(
            plan.profile_kind.as_str(),
            "cuda" | "rocm" | "cpu-only"
        ));
        assert!(plan
            .components
            .iter()
            .any(|component| component.id == "openblas"));
    }
}
