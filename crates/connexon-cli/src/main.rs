//! `cnx` — the Connexon command-line interface.

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use connexon_core::Value;
use connexon_runtime::{build_plan, probe, run_source, InstallPlan, Profile};

#[derive(Parser)]
#[command(
    name = "cnx",
    version,
    author = "Prof. Dr. rer. nat. Lingzhang Meng",
    about = "Connexon — one channel for R and Python",
    long_about = "Connexon is a Rust runtime and language that composes R packages and Python tools into one typed, reproducible program.",
    after_help = "Developed by Prof. Dr. rer. nat. Lingzhang Meng (孟令章 教授)\nResearch Center for Medical Big Data and Artificial Intelligence,\nThe People's Hospital of Guangxi Zhuang Autonomous Region (Guangxi Academy of Medical Sciences)."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a .cnx program.
    Run {
        /// Path to the .cnx file.
        file: PathBuf,
    },
    /// Evaluate a Connexon expression.
    Eval {
        /// Source text.
        source: String,
    },
    /// Probe the machine and print the proposed install plan.
    Doctor,
    /// Probe, plan, and (after confirmation) write the install plan.
    Install {
        /// Write the plan and exit without prompting.
        #[arg(long)]
        plan_only: bool,
        /// Accept the plan without prompting (non-interactive use).
        #[arg(long)]
        yes: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(error) = execute(cli) {
        eprintln!("cnx: {error}");
        std::process::exit(1);
    }
}

fn execute(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Commands::Run { file } => {
            let source = fs::read_to_string(&file)
                .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
            let value = run_source(&source)?;
            if !matches!(value, Value::Null) {
                println!("{}", value.display());
            }
        }
        Commands::Eval { source } => {
            let value = run_source(&source)?;
            if !matches!(value, Value::Null) {
                println!("{}", value.display());
            }
        }
        Commands::Doctor => {
            let profile = probe();
            let plan = build_plan(&profile);
            print_profile(&profile);
            print_plan(&plan);
        }
        Commands::Install { plan_only, yes } => {
            let profile = probe();
            let plan = build_plan(&profile);
            print_plan(&plan);
            if plan_only {
                write_plan(&plan)?;
                println!("\nInstall plan written to connexon.install-plan.json");
                return Ok(());
            }
            let proceed = if yes { true } else { prompt()? };
            if !proceed {
                println!("aborted; nothing was installed");
                return Ok(());
            }
            write_plan(&plan)?;
            println!("\nPhase 0 planner: the plan is validated and written, but no downloads were performed.");
            println!(
                "{} components are planned; the next phase will apply them transactionally.",
                plan.components.len()
            );
        }
    }
    Ok(())
}

fn print_profile(profile: &Profile) {
    println!("Connexon system profile");
    println!(
        "  OS        {} {} ({})",
        profile.os.name, profile.os.version, profile.os.arch
    );
    println!("  Kernel    {}", profile.os.kernel);
    println!(
        "  CPU       {} — {} physical cores / {} threads",
        profile.cpu.model, profile.cpu.physical_cores, profile.cpu.logical_threads
    );
    println!(
        "  Features  {}",
        if profile.cpu.features.is_empty() {
            "none detected".to_string()
        } else {
            profile.cpu.features.join(", ")
        }
    );
    println!(
        "  Memory    {:.1} GiB total, {:.1} GiB available",
        profile.memory.total_gib, profile.memory.available_gib
    );
    println!("  NUMA      {} node(s)", profile.cpu.numa_nodes);
    if profile.gpus.is_empty() {
        println!("  GPU       none detected");
    } else {
        for gpu in &profile.gpus {
            println!(
                "  GPU       {} {} (vram: {}, arch: {}, compute: {})",
                gpu.vendor,
                gpu.model,
                gpu.vram_gib
                    .map(|vram| format!("{vram:.1} GiB"))
                    .unwrap_or_else(|| "unknown".to_string()),
                gpu.arch.as_deref().unwrap_or("unknown"),
                gpu.compute_supported
            );
        }
    }
    println!(
        "  R         {}",
        profile.toolchains.r.as_deref().unwrap_or("not found")
    );
    println!(
        "  Python    {}",
        profile.toolchains.python.as_deref().unwrap_or("not found")
    );
    println!(
        "  CUDA      {}",
        profile.toolchains.cuda.as_deref().unwrap_or("not found")
    );
    println!(
        "  ROCm      {}",
        profile.toolchains.rocm.as_deref().unwrap_or("not found")
    );
}

fn print_plan(plan: &InstallPlan) {
    println!("\nProposed profile: {}", plan.profile_kind);
    println!("  sta component          version    download  license          reason");
    for component in &plan.components {
        println!(
            "  [{}] {:<18} {:<10} {:>7.0} MB  {:<16} {}",
            if component.required { "x" } else { " " },
            component.id,
            component.version,
            component.size_download_mb,
            component.license,
            component.reason
        );
    }
    let download: f64 = plan
        .components
        .iter()
        .map(|component| component.size_download_mb)
        .sum();
    let installed: f64 = plan
        .components
        .iter()
        .map(|component| component.size_install_mb)
        .sum();
    println!("  Total: {download:.0} MB download / {installed:.0} MB installed");
    for note in &plan.notes {
        println!("  note: {note}");
    }
    for command in &plan.system_commands {
        println!("  system: {command}");
    }
}

fn write_plan(plan: &InstallPlan) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(plan)?;
    fs::write("connexon.install-plan.json", json)?;
    Ok(())
}

fn prompt() -> Result<bool, Box<dyn std::error::Error>> {
    print!("Proceed with this plan? [y/N] ");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}
