use std::process::ExitCode;

use clap::{Parser, Subcommand};
use disk_nix_plan::default_capabilities;
use disk_nix_probe::{LinuxProbe, ProbeAdapter, ProbeStatus};

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug, Parser)]
#[command(version, about = "NixOS-native storage topology and lifecycle manager")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Inspect storage topology.
    Topology {
        /// Emit the canonical JSON graph.
        #[arg(long)]
        json: bool,
    },
    /// Show modeled storage operation capabilities and risk classes.
    Capabilities,
    /// Plan desired storage changes from a JSON spec.
    Plan {
        /// Desired storage specification path.
        #[arg(long)]
        spec: String,
    },
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Topology { json: false } => {
            let probe = LinuxProbe::new();
            let result = probe.collect().map_err(|error| error.to_string())?;
            print_topology_summary(&result);
            Ok(())
        }
        Command::Topology { json: true } => {
            let probe = LinuxProbe::new();
            let result = probe.collect().map_err(|error| error.to_string())?;
            println!(
                "{}",
                result.graph.to_json().map_err(|error| error.to_string())?
            );
            Ok(())
        }
        Command::Capabilities => {
            for capability in default_capabilities() {
                println!(
                    "{:?} {:?} {:?}",
                    capability.node_kind, capability.operation, capability.risk
                );
            }
            Ok(())
        }
        Command::Plan { spec } => {
            println!("planning is scaffolded; received desired spec at {spec}");
            println!("all future mutation plans will be safety-classified before execution");
            Ok(())
        }
    }
}

fn print_topology_summary(result: &disk_nix_probe::ProbeResult) {
    println!("Storage topology probe");
    println!("nodes: {}", result.graph.nodes.len());
    println!("edges: {}", result.graph.edges.len());
    println!();
    println!("Adapters:");

    for report in &result.reports {
        let status = match report.status {
            ProbeStatus::Available => "available",
            ProbeStatus::Unavailable => "unavailable",
            ProbeStatus::Partial => "partial",
            ProbeStatus::Failed => "failed",
        };

        if let Some(message) = &report.message {
            println!("  {:<12} {:<12} {}", report.adapter, status, message);
        } else {
            println!("  {:<12} {}", report.adapter, status);
        }
    }
}
