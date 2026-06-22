use std::{env, process::ExitCode};

use disk_nix_plan::default_capabilities;
use disk_nix_probe::{LinuxProbe, ProbeAdapter, ProbeStatus};

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    match args.as_slice() {
        [] => {
            print_help();
            Ok(())
        }
        [command] if command == "help" || command == "--help" || command == "-h" => {
            print_help();
            Ok(())
        }
        [command] if command == "topology" => {
            let probe = LinuxProbe::new();
            let result = probe.collect();
            print_topology_summary(&result);
            Ok(())
        }
        [command, flag] if command == "topology" && flag == "--json" => {
            let probe = LinuxProbe::new();
            let result = probe.collect();
            println!("{}", result.graph.to_json());
            Ok(())
        }
        [command] if command == "capabilities" => {
            for capability in default_capabilities() {
                println!(
                    "{:?} {:?} {:?}",
                    capability.node_kind, capability.operation, capability.risk
                );
            }
            Ok(())
        }
        [command, flag, spec] if command == "plan" && flag == "--spec" => {
            println!("planning is scaffolded; received desired spec at {spec}");
            println!("all future mutation plans will be safety-classified before execution");
            Ok(())
        }
        [unknown, ..] => Err(format!("unknown command '{unknown}'. Try 'disk-nix help'.")),
    }
}

fn print_help() {
    println!(
        "disk-nix\n\n\
         Usage:\n\
           disk-nix topology [--json]\n\
           disk-nix capabilities\n\
           disk-nix plan --spec <path>\n\n\
         Current commands are read-only scaffolding for the storage graph and planner."
    );
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
