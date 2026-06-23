#![recursion_limit = "256"]

use std::{
    collections::BTreeSet,
    fmt,
    io::{self, Write},
    os::unix::fs::PermissionsExt,
    process::ExitCode,
};

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use clap_mangen::Man;
use disk_nix_exec::{ExecutionMode, ExecutionReport, ExecutionStatus, prepare_execution};
use disk_nix_model::{Node, NodeKind, StorageGraph};
use disk_nix_plan::{
    ApplyPolicy, Plan, TopologyComparison, TopologyDiagnosticLevel, compare_plan_with_topology,
    default_capabilities, plan_and_policy_from_json_bytes, plan_from_json_bytes,
};
use disk_nix_probe::{LinuxProbe, ProbeAdapter, ProbeStatus};

fn main() -> ExitCode {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    match run(Cli::parse(), &mut stdout) {
        Ok(()) => ExitCode::SUCCESS,
        Err(AppError::Io(error)) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "disk-nix",
    version,
    about = "NixOS-native storage topology and lifecycle manager"
)]
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
    /// Show probe adapter availability and degradation status.
    ProbeStatus {
        /// Emit JSON probe reports.
        #[arg(long)]
        json: bool,
    },
    /// Show modeled storage operation capabilities and risk classes.
    Capabilities {
        /// Emit JSON capability records.
        #[arg(long)]
        json: bool,
    },
    /// List block-like storage devices.
    Devices {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered partition nodes.
    Partitions {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered filesystems.
    Filesystems {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered volumes, pools, datasets, LUNs, and exports.
    Volumes {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered storage pools and volume groups.
    Pools {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered snapshots across LVM, Btrfs, and ZFS.
    Snapshots {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered mapping layers such as LUKS, dm, LVM, VDO, and multipath.
    Mappings {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered mountpoints.
    Mounts {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered iSCSI, LUN, and NFS nodes.
    NetworkStorage {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List storage identity fields such as UUIDs, labels, serials, and WWNs.
    Ids {
        /// Emit JSON for identity-bearing graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// Summarize capacity, free space, allocation, and utilization.
    Usage {
        /// Emit JSON for graph nodes with size or usage information.
        #[arg(long)]
        json: bool,
    },
    /// Inspect a graph node by id, path, name, UUID, label, serial, or property.
    Inspect {
        /// Query value to inspect.
        query: String,
        /// Emit JSON for matched nodes and direct relationships.
        #[arg(long)]
        json: bool,
    },
    /// Plan desired storage changes from a JSON spec.
    Plan {
        /// Desired storage specification path.
        #[arg(long)]
        spec: String,
        /// Probe current topology and compare planned actions against it.
        #[arg(long)]
        probe_current: bool,
        /// Emit JSON plan output.
        #[arg(long)]
        json: bool,
    },
    /// Evaluate apply policy for a desired storage spec.
    Apply {
        /// Desired storage specification path.
        #[arg(long)]
        spec: String,
        /// Probe current topology and compare planned actions against it.
        #[arg(long)]
        probe_current: bool,
        /// Attempt execution after policy validation.
        #[arg(long)]
        execute: bool,
        /// Write a reviewable shell script for the allowed command and verification plan.
        #[arg(long)]
        script_out: Option<String>,
        /// Write the JSON apply report to a file before exit handling.
        #[arg(long)]
        report_out: Option<String>,
        /// Emit JSON apply report.
        #[arg(long)]
        json: bool,
    },
    /// Validate a desired storage spec and policy without treating policy blocks as command failure.
    Validate {
        /// Desired storage specification path.
        #[arg(long)]
        spec: String,
        /// Probe current topology and compare planned actions against it.
        #[arg(long)]
        probe_current: bool,
        /// Write a reviewable shell script when every planned action is policy-allowed.
        #[arg(long)]
        script_out: Option<String>,
        /// Write the JSON validation report to a file.
        #[arg(long)]
        report_out: Option<String>,
        /// Emit JSON validation report.
        #[arg(long)]
        json: bool,
    },
    /// Emit the supported desired-spec JSON contract.
    Schema,
    /// Generate shell completions.
    Completions {
        /// Shell completion format to emit.
        shell: Shell,
    },
    /// Generate a roff manpage.
    Manpage,
}

#[derive(Debug)]
enum AppError {
    Message(String),
    Io(io::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message(message) => f.write_str(message),
            Self::Io(error) => error.fmt(f),
        }
    }
}

impl From<io::Error> for AppError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

fn run(cli: Cli, output: &mut impl Write) -> Result<(), AppError> {
    match cli.command {
        Command::Topology { json: false } => {
            let probe = LinuxProbe::new();
            let result = probe
                .collect()
                .map_err(|error| AppError::Message(error.to_string()))?;
            print_topology_summary(output, &result)?;
            Ok(())
        }
        Command::Topology { json: true } => {
            let graph = collect_graph()?;
            writeln!(
                output,
                "{}",
                graph
                    .to_json()
                    .map_err(|error| AppError::Message(error.to_string()))?
            )?;
            Ok(())
        }
        Command::ProbeStatus { json } => {
            let probe = LinuxProbe::new();
            let result = probe
                .collect()
                .map_err(|error| AppError::Message(error.to_string()))?;
            if json {
                writeln!(
                    output,
                    "{}",
                    serde_json::to_string_pretty(&result.reports)
                        .map_err(|error| AppError::Message(error.to_string()))?
                )?;
            } else {
                print_probe_reports(output, &result.reports)?;
            }
            Ok(())
        }
        Command::Capabilities { json } => {
            let capabilities = default_capabilities();
            if json {
                writeln!(
                    output,
                    "{}",
                    serde_json::to_string_pretty(&capabilities)
                        .map_err(|error| AppError::Message(error.to_string()))?
                )?;
            } else {
                for capability in capabilities {
                    writeln!(
                        output,
                        "{:?} {:?} {:?}",
                        capability.node_kind, capability.operation, capability.risk
                    )?;
                }
            }
            Ok(())
        }
        Command::Devices { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_device_node)?;
            } else {
                print_devices(output, &graph)?;
            }
            Ok(())
        }
        Command::Partitions { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_partition_node)?;
            } else {
                print_partitions(output, &graph)?;
            }
            Ok(())
        }
        Command::Filesystems { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_filesystem_node)?;
            } else {
                print_filesystems(output, &graph)?;
            }
            Ok(())
        }
        Command::Volumes { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_volume_node)?;
            } else {
                print_volumes(output, &graph)?;
            }
            Ok(())
        }
        Command::Pools { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_pool_node)?;
            } else {
                print_pools(output, &graph)?;
            }
            Ok(())
        }
        Command::Snapshots { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_snapshot_node)?;
            } else {
                print_snapshots(output, &graph)?;
            }
            Ok(())
        }
        Command::Mappings { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_mapping_node)?;
            } else {
                print_mappings(output, &graph)?;
            }
            Ok(())
        }
        Command::Mounts { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_mount_node)?;
            } else {
                print_mounts(output, &graph)?;
            }
            Ok(())
        }
        Command::NetworkStorage { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_network_storage_node)?;
            } else {
                print_network_storage(output, &graph)?;
            }
            Ok(())
        }
        Command::Ids { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, has_identity)?;
            } else {
                print_ids(output, &graph)?;
            }
            Ok(())
        }
        Command::Usage { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, has_capacity_or_usage)?;
            } else {
                print_usage(output, &graph)?;
            }
            Ok(())
        }
        Command::Inspect { query, json } => {
            let graph = collect_graph()?;
            if json {
                print_inspect_json(output, &graph, &query)?;
            } else {
                print_inspect(output, &graph, &query)?;
            }
            Ok(())
        }
        Command::Plan {
            spec,
            probe_current,
            json,
        } => {
            let bytes = std::fs::read(&spec)?;
            let mut plan = plan_from_json_bytes(&bytes)
                .map_err(|error| AppError::Message(format!("failed to parse {spec}: {error}")))?;
            if probe_current {
                plan = compare_plan_with_topology(plan, &collect_graph()?);
            }
            if json {
                writeln!(
                    output,
                    "{}",
                    plan.to_json()
                        .map_err(|error| AppError::Message(error.to_string()))?
                )?;
            } else {
                print_plan(output, &plan)?;
            }
            Ok(())
        }
        Command::Apply {
            spec,
            probe_current,
            execute,
            script_out,
            report_out,
            json,
        } => {
            let mode = if execute {
                ExecutionMode::Execute
            } else {
                ExecutionMode::DryRun
            };
            let report = prepare_apply_report(&spec, probe_current, mode)?;
            if let Some(report_out) = report_out.as_deref() {
                write_execution_report(report_out, &report)?;
            }
            if let Some(script_out) = script_out.as_deref() {
                write_execution_script(script_out, &report)?;
            }

            if json {
                writeln!(
                    output,
                    "{}",
                    report
                        .to_json()
                        .map_err(|error| AppError::Message(error.to_string()))?
                )?;
            } else {
                print_execution_report(output, &report, execute)?;
            }

            if report.status == ExecutionStatus::Blocked {
                return Err(AppError::Message(format!(
                    "apply policy blocked {} action(s)",
                    report.apply.blocked_count
                )));
            }
            if matches!(
                report.status,
                ExecutionStatus::NotReady | ExecutionStatus::Failed
            ) {
                return Err(AppError::Message(report.messages.join("; ")));
            }

            Ok(())
        }
        Command::Validate {
            spec,
            probe_current,
            script_out,
            report_out,
            json,
        } => {
            let report = prepare_apply_report(&spec, probe_current, ExecutionMode::DryRun)?;
            if let Some(report_out) = report_out.as_deref() {
                write_execution_report(report_out, &report)?;
            }
            if let Some(script_out) = script_out.as_deref() {
                write_execution_script(script_out, &report)?;
            }

            if json {
                writeln!(
                    output,
                    "{}",
                    report
                        .to_json()
                        .map_err(|error| AppError::Message(error.to_string()))?
                )?;
            } else {
                print_execution_report(output, &report, false)?;
            }

            Ok(())
        }
        Command::Schema => {
            writeln!(
                output,
                "{}",
                serde_json::to_string_pretty(&spec_schema())
                    .map_err(|error| AppError::Message(error.to_string()))?
            )?;
            Ok(())
        }
        Command::Completions { shell } => {
            let mut command = Cli::command();
            generate(shell, &mut command, "disk-nix", output);
            Ok(())
        }
        Command::Manpage => {
            let command = Cli::command();
            Man::new(command).render(output)?;
            Ok(())
        }
    }
}

fn spec_schema() -> serde_json::Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://github.com/midischwarz12/disk-nix/schema/disk-nix-spec.schema.json",
        "title": "disk-nix desired storage spec",
        "description": "Desired storage declaration accepted by disk-nix plan, apply, and validate. The CLI accepts either this direct shape or a wrapper with { spec, apply } as produced by the NixOS module.",
        "type": "object",
        "additionalProperties": true,
        "properties": {
            "version": {
                "type": "integer",
                "description": "Optional spec version marker for callers."
            },
            "spec": {
                "$ref": "#/$defs/specBody",
                "description": "NixOS module wrapper body. When present, planner lifecycle inputs are read from this object."
            },
            "apply": {
                "$ref": "#/$defs/applyPolicy"
            },
            "filesystems": {
                "$ref": "#/$defs/filesystemMap"
            },
            "swaps": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "luks": {
                "$ref": "#/$defs/luksSpec"
            },
            "nfs": {
                "$ref": "#/$defs/nfsSpec"
            },
            "iscsi": {
                "$ref": "#/$defs/iscsiSpec"
            },
            "disks": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "partitions": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "btrfsSubvolumes": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "btrfsQgroups": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "vdoVolumes": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "physicalVolumes": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "luksKeyslots": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "luksTokens": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "volumes": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "volumeGroups": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "thinPools": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "lvmSnapshots": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "lvmCaches": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "loopDevices": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "mdRaids": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "multipathMaps": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "pools": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "datasets": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "zvols": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "luns": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "nvmeNamespaces": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "iscsiSessions": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "exports": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "caches": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "snapshots": {
                "$ref": "#/$defs/snapshotMap"
            }
        },
        "$defs": {
            "specBody": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "filesystems": { "$ref": "#/$defs/filesystemMap" },
                    "swaps": { "$ref": "#/$defs/lifecycleMap" },
                    "luks": { "$ref": "#/$defs/luksSpec" },
                    "nfs": { "$ref": "#/$defs/nfsSpec" },
                    "iscsi": { "$ref": "#/$defs/iscsiSpec" },
                    "disks": { "$ref": "#/$defs/lifecycleMap" },
                    "partitions": { "$ref": "#/$defs/lifecycleMap" },
                    "btrfsSubvolumes": { "$ref": "#/$defs/lifecycleMap" },
                    "btrfsQgroups": { "$ref": "#/$defs/lifecycleMap" },
                    "vdoVolumes": { "$ref": "#/$defs/lifecycleMap" },
                    "physicalVolumes": { "$ref": "#/$defs/lifecycleMap" },
                    "luksKeyslots": { "$ref": "#/$defs/lifecycleMap" },
                    "luksTokens": { "$ref": "#/$defs/lifecycleMap" },
                    "volumes": { "$ref": "#/$defs/lifecycleMap" },
                    "volumeGroups": { "$ref": "#/$defs/lifecycleMap" },
                    "thinPools": { "$ref": "#/$defs/lifecycleMap" },
                    "lvmSnapshots": { "$ref": "#/$defs/lifecycleMap" },
                    "lvmCaches": { "$ref": "#/$defs/lifecycleMap" },
                    "loopDevices": { "$ref": "#/$defs/lifecycleMap" },
                    "mdRaids": { "$ref": "#/$defs/lifecycleMap" },
                    "multipathMaps": { "$ref": "#/$defs/lifecycleMap" },
                    "pools": { "$ref": "#/$defs/lifecycleMap" },
                    "datasets": { "$ref": "#/$defs/lifecycleMap" },
                    "zvols": { "$ref": "#/$defs/lifecycleMap" },
                    "luns": { "$ref": "#/$defs/lifecycleMap" },
                    "nvmeNamespaces": { "$ref": "#/$defs/lifecycleMap" },
                    "iscsiSessions": { "$ref": "#/$defs/lifecycleMap" },
                    "exports": { "$ref": "#/$defs/lifecycleMap" },
                    "caches": { "$ref": "#/$defs/lifecycleMap" },
                    "snapshots": { "$ref": "#/$defs/snapshotMap" }
                }
            },
            "filesystemMap": {
                "type": "object",
                "additionalProperties": { "$ref": "#/$defs/filesystem" }
            },
            "filesystem": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "mountpoint": { "type": "string" },
                    "device": { "type": "string" },
                    "fsType": { "type": "string" },
                    "type": { "type": "string" },
                    "operation": { "$ref": "#/$defs/operation" },
                    "action": { "$ref": "#/$defs/operation" },
                    "resizePolicy": {
                        "type": "string",
                        "enum": ["none", "grow-only", "shrink-allowed"]
                    },
                    "desiredSize": { "type": ["string", "number"] },
                    "targetSize": { "type": ["string", "number"] },
                    "size": { "type": ["string", "number"] },
                    "options": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "addDevices": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "removeDevices": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "replaceDevices": {
                        "type": "object",
                        "additionalProperties": { "type": "string" }
                    },
                    "renameTo": { "type": "string" },
                    "renameTarget": { "type": "string" },
                    "newName": { "type": "string" },
                    "properties": {
                        "type": "object",
                        "additionalProperties": true
                    },
                    "preserveData": { "type": "boolean", "default": true },
                    "readOnly": { "type": "boolean" },
                    "readonly": { "type": "boolean" }
                }
            },
            "lifecycleMap": {
                "type": "object",
                "additionalProperties": { "$ref": "#/$defs/lifecycleObject" }
            },
            "luksSpec": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "devices": { "$ref": "#/$defs/lifecycleMap" }
                }
            },
            "nfsSpec": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "mounts": { "$ref": "#/$defs/nfsMountMap" }
                }
            },
            "nfsMountMap": {
                "type": "object",
                "additionalProperties": { "$ref": "#/$defs/nfsMount" }
            },
            "nfsMount": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "source": { "type": "string" },
                    "device": { "type": "string" },
                    "fsType": {
                        "type": "string",
                        "enum": ["nfs", "nfs4"]
                    },
                    "operation": { "$ref": "#/$defs/operation" },
                    "action": { "$ref": "#/$defs/operation" },
                    "mountpoint": { "type": "string" },
                    "options": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "neededForBoot": { "type": "boolean" },
                    "destroy": { "type": "boolean" },
                    "preserveData": { "type": "boolean", "default": true }
                }
            },
            "iscsiSpec": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "initiatorName": { "type": ["string", "null"] },
                    "discoverPortal": { "type": ["string", "null"] },
                    "enableAutoLoginOut": { "type": "boolean" },
                    "extraConfig": { "type": "string" },
                    "sessions": { "$ref": "#/$defs/lifecycleMap" },
                    "boot": { "$ref": "#/$defs/iscsiBoot" }
                }
            },
            "iscsiBoot": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "enable": { "type": "boolean" },
                    "discoverPortal": { "type": ["string", "null"] },
                    "target": { "type": ["string", "null"] },
                    "loginAll": { "type": "boolean" },
                    "logLevel": { "type": "integer" },
                    "extraIscsiCommands": { "type": "string" },
                    "extraConfig": { "type": ["string", "null"] }
                }
            },
            "lifecycleObject": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "operation": { "$ref": "#/$defs/operation" },
                    "action": { "$ref": "#/$defs/operation" },
                    "addDevices": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "devices": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "removeDevices": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "replaceDevices": {
                        "type": "object",
                        "additionalProperties": { "type": "string" }
                    },
                    "properties": {
                        "type": "object",
                        "additionalProperties": true
                    },
                    "desiredSize": { "type": ["string", "number"] },
                    "targetSize": { "type": ["string", "number"] },
                    "size": { "type": ["string", "number"] },
                    "name": { "type": "string" },
                    "target": { "type": "string" },
                    "path": { "type": "string" },
                    "mountpoint": { "type": "string" },
                    "device": { "type": "string" },
                    "disk": { "type": "string" },
                    "client": { "type": "string" },
                    "portal": { "type": "string" },
                    "namespaceId": { "type": ["string", "number"] },
                    "nsid": { "type": ["string", "number"] },
                    "controllers": { "type": "string" },
                    "controllerId": { "type": ["string", "number"] },
                    "controller": { "type": ["string", "number"] },
                    "keySlot": { "type": ["string", "number"] },
                    "keyFile": { "type": "string" },
                    "newKeyFile": { "type": "string" },
                    "tokenId": { "type": ["string", "number"] },
                    "tokenFile": { "type": "string" },
                    "jsonFile": { "type": "string" },
                    "options": { "type": "string" },
                    "priority": { "type": "integer" },
                    "randomEncryption": { "type": "boolean" },
                    "allowDiscards": { "type": "boolean" },
                    "bypassWorkqueues": { "type": "boolean" },
                    "preLVM": { "type": "boolean" },
                    "start": { "type": ["string", "number"] },
                    "startOffset": { "type": ["string", "number"] },
                    "end": { "type": ["string", "number"] },
                    "endOffset": { "type": ["string", "number"] },
                    "partitionNumber": { "type": ["string", "number"] },
                    "number": { "type": ["string", "number"] },
                    "partitionType": { "type": "string" },
                    "level": { "type": "string" },
                    "raidLevel": { "type": "string" },
                    "type": { "type": "string" },
                    "destroy": { "type": "boolean" },
                    "preserveData": { "type": "boolean", "default": true },
                    "metadata": {
                        "type": "object",
                        "additionalProperties": true
                    }
                }
            },
            "snapshotMap": {
                "type": "object",
                "additionalProperties": { "$ref": "#/$defs/snapshot" }
            },
            "snapshot": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "target": { "type": "string" },
                    "operation": { "$ref": "#/$defs/operation" },
                    "action": { "$ref": "#/$defs/operation" },
                    "destroy": { "type": "boolean" },
                    "rollback": { "type": "boolean" },
                    "cloneTo": { "type": "string" },
                    "cloneTarget": { "type": "string" },
                    "clone": { "type": "string" },
                    "renameTo": { "type": "string" },
                    "renameTarget": { "type": "string" },
                    "newName": { "type": "string" },
                    "recursiveRollback": { "type": "boolean" },
                    "recursive": { "type": "boolean" },
                    "zfs.rollbackRecursive": { "type": "boolean" },
                    "hold": { "type": "string" },
                    "holdTag": { "type": "string" },
                    "releaseHold": { "type": "string" },
                    "readOnly": { "type": "boolean" },
                    "readonly": { "type": "boolean" },
                    "preserveData": { "type": "boolean", "default": true },
                    "metadata": {
                        "type": "object",
                        "additionalProperties": true
                    }
                }
            },
            "operation": {
                "type": "string",
                "enum": [
                    "create",
                    "format",
                    "grow",
                    "shrink",
                    "check",
                    "repair",
                    "scrub",
                    "trim",
                    "rescan",
                    "replace-device",
                    "add-device",
                    "remove-device",
                    "add-key",
                    "remove-key",
                    "import-token",
                    "remove-token",
                    "set-property",
                    "snapshot",
                    "clone",
                    "promote",
                    "import",
                    "export",
                    "unexport",
                    "attach",
                    "detach",
                    "activate",
                    "deactivate",
                    "assemble",
                    "start",
                    "stop",
                    "login",
                    "logout",
                    "open",
                    "close",
                    "mount",
                    "unmount",
                    "remount",
                    "rename",
                    "rebalance",
                    "rollback",
                    "destroy"
                ]
            },
            "applyPolicy": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "mode": {
                        "type": "string",
                        "enum": ["manual", "activation", "boot", "install"],
                        "default": "manual"
                    },
                    "allowDestructive": { "type": "boolean", "default": false },
                    "allowFormat": { "type": "boolean", "default": false },
                    "allowShrink": { "type": "boolean", "default": false },
                    "allowPotentialDataLoss": { "type": "boolean", "default": false },
                    "allowGrow": { "type": "boolean", "default": true },
                    "allowOffline": { "type": "boolean", "default": false },
                    "allowPropertyChanges": { "type": "boolean", "default": true },
                    "allowDeviceReplacement": { "type": "boolean", "default": true },
                    "allowRebalance": { "type": "boolean", "default": true },
                    "requireBackup": { "type": "boolean", "default": false },
                    "backupVerified": { "type": "boolean", "default": false },
                    "requireConfirmation": { "type": "boolean", "default": false },
                    "confirmation": { "type": "boolean", "default": false },
                    "requireConfirmationFile": { "type": ["string", "null"] },
                    "probeCurrent": {
                        "type": "boolean",
                        "description": "NixOS module helper that controls whether activation validation passes --probe-current."
                    },
                    "failOnBlocked": {
                        "type": "boolean",
                        "default": true,
                        "description": "NixOS module helper that controls whether activation uses apply and fails on blocked policy, or validate and reports blocked policy without failing the unit."
                    },
                    "scriptOut": {
                        "type": ["string", "null"],
                        "description": "NixOS module helper that controls activation --script-out."
                    },
                    "reportOut": {
                        "type": ["string", "null"],
                        "description": "NixOS module helper that controls activation --report-out."
                    }
                }
            }
        }
    })
}

fn prepare_apply_report(
    spec: &str,
    probe_current: bool,
    mode: ExecutionMode,
) -> Result<ExecutionReport, AppError> {
    let bytes = std::fs::read(spec)?;
    let (mut plan, mut policy) = plan_and_policy_from_json_bytes(&bytes)
        .map_err(|error| AppError::Message(format!("failed to parse {spec}: {error}")))?;
    if probe_current {
        plan = compare_plan_with_topology(plan, &collect_graph()?);
    }
    apply_confirmation_file(&mut policy)?;
    Ok(prepare_execution(&plan, policy, mode))
}

fn apply_confirmation_file(policy: &mut ApplyPolicy) -> Result<(), AppError> {
    let Some(path) = policy.require_confirmation_file.as_deref() else {
        return Ok(());
    };

    match std::fs::read_to_string(path) {
        Ok(content) if confirmation_file_accepts(&content) => {
            policy.confirmation = true;
            Ok(())
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::Io(error)),
    }
}

fn confirmation_file_accepts(content: &str) -> bool {
    content
        .lines()
        .any(|line| line.trim() == "disk-nix confirm")
}

fn write_execution_script(path: &str, report: &ExecutionReport) -> Result<(), AppError> {
    let script = report.to_shell_script().ok_or_else(|| {
        AppError::Message(
            "script generation requires apply policy to allow every planned action".to_string(),
        )
    })?;
    std::fs::write(path, script)?;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

fn write_execution_report(path: &str, report: &ExecutionReport) -> Result<(), AppError> {
    let mut report_json = report
        .to_json()
        .map_err(|error| AppError::Message(error.to_string()))?;
    report_json.push('\n');
    std::fs::write(path, report_json)?;
    Ok(())
}

fn collect_graph() -> Result<StorageGraph, AppError> {
    let probe = LinuxProbe::new();
    Ok(probe
        .collect()
        .map_err(|error| AppError::Message(error.to_string()))?
        .graph)
}

fn print_filtered_json(
    output: &mut impl Write,
    graph: &StorageGraph,
    predicate: fn(&Node) -> bool,
) -> Result<(), AppError> {
    let nodes: Vec<Node> = graph
        .nodes
        .iter()
        .filter(|node| predicate(node))
        .cloned()
        .collect();
    let node_ids: BTreeSet<String> = nodes.iter().map(|node| node.id.0.clone()).collect();
    let filtered = StorageGraph {
        nodes,
        edges: graph
            .edges
            .iter()
            .filter(|edge| {
                node_ids.contains(edge.from.0.as_str()) && node_ids.contains(edge.to.0.as_str())
            })
            .cloned()
            .collect(),
    };

    writeln!(
        output,
        "{}",
        filtered
            .to_json()
            .map_err(|error| AppError::Message(error.to_string()))?
    )?;
    Ok(())
}

fn print_inspect_json(
    output: &mut impl Write,
    graph: &StorageGraph,
    query: &str,
) -> Result<(), AppError> {
    let matched_ids: BTreeSet<String> = graph
        .find_nodes(query)
        .into_iter()
        .map(|node| node.id.0.clone())
        .collect();

    let mut node_ids = matched_ids.clone();
    let edges = graph
        .edges
        .iter()
        .filter(|edge| {
            matched_ids.contains(edge.from.0.as_str()) || matched_ids.contains(edge.to.0.as_str())
        })
        .inspect(|edge| {
            node_ids.insert(edge.from.0.clone());
            node_ids.insert(edge.to.0.clone());
        })
        .cloned()
        .collect();
    let nodes = graph
        .nodes
        .iter()
        .filter(|node| node_ids.contains(node.id.0.as_str()))
        .cloned()
        .collect();

    let subgraph = StorageGraph { nodes, edges };
    writeln!(
        output,
        "{}",
        subgraph
            .to_json()
            .map_err(|error| AppError::Message(error.to_string()))?
    )?;
    Ok(())
}

fn print_topology_summary(
    output: &mut impl Write,
    result: &disk_nix_probe::ProbeResult,
) -> io::Result<()> {
    writeln!(output, "Storage topology probe")?;
    writeln!(output, "nodes: {}", result.graph.nodes.len())?;
    writeln!(output, "edges: {}", result.graph.edges.len())?;
    writeln!(output)?;
    print_probe_reports(output, &result.reports)?;

    Ok(())
}

fn print_probe_reports(
    output: &mut impl Write,
    reports: &[disk_nix_probe::ProbeReport],
) -> io::Result<()> {
    writeln!(output, "Adapters:")?;

    for report in reports {
        let status = match report.status {
            ProbeStatus::Available => "available",
            ProbeStatus::Unavailable => "unavailable",
            ProbeStatus::Partial => "partial",
            ProbeStatus::Failed => "failed",
        };

        if let Some(message) = &report.message {
            writeln!(
                output,
                "  {:<12} {:<12} {}",
                report.adapter, status, message
            )?;
        } else {
            writeln!(output, "  {:<12} {}", report.adapter, status)?;
        }
    }

    Ok(())
}

fn print_devices(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:<36} PATH",
        "KIND", "NAME", "SIZE", "DETAILS"
    )?;
    for node in graph.nodes.iter().filter(|node| is_device_node(node)) {
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:<36} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            usage_details(node),
            node.path.as_deref().unwrap_or("-")
        )?;
    }
    Ok(())
}

fn print_partitions(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:<24} {:<36} PATH",
        "KIND", "NAME", "SIZE", "PARTUUID", "DETAILS"
    )?;
    for node in graph.nodes.iter().filter(|node| is_partition_node(node)) {
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:<24} {:<36} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            node.identity.partuuid.as_deref().unwrap_or("-"),
            usage_details(node),
            node.path.as_deref().unwrap_or("-")
        )?;
    }
    Ok(())
}

fn print_filesystems(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<32} {:>12} {:>12} {:<24} DETAILS",
        "KIND", "NAME", "USED", "FREE", "UUID"
    )?;
    for node in graph.nodes.iter().filter(|node| is_filesystem_node(node)) {
        writeln!(
            output,
            "{:<22} {:<32} {:>12} {:>12} {:<24} {}",
            node.kind,
            node.name,
            human_bytes(node.usage.as_ref().and_then(|usage| usage.used_bytes)),
            human_bytes(node.usage.as_ref().and_then(|usage| usage.free_bytes)),
            node.identity.uuid.as_deref().unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_volumes(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:>12} {:>12} DETAILS",
        "KIND", "NAME", "SIZE", "USED", "FREE"
    )?;
    for node in graph.nodes.iter().filter(|node| is_volume_node(node)) {
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:>12} {:>12} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            human_bytes(node.usage.as_ref().and_then(|usage| usage.used_bytes)),
            human_bytes(node.usage.as_ref().and_then(|usage| usage.free_bytes)),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_pools(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:>12} {:>12} {:>8} DETAILS",
        "KIND", "NAME", "SIZE", "USED", "FREE", "BACKING"
    )?;
    for node in graph.nodes.iter().filter(|node| is_pool_node(node)) {
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:>12} {:>12} {:>8} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            human_bytes(node.usage.as_ref().and_then(|usage| usage.used_bytes)),
            human_bytes(node.usage.as_ref().and_then(|usage| usage.free_bytes)),
            backing_count(graph, node),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_snapshots(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<48} {:>12} {:<32} DETAILS",
        "KIND", "NAME", "SIZE", "SOURCE"
    )?;
    for node in graph.nodes.iter().filter(|node| is_snapshot_node(node)) {
        writeln!(
            output,
            "{:<22} {:<48} {:>12} {:<32} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            snapshot_source(graph, node).unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_mappings(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>8} {:<44} PATH",
        "KIND", "NAME", "BACKING", "DETAILS"
    )?;
    for node in graph.nodes.iter().filter(|node| is_mapping_node(node)) {
        writeln!(
            output,
            "{:<22} {:<38} {:>8} {:<44} {}",
            node.kind,
            node.name,
            backing_count(graph, node),
            usage_details(node),
            node.path.as_deref().unwrap_or("-")
        )?;
    }
    Ok(())
}

fn print_mounts(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<48} {:<12} DETAILS",
        "KIND", "TARGET", "FSTYPE"
    )?;
    for node in graph.nodes.iter().filter(|node| is_mount_node(node)) {
        writeln!(
            output,
            "{:<22} {:<48} {:<12} {}",
            node.kind,
            node.name,
            property_value(node, "filesystem.type").unwrap_or("-"),
            mount_details(node)
        )?;
    }
    Ok(())
}

fn print_network_storage(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<48} {:>12} {:<36} PATH",
        "KIND", "NAME", "SIZE", "DETAILS"
    )?;
    for node in graph
        .nodes
        .iter()
        .filter(|node| is_network_storage_node(node))
    {
        writeln!(
            output,
            "{:<22} {:<48} {:>12} {:<36} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            usage_details(node),
            node.path.as_deref().unwrap_or("-")
        )?;
    }
    Ok(())
}

fn print_ids(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:<24} {:<24} {:<20} {:<20}",
        "KIND", "NAME", "UUID", "PARTUUID", "LABEL", "SERIAL/WWN"
    )?;
    for node in graph.nodes.iter().filter(|node| has_identity(node)) {
        let hardware_id = node
            .identity
            .serial
            .as_deref()
            .or(node.identity.wwn.as_deref())
            .unwrap_or("-");

        writeln!(
            output,
            "{:<22} {:<38} {:<24} {:<24} {:<20} {:<20}",
            node.kind,
            node.name,
            node.identity.uuid.as_deref().unwrap_or("-"),
            node.identity.partuuid.as_deref().unwrap_or("-"),
            node.identity.label.as_deref().unwrap_or("-"),
            hardware_id
        )?;
    }
    Ok(())
}

fn print_usage(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:>12} {:>12} {:>12} {:>7} {:<28} PATH",
        "KIND", "NAME", "SIZE", "USED", "FREE", "ALLOC", "USE%", "DETAILS"
    )?;
    for node in graph
        .nodes
        .iter()
        .filter(|node| has_capacity_or_usage(node))
    {
        let usage = node.usage.as_ref();
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:>12} {:>12} {:>12} {:>7} {:<28} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            human_bytes(usage.and_then(|usage| usage.used_bytes)),
            human_bytes(usage.and_then(|usage| usage.free_bytes)),
            human_bytes(usage.and_then(|usage| usage.allocated_bytes)),
            usage_percent(node),
            usage_details(node),
            node.path.as_deref().unwrap_or("-")
        )?;
    }
    Ok(())
}

fn has_identity(node: &Node) -> bool {
    !node.identity.is_empty()
}

fn print_inspect(output: &mut impl Write, graph: &StorageGraph, query: &str) -> io::Result<()> {
    let matches = graph.find_nodes(query);

    if matches.is_empty() {
        writeln!(output, "No storage graph nodes matched '{query}'.")?;
        return Ok(());
    }

    for (index, node) in matches.iter().enumerate() {
        if index > 0 {
            writeln!(output)?;
        }

        writeln!(output, "{} {}", node.kind, node.name)?;
        writeln!(output, "  id: {}", node.id.0)?;
        if let Some(path) = &node.path {
            writeln!(output, "  path: {path}")?;
        }
        if let Some(size_bytes) = node.size_bytes {
            writeln!(output, "  size: {}", human_bytes(Some(size_bytes)))?;
        }
        if let Some(usage) = &node.usage {
            if usage.used_bytes.is_some() || usage.free_bytes.is_some() {
                writeln!(
                    output,
                    "  usage: used={} free={}",
                    human_bytes(usage.used_bytes),
                    human_bytes(usage.free_bytes)
                )?;
            }
        }

        print_identity(output, node)?;
        print_properties(output, node)?;
        print_relationships(output, graph, node)?;
    }

    Ok(())
}

fn print_identity(output: &mut impl Write, node: &Node) -> io::Result<()> {
    if node.identity.is_empty() {
        return Ok(());
    }

    writeln!(output, "  identity:")?;
    for (key, value) in [
        ("uuid", node.identity.uuid.as_deref()),
        ("partuuid", node.identity.partuuid.as_deref()),
        ("label", node.identity.label.as_deref()),
        ("serial", node.identity.serial.as_deref()),
        ("wwn", node.identity.wwn.as_deref()),
    ] {
        if let Some(value) = value {
            writeln!(output, "    {key}: {value}")?;
        }
    }
    Ok(())
}

fn print_properties(output: &mut impl Write, node: &Node) -> io::Result<()> {
    if node.properties.is_empty() {
        return Ok(());
    }

    writeln!(output, "  properties:")?;
    for property in &node.properties {
        writeln!(output, "    {}: {}", property.key, property.value)?;
    }
    Ok(())
}

fn print_relationships(
    output: &mut impl Write,
    graph: &StorageGraph,
    node: &Node,
) -> io::Result<()> {
    let edges = graph.related_edges(&node.id);
    if edges.is_empty() {
        return Ok(());
    }

    writeln!(output, "  relationships:")?;
    for edge in edges {
        let direction = if edge.from == node.id { "out" } else { "in" };
        let other_id = if edge.from == node.id {
            &edge.to
        } else {
            &edge.from
        };
        let other_name = graph
            .nodes
            .iter()
            .find(|candidate| &candidate.id == other_id)
            .map(|candidate| candidate.name.as_str())
            .unwrap_or(other_id.0.as_str());

        writeln!(
            output,
            "    {direction} {} {} ({})",
            edge.relationship, other_id.0, other_name
        )?;
    }

    Ok(())
}

fn print_plan(output: &mut impl Write, plan: &Plan) -> io::Result<()> {
    writeln!(
        output,
        "Plan: {} actions, {} offline required, {} destructive, {} potential data loss, {} unsupported",
        plan.summary.action_count,
        plan.summary.offline_required_count,
        plan.summary.destructive_count,
        plan.summary.potential_data_loss_count,
        plan.summary.unsupported_count
    )?;

    for action in &plan.actions {
        writeln!(
            output,
            "- {:?} {:?}: {}",
            action.risk, action.operation, action.description
        )?;

        if let Some(advice) = &action.advice {
            writeln!(output, "  advice: {}", advice.summary)?;
            for alternative in &advice.alternatives {
                writeln!(output, "  alternative: {alternative}")?;
            }
        }
    }

    if let Some(comparison) = &plan.topology_comparison {
        print_topology_comparison(output, comparison)?;
    }

    Ok(())
}

fn print_topology_comparison(
    output: &mut impl Write,
    comparison: &TopologyComparison,
) -> io::Result<()> {
    writeln!(
        output,
        "Topology comparison: {} actions, {} matched, {} missing, {} size notes, {} type conflicts, {} already satisfied",
        comparison.summary.action_count,
        comparison.summary.matched_count,
        comparison.summary.missing_count,
        comparison.summary.size_diagnostic_count,
        comparison.summary.type_conflict_count,
        comparison.summary.already_satisfied_count
    )?;

    for diagnostic in &comparison.diagnostics {
        let level = match diagnostic.level {
            TopologyDiagnosticLevel::Info => "info",
            TopologyDiagnosticLevel::Warning => "warning",
        };
        writeln!(
            output,
            "  {level}: {:?} {}: {}",
            diagnostic.kind, diagnostic.action_id, diagnostic.message
        )?;
    }

    Ok(())
}

fn print_execution_report(
    output: &mut impl Write,
    report: &ExecutionReport,
    execute: bool,
) -> io::Result<()> {
    writeln!(
        output,
        "Apply policy: {} allowed, {} blocked",
        report.apply.allowed_count, report.apply.blocked_count
    )?;
    writeln!(output, "mode: {:?}", report.apply.policy.mode)?;
    writeln!(output, "status: {:?}", report.status)?;
    writeln!(output, "execute requested: {execute}")?;
    if let Some(comparison) = &report.topology_comparison {
        print_topology_comparison(output, comparison)?;
    }

    if report.apply.blocked.is_empty() {
        writeln!(output, "No policy blocks detected.")?;
        for message in &report.messages {
            writeln!(output, "{message}")?;
        }
        if !report.command_plan.is_empty() {
            writeln!(
                output,
                "Command summary: {} steps, {} commands, {} mutating, {} manual review, {} ready, {} need size, {} need implementation, {} manual only",
                report.command_summary.step_count,
                report.command_summary.command_count,
                report.command_summary.mutating_count,
                report.command_summary.manual_review_count,
                report.command_summary.ready_count,
                report.command_summary.needs_desired_size_count,
                report.command_summary.needs_domain_implementation_count,
                report.command_summary.manual_only_count
            )?;
            writeln!(output, "Command plan:")?;
            for step in &report.command_plan {
                writeln!(
                    output,
                    "- {:?} {:?} {}",
                    step.risk, step.operation, step.action_id
                )?;
                if step.requires_manual_review {
                    writeln!(output, "  manual review required")?;
                }
                for command in &step.commands {
                    let mutation = if command.mutates {
                        "mutating"
                    } else {
                        "read-only"
                    };
                    writeln!(output, "  {mutation}: {}", command.argv.join(" "))?;
                    writeln!(output, "    readiness: {:?}", command.readiness)?;
                    if !command.unresolved_inputs.is_empty() {
                        writeln!(
                            output,
                            "    unresolved: {}",
                            command.unresolved_inputs.join(", ")
                        )?;
                    }
                    writeln!(output, "    {}", command.note)?;
                }
                for note in &step.notes {
                    writeln!(output, "  note: {note}")?;
                }
            }
        }
        if !report.execution_results.is_empty() {
            writeln!(
                output,
                "Execution results: {} command(s)",
                report.execution_results.len()
            )?;
            for result in &report.execution_results {
                let status = if result.success { "ok" } else { "failed" };
                writeln!(
                    output,
                    "- {:?} {} {}",
                    result.phase,
                    status,
                    result.argv.join(" ")
                )?;
                if let Some(status_code) = result.status_code {
                    writeln!(output, "  exit: {status_code}")?;
                }
                if !result.stdout.is_empty() {
                    writeln!(output, "  stdout: {}", result.stdout.trim_end())?;
                }
                if !result.stderr.is_empty() {
                    writeln!(output, "  stderr: {}", result.stderr.trim_end())?;
                }
            }
        }
        if !report.verification_plan.is_empty() {
            writeln!(
                output,
                "Verification summary: {} steps, {} read-only commands, {} checks",
                report.verification_summary.step_count,
                report.verification_summary.command_count,
                report.verification_summary.check_count
            )?;
            writeln!(output, "Verification plan:")?;
            for step in &report.verification_plan {
                writeln!(
                    output,
                    "- {:?} {:?} {}",
                    step.risk, step.operation, step.action_id
                )?;
                for command in &step.commands {
                    writeln!(output, "  read-only: {}", command.argv.join(" "))?;
                    writeln!(output, "    {}", command.note)?;
                }
                for check in &step.checks {
                    writeln!(output, "  check: {check}")?;
                }
            }
        }
    } else {
        writeln!(
            output,
            "Blocked summary: {} offline required, {} destructive, {} potential data loss, {} unsupported",
            report.apply.blocked_summary.offline_required_count,
            report.apply.blocked_summary.destructive_count,
            report.apply.blocked_summary.potential_data_loss_count,
            report.apply.blocked_summary.unsupported_count
        )?;
        writeln!(output, "Blocked actions:")?;
        for blocked in &report.apply.blocked {
            writeln!(
                output,
                "- {:?} {:?} {}: {}",
                blocked.risk, blocked.operation, blocked.id, blocked.reason
            )?;
        }
    }

    Ok(())
}

fn is_device_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::PhysicalDisk
            | NodeKind::Partition
            | NodeKind::LuksContainer
            | NodeKind::DeviceMapper
            | NodeKind::LvmLogicalVolume
            | NodeKind::LvmPhysicalVolume
            | NodeKind::LvmVolumeGroup
            | NodeKind::LvmThinPool
            | NodeKind::LvmSnapshot
            | NodeKind::LvmCache
            | NodeKind::VdoVolume
            | NodeKind::MdRaid
            | NodeKind::Zvol
            | NodeKind::CacheDevice
            | NodeKind::MultipathDevice
            | NodeKind::NvmeNamespace
            | NodeKind::LoopDevice
            | NodeKind::BackingFile
            | NodeKind::Swap
    )
}

fn is_partition_node(node: &Node) -> bool {
    node.kind == NodeKind::Partition
}

fn is_filesystem_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::Filesystem
            | NodeKind::BtrfsFilesystem
            | NodeKind::BtrfsSubvolume
            | NodeKind::BtrfsSnapshot
            | NodeKind::BtrfsQgroup
            | NodeKind::ZfsDataset
            | NodeKind::ZfsSnapshot
            | NodeKind::NfsExport
    )
}

fn is_volume_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::LvmVolumeGroup
            | NodeKind::LvmLogicalVolume
            | NodeKind::LvmThinPool
            | NodeKind::LvmSnapshot
            | NodeKind::LvmCache
            | NodeKind::VdoVolume
            | NodeKind::MdRaid
            | NodeKind::BtrfsFilesystem
            | NodeKind::BtrfsSubvolume
            | NodeKind::BtrfsSnapshot
            | NodeKind::BtrfsQgroup
            | NodeKind::ZfsPool
            | NodeKind::ZfsDataset
            | NodeKind::ZfsSnapshot
            | NodeKind::Zvol
            | NodeKind::Lun
            | NodeKind::NfsExport
    )
}

fn is_pool_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::LvmVolumeGroup
            | NodeKind::LvmThinPool
            | NodeKind::BtrfsFilesystem
            | NodeKind::BtrfsQgroup
            | NodeKind::ZfsPool
            | NodeKind::ZfsVdev
            | NodeKind::MdRaid
    )
}

fn is_snapshot_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::LvmSnapshot | NodeKind::BtrfsSnapshot | NodeKind::ZfsSnapshot
    )
}

fn is_mapping_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::LuksContainer
            | NodeKind::DeviceMapper
            | NodeKind::LvmLogicalVolume
            | NodeKind::LvmThinPool
            | NodeKind::LvmSnapshot
            | NodeKind::LvmCache
            | NodeKind::VdoVolume
            | NodeKind::MdRaid
            | NodeKind::MultipathDevice
            | NodeKind::LoopDevice
            | NodeKind::CacheDevice
    )
}

fn is_mount_node(node: &Node) -> bool {
    matches!(node.kind, NodeKind::Mountpoint | NodeKind::NfsMount)
}

fn is_network_storage_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::IscsiSession
            | NodeKind::IscsiTarget
            | NodeKind::Lun
            | NodeKind::NfsExport
            | NodeKind::NfsMount
    )
}

fn has_capacity_or_usage(node: &Node) -> bool {
    node.size_bytes.is_some()
        || node.usage.as_ref().is_some_and(|usage| {
            usage.used_bytes.is_some()
                || usage.free_bytes.is_some()
                || usage.allocated_bytes.is_some()
        })
}

fn usage_details(node: &Node) -> String {
    const DETAIL_KEYS: &[(&str, &str)] = &[
        ("model", "model"),
        ("vendor", "vendor"),
        ("transport", "transport"),
        ("rotational", "rotational"),
        ("nvme.model", "nvme-model"),
        ("nvme.firmware", "firmware"),
        ("nvme.index", "ns-index"),
        ("nvme.maximum-lba", "max-lba"),
        ("nvme.sector-size", "sector-size"),
        ("lsblk.type", "lsblk-type"),
        ("filesystem.type", "fstype"),
        ("blkid.type", "blkid-type"),
        ("blkid.version", "version"),
        ("blkid.block-size", "blkid-block-size"),
        ("blkid.usage", "usage"),
        ("blkid.uuid-sub", "uuid-sub"),
        ("blkid.partlabel", "partlabel"),
        ("partition.table", "ptable"),
        ("partition.number", "partno"),
        ("partition.start", "start"),
        ("partition.end", "end"),
        ("partition.type", "type"),
        ("partition.name", "part-name"),
        ("partition.flags", "flags"),
        ("partition.disk-flags", "disk-flags"),
        ("parted.transport", "parted-transport"),
        ("parted.logical-sector-size", "logical-sector"),
        ("parted.physical-sector-size", "physical-sector"),
        ("swap.active", "swap-active"),
        ("swap.type", "swap-type"),
        ("swap.priority", "swap-priority"),
        ("loop.backing", "loop-backing"),
        ("loop.back-file", "back-file"),
        ("loop.major-minor", "major-minor"),
        ("loop.offset", "offset"),
        ("loop.sizelimit", "sizelimit"),
        ("loop.logical-sector-size", "logical-sector"),
        ("loop.autoclear", "autoclear"),
        ("loop.read-only", "ro"),
        ("loop.direct-io", "dio"),
        ("udev.symlink", "udev-link"),
        ("udev.devpath", "udev-devpath"),
        ("udev.id-fs-type", "udev-fstype"),
        ("udev.id-bus", "udev-bus"),
        ("lvm.data-percent", "data"),
        ("lvm.metadata-percent", "metadata"),
        ("lvm.attr", "attr"),
        ("lvm.origin", "origin"),
        ("lvm.pool", "pool"),
        ("lvm.extent-size", "extent"),
        ("lvm.pv-count", "pvs"),
        ("lvm.lv-count", "lvs"),
        ("btrfs.qgroup-id", "qgroup"),
        ("btrfs.mount-target", "mount-target"),
        ("btrfs.device-id", "device-id"),
        ("btrfs.id", "subvol-id"),
        ("btrfs.parent-uuid", "parent-uuid"),
        ("btrfs.max-referenced", "max-rfer"),
        ("btrfs.max-exclusive", "max-excl"),
        ("vdo.storage-device", "backing"),
        ("vdo.logical-size", "logical"),
        ("vdo.physical-size", "physical"),
        ("vdo.stats-size", "stats-size"),
        ("vdo.stats-used", "stats-used"),
        ("vdo.stats-available", "stats-free"),
        ("vdo.use-percent", "vdo-use"),
        ("vdo.space-saving-percent", "saving"),
        ("vdo.operating-mode", "mode"),
        ("vdo.recovery-percentage", "recovery"),
        ("vdo.write-policy", "write-policy"),
        ("vdo.compression", "compression"),
        ("vdo.deduplication", "deduplication"),
        ("vdo.overhead-blocks-used", "overhead-blocks"),
        ("dm.uuid", "dm-uuid"),
        ("dm.open-count", "open"),
        ("dm.segments", "segments"),
        ("dm.events", "events"),
        ("cryptsetup.active", "active"),
        ("cryptsetup.in-use", "in-use"),
        ("cryptsetup.cipher", "cipher"),
        ("cryptsetup.mode", "mode"),
        ("cryptsetup.sector-size", "sector-size"),
        ("cryptsetup.sector-count", "sectors"),
        ("cryptsetup.luks-version", "luks"),
        ("cryptsetup.luks-epoch", "epoch"),
        ("cryptsetup.luks-metadata-area", "metadata-area"),
        ("cryptsetup.luks-keyslots-area", "keyslots-area"),
        ("cryptsetup.luks-subsystem", "subsystem"),
        ("cryptsetup.luks-flags", "flags"),
        ("cryptsetup.luks-keyslot-count", "keyslots"),
        ("cryptsetup.luks-token-count", "tokens"),
        ("cryptsetup.luks-keyslots", "keyslot-ids"),
        ("cryptsetup.luks-tokens", "token-ids"),
        ("cryptsetup.luks-data-cipher", "data-cipher"),
        ("cryptsetup.luks-data-offset", "data-offset"),
        ("cryptsetup.luks-data-length", "data-length"),
        ("cryptsetup.luks-data-sector", "data-sector"),
        ("multipath.dm", "dm"),
        ("multipath.wwid", "wwid"),
        ("multipath.vendor-product", "vendor"),
        ("multipath.size", "size"),
        ("multipath.features", "features"),
        ("multipath.hwhandler", "handler"),
        ("multipath.write-protect", "wp"),
        ("multipath.host-path", "host-path"),
        ("major-minor", "major-minor"),
        ("multipath.path-state", "path-state"),
        ("md.level", "level"),
        ("md.state", "state"),
        ("md.raid-devices", "raid-devices"),
        ("md.total-devices", "total-devices"),
        ("md.member-state", "member-state"),
        ("iscsi.portal", "portal"),
        ("iscsi.persistent-portal", "persistent-portal"),
        ("iscsi.connection-state", "connection-state"),
        ("iscsi.attached-disk", "attached-disk"),
        ("nfs.source", "source"),
        ("nfs.server", "server"),
        ("nfs.export", "export"),
        ("nfs.vers", "vers"),
        ("nfs.proto", "proto"),
        ("nfs.sec", "sec"),
        ("nfs.clientaddr", "clientaddr"),
        ("nfs.addr", "addr"),
        ("nfs.rsize", "rsize"),
        ("nfs.wsize", "wsize"),
        ("ext.state", "ext-state"),
        ("ext.errors-behavior", "errors"),
        ("ext.block-count", "blocks"),
        ("ext.free-blocks", "free-blocks"),
        ("ext.block-size", "block-size"),
        ("ext.inode-count", "inodes"),
        ("ext.free-inodes", "free-inodes"),
        ("ext.features", "features"),
        ("ext.mount-count", "mount-count"),
        ("ext.last-checked", "last-checked"),
        ("ext.lifetime-writes", "lifetime-writes"),
        ("ext.journal-size", "journal-size"),
        ("exfat.guid", "guid"),
        ("exfat.volume-serial", "serial"),
        ("exfat.volume-length-sectors", "sectors"),
        ("exfat.cluster-count", "clusters"),
        ("exfat.free-clusters", "free-clusters"),
        ("exfat.bytes-per-sector", "sector-bytes"),
        ("exfat.sectors-per-cluster", "sectors-per-cluster"),
        ("bcache.role", "role"),
        ("bcache.kind", "kind"),
        ("bcache.set-uuid", "set-uuid"),
        ("bcache.label", "label"),
        ("bcache.state", "state"),
        ("bcache.cache-mode", "cache-mode"),
        ("bcache.cache-replacement-policy", "replacement"),
        ("bcache.dirty-data", "dirty"),
        ("bcache.readahead", "readahead"),
        ("bcache.sequential-cutoff", "sequential-cutoff"),
        ("bcache.writeback-percent", "writeback-percent"),
        ("bcache.writeback-rate", "writeback-rate"),
        ("zfs.health", "health"),
        ("zfs.state", "state"),
        ("zfs.vdev-role", "vdev-role"),
        ("zfs.vdev-state", "vdev-state"),
        ("zfs.read-errors", "read-errors"),
        ("zfs.write-errors", "write-errors"),
        ("zfs.checksum-errors", "checksum-errors"),
        ("zfs.origin", "origin"),
        ("zfs.userrefs", "userrefs"),
        ("zfs.compression", "compression"),
        ("zfs.quota", "quota"),
        ("zfs.reservation", "reservation"),
        ("zfs.encryption", "encryption"),
        ("zfs.keystatus", "keystatus"),
        ("zfs.volsize", "volsize"),
        ("xfs.data.blocks", "xfs-blocks"),
        ("xfs.data.bsize", "xfs-bsize"),
        ("xfs.meta-data.reflink", "reflink"),
        ("xfs.meta-data.bigtime", "bigtime"),
        ("xfs.log.blocks", "log-blocks"),
    ];

    let details = DETAIL_KEYS
        .iter()
        .filter_map(|(key, label)| {
            property_value(node, key).map(|value| format!("{label}={value}"))
        })
        .collect::<Vec<_>>();

    if details.is_empty() {
        "-".to_string()
    } else {
        details.join(" ")
    }
}

fn mount_details(node: &Node) -> String {
    const DETAIL_KEYS: &[(&str, &str)] = &[
        ("mount.source", "source"),
        ("mount.read-only", "ro"),
        ("mount.read-write", "rw"),
        ("mount.bind", "bind"),
        ("mount.propagation", "propagation"),
        ("mount.propagation.id", "propagation-id"),
        ("tmpfs.size", "tmpfs-size"),
        ("tmpfs.mode", "mode"),
        ("tmpfs.uid", "uid"),
        ("tmpfs.gid", "gid"),
        ("tmpfs.nr-inodes", "nr-inodes"),
        ("overlay.lowerdir", "lowerdir"),
        ("overlay.upperdir", "upperdir"),
        ("overlay.workdir", "workdir"),
        ("overlay.index", "index"),
    ];

    let details = DETAIL_KEYS
        .iter()
        .filter_map(|(key, label)| {
            property_value(node, key).map(|value| format!("{label}={value}"))
        })
        .collect::<Vec<_>>();

    if details.is_empty() {
        "-".to_string()
    } else {
        details.join(" ")
    }
}

fn backing_count(graph: &StorageGraph, node: &Node) -> usize {
    graph
        .edges
        .iter()
        .filter(|edge| {
            edge.to == node.id
                && matches!(
                    edge.relationship,
                    disk_nix_model::Relationship::Backs
                        | disk_nix_model::Relationship::DependsOn
                        | disk_nix_model::Relationship::MemberOf
                )
        })
        .count()
}

fn snapshot_source<'a>(graph: &'a StorageGraph, node: &Node) -> Option<&'a str> {
    graph
        .edges
        .iter()
        .find(|edge| {
            edge.from == node.id && edge.relationship == disk_nix_model::Relationship::SnapshotOf
        })
        .and_then(|edge| graph.nodes.iter().find(|candidate| candidate.id == edge.to))
        .map(|source| source.name.as_str())
}

fn property_value<'a>(node: &'a Node, key: &str) -> Option<&'a str> {
    node.properties
        .iter()
        .find(|property| property.key == key)
        .map(|property| property.value.as_str())
}

fn usage_percent(node: &Node) -> String {
    let Some(usage) = &node.usage else {
        return "-".to_string();
    };
    let Some(used) = usage.used_bytes else {
        return "-".to_string();
    };
    let capacity = node
        .size_bytes
        .or(usage.allocated_bytes)
        .or_else(|| usage.free_bytes.map(|free| used.saturating_add(free)));
    let Some(capacity) = capacity else {
        return "-".to_string();
    };
    if capacity == 0 {
        return "-".to_string();
    }

    format!("{:.1}%", (used as f64 / capacity as f64) * 100.0)
}

fn human_bytes(value: Option<u64>) -> String {
    let Some(bytes) = value else {
        return "-".to_string();
    };

    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut size = bytes as f64;
    let mut unit = UNITS[0];
    for next_unit in UNITS.iter().skip(1) {
        if size < 1024.0 {
            break;
        }
        size /= 1024.0;
        unit = next_unit;
    }

    if unit == "B" {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {unit}")
    }
}

#[cfg(test)]
mod tests {
    use disk_nix_model::{Edge, Identity, Node, NodeKind, Relationship, StorageGraph, Usage};

    use super::{
        confirmation_file_accepts, is_device_node, is_mapping_node, is_network_storage_node,
        is_partition_node, is_pool_node, is_snapshot_node, mount_details, print_devices,
        print_filesystems, print_mappings, print_mounts, print_network_storage, print_partitions,
        print_pools, print_snapshots, print_usage, print_volumes, snapshot_source, usage_details,
        usage_percent,
    };

    #[test]
    fn confirmation_file_accepts_exact_token_line() {
        assert!(confirmation_file_accepts("disk-nix confirm\n"));
        assert!(confirmation_file_accepts("# reviewed\ndisk-nix confirm\n"));
        assert!(confirmation_file_accepts("  disk-nix confirm  \n"));
    }

    #[test]
    fn confirmation_file_rejects_partial_or_different_tokens() {
        assert!(!confirmation_file_accepts(""));
        assert!(!confirmation_file_accepts("disk-nix"));
        assert!(!confirmation_file_accepts("disk-nix confirm now"));
        assert!(!confirmation_file_accepts("prefix disk-nix confirm"));
    }

    #[test]
    fn focused_view_predicates_cover_storage_domains() {
        assert!(is_partition_node(&Node::new(
            "partition:sda1",
            NodeKind::Partition,
            "sda1"
        )));
        assert!(is_pool_node(&Node::new(
            "zpool:tank",
            NodeKind::ZfsPool,
            "tank"
        )));
        assert!(is_pool_node(&Node::new(
            "vg:root",
            NodeKind::LvmVolumeGroup,
            "root"
        )));
        assert!(is_snapshot_node(&Node::new(
            "snapshot:tank/home@before",
            NodeKind::ZfsSnapshot,
            "tank/home@before"
        )));
        assert!(is_network_storage_node(&Node::new(
            "lun:iqn.example:0",
            NodeKind::Lun,
            "iqn.example:0"
        )));
        assert!(is_network_storage_node(&Node::new(
            "nfs:server:/export",
            NodeKind::NfsExport,
            "server:/export"
        )));
        assert!(is_device_node(&Node::new(
            "file:/var/lib/images/root.img",
            NodeKind::BackingFile,
            "/var/lib/images/root.img"
        )));
        assert!(is_mapping_node(&Node::new(
            "block:/dev/loop0",
            NodeKind::LoopDevice,
            "/dev/loop0"
        )));
    }

    #[test]
    fn snapshot_source_follows_snapshot_relationships() {
        let mut graph = StorageGraph::empty();
        graph.add_node(Node::new(
            "dataset:tank/home",
            NodeKind::ZfsDataset,
            "tank/home",
        ));
        graph.add_node(Node::new(
            "snapshot:tank/home@before",
            NodeKind::ZfsSnapshot,
            "tank/home@before",
        ));
        graph.add_edge(Edge::new(
            "snapshot:tank/home@before",
            "dataset:tank/home",
            Relationship::SnapshotOf,
        ));

        let snapshot = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::ZfsSnapshot)
            .expect("snapshot exists");
        assert_eq!(snapshot_source(&graph, snapshot), Some("tank/home"));
    }

    #[test]
    fn devices_table_includes_probe_metadata_details() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("block:/dev/nvme0n1", NodeKind::PhysicalDisk, "/dev/nvme0n1")
                .with_path("/dev/nvme0n1")
                .with_size_bytes(1_000_000_000_000)
                .with_property("model", "FastDisk")
                .with_property("vendor", "Acme")
                .with_property("transport", "nvme")
                .with_property("rotational", "false")
                .with_property("nvme.model", "Example NVMe")
                .with_property("nvme.firmware", "1.0")
                .with_property("nvme.index", "0")
                .with_property("nvme.maximum-lba", "1953125")
                .with_property("nvme.sector-size", "512")
                .with_property("partition.table", "gpt")
                .with_property("udev.symlink", "disk/by-id/nvme-Acme_FastDisk"),
        );
        graph.add_node(
            Node::new(
                "block:/dev/nvme0n1p1",
                NodeKind::Partition,
                "/dev/nvme0n1p1",
            )
            .with_path("/dev/nvme0n1p1")
            .with_property("lsblk.type", "part")
            .with_property("filesystem.type", "vfat")
            .with_property("partition.number", "1")
            .with_property("udev.id-fs-type", "vfat"),
        );
        graph.add_node(
            Node::new("block:/dev/loop0", NodeKind::LoopDevice, "/dev/loop0")
                .with_path("/dev/loop0")
                .with_property("lsblk.type", "loop")
                .with_property("loop.back-file", "/var/lib/images/root.img")
                .with_property("loop.offset", "1048576")
                .with_property("loop.autoclear", "true")
                .with_property("loop.direct-io", "true"),
        );
        graph.add_node(
            Node::new(
                "file:/var/lib/images/root.img",
                NodeKind::BackingFile,
                "/var/lib/images/root.img",
            )
            .with_path("/var/lib/images/root.img")
            .with_property("loop.backing", "true"),
        );
        graph.add_node(
            Node::new("swap:/dev/zram0", NodeKind::Swap, "/dev/zram0")
                .with_path("/dev/zram0")
                .with_property("swap.active", "true")
                .with_property("swap.type", "partition")
                .with_property("swap.priority", "100"),
        );
        graph.add_node(
            Node::new("block:/dev/sda1", NodeKind::Partition, "/dev/sda1")
                .with_path("/dev/sda1")
                .with_property("md.member-state", "active sync"),
        );
        graph.add_node(
            Node::new("block:/dev/sdb", NodeKind::PhysicalDisk, "/dev/sdb")
                .with_path("/dev/sdb")
                .with_property("multipath.host-path", "2:0:0:1")
                .with_property("major-minor", "8:16")
                .with_property("multipath.path-state", "active ready running"),
        );

        let mut output = Vec::new();
        print_devices(&mut output, &graph).expect("devices table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("DETAILS"));
        assert!(
            output
                .contains("model=FastDisk vendor=Acme transport=nvme rotational=false nvme-model=Example NVMe firmware=1.0 ns-index=0 max-lba=1953125 sector-size=512 ptable=gpt")
        );
        assert!(output.contains("udev-link=disk/by-id/nvme-Acme_FastDisk"));
        assert!(output.contains("lsblk-type=part fstype=vfat partno=1 udev-fstype=vfat"));
        assert!(output.contains(
            "lsblk-type=loop back-file=/var/lib/images/root.img offset=1048576 autoclear=true dio=true"
        ));
        assert!(output.contains("loop-backing=true"));
        assert!(output.contains("swap-active=true swap-type=partition swap-priority=100"));
        assert!(output.contains("member-state=active sync"));
        assert!(
            output.contains("host-path=2:0:0:1 major-minor=8:16 path-state=active ready running")
        );
        assert!(output.contains("/var/lib/images/root.img"));
    }

    #[test]
    fn partitions_table_includes_geometry_metadata_details() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "block:/dev/nvme0n1p1",
                NodeKind::Partition,
                "/dev/nvme0n1p1",
            )
            .with_path("/dev/nvme0n1p1")
            .with_size_bytes(536_870_912)
            .with_identity(Identity {
                partuuid: Some("1111-2222".to_string()),
                ..Default::default()
            })
            .with_property("partition.number", "1")
            .with_property("partition.start", "1049kB")
            .with_property("partition.end", "538MB")
            .with_property("partition.type", "fat32")
            .with_property("partition.name", "ESP")
            .with_property("partition.flags", "boot, esp")
            .with_property("filesystem.type", "vfat")
            .with_property("blkid.type", "vfat")
            .with_property("blkid.version", "FAT32")
            .with_property("blkid.block-size", "512")
            .with_property("blkid.usage", "filesystem")
            .with_property("blkid.partlabel", "EFI System Partition"),
        );

        let mut output = Vec::new();
        print_partitions(&mut output, &graph).expect("partitions table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("DETAILS"));
        assert!(output.contains("1111-2222"));
        assert!(output.contains(
            "fstype=vfat blkid-type=vfat version=FAT32 blkid-block-size=512 usage=filesystem partlabel=EFI System Partition partno=1 start=1049kB end=538MB type=fat32 part-name=ESP flags=boot, esp"
        ));
    }

    #[test]
    fn usage_percent_prefers_size_then_allocated_then_used_plus_free() {
        let sized = Node::new("filesystem:root", NodeKind::Filesystem, "/")
            .with_size_bytes(100)
            .with_usage(Usage {
                used_bytes: Some(25),
                free_bytes: Some(75),
                allocated_bytes: Some(50),
            });
        assert_eq!(usage_percent(&sized), "25.0%");

        let allocated =
            Node::new("btrfs:data", NodeKind::BtrfsFilesystem, "data").with_usage(Usage {
                used_bytes: Some(25),
                free_bytes: None,
                allocated_bytes: Some(50),
            });
        assert_eq!(usage_percent(&allocated), "50.0%");

        let used_free =
            Node::new("swap:/dev/sda3", NodeKind::Swap, "/dev/sda3").with_usage(Usage {
                used_bytes: Some(25),
                free_bytes: Some(75),
                allocated_bytes: None,
            });
        assert_eq!(usage_percent(&used_free), "25.0%");
    }

    #[test]
    fn usage_details_surfaces_storage_metadata() {
        let lv = Node::new("lv:vg/thin", NodeKind::LvmLogicalVolume, "vg/thin")
            .with_size_bytes(100)
            .with_usage(Usage {
                used_bytes: Some(25),
                free_bytes: Some(75),
                allocated_bytes: None,
            })
            .with_property("lvm.data-percent", "12.50")
            .with_property("lvm.metadata-percent", "3.00");
        assert_eq!(usage_details(&lv), "data=12.50 metadata=3.00");

        let pool = Node::new("zpool:tank", NodeKind::ZfsPool, "tank")
            .with_size_bytes(100)
            .with_property("zfs.health", "ONLINE");
        assert_eq!(usage_details(&pool), "health=ONLINE");

        let snapshot = Node::new(
            "zfs-snapshot:tank/home@daily",
            NodeKind::ZfsSnapshot,
            "tank/home@daily",
        )
        .with_property("zfs.userrefs", "2");
        assert_eq!(usage_details(&snapshot), "userrefs=2");

        let dataset = Node::new("zfs-dataset:tank/home", NodeKind::ZfsDataset, "tank/home")
            .with_property("zfs.compression", "zstd")
            .with_property("zfs.encryption", "aes-256-gcm")
            .with_property("zfs.keystatus", "available");
        assert_eq!(
            usage_details(&dataset),
            "compression=zstd encryption=aes-256-gcm keystatus=available"
        );

        let xfs = Node::new("mount:/", NodeKind::Mountpoint, "/")
            .with_property("xfs.data.blocks", "262144")
            .with_property("xfs.data.bsize", "4096")
            .with_property("xfs.meta-data.reflink", "1")
            .with_property("xfs.meta-data.bigtime", "1");
        assert_eq!(
            usage_details(&xfs),
            "xfs-blocks=262144 xfs-bsize=4096 reflink=1 bigtime=1"
        );

        let ext = Node::new("fs:/dev/sda2", NodeKind::Filesystem, "ext4")
            .with_property("filesystem.type", "ext4")
            .with_property("blkid.version", "1.0")
            .with_property("blkid.block-size", "4096")
            .with_property("blkid.usage", "filesystem")
            .with_property("blkid.uuid-sub", "subvol-uuid")
            .with_property("ext.state", "clean")
            .with_property("ext.errors-behavior", "Continue")
            .with_property("ext.block-count", "122096646")
            .with_property("ext.free-blocks", "73328197")
            .with_property("ext.block-size", "4096")
            .with_property("ext.inode-count", "30531584")
            .with_property("ext.free-inodes", "27187554")
            .with_property("ext.features", "has_journal extent metadata_csum")
            .with_property("ext.mount-count", "12")
            .with_property("ext.last-checked", "Mon Jan 01 00:00:00 2024")
            .with_property("ext.lifetime-writes", "189 GB")
            .with_property("ext.journal-size", "1024M");
        assert_eq!(
            usage_details(&ext),
            "fstype=ext4 version=1.0 blkid-block-size=4096 usage=filesystem uuid-sub=subvol-uuid ext-state=clean errors=Continue blocks=122096646 free-blocks=73328197 block-size=4096 inodes=30531584 free-inodes=27187554 features=has_journal extent metadata_csum mount-count=12 last-checked=Mon Jan 01 00:00:00 2024 lifetime-writes=189 GB journal-size=1024M"
        );

        let exfat = Node::new("fs:/dev/sdb1", NodeKind::Filesystem, "exfat")
            .with_property("exfat.guid", "01234567-89ab-cdef-0123-456789abcdef")
            .with_property("exfat.volume-serial", "0x6eef953b")
            .with_property("exfat.cluster-count", "49984")
            .with_property("exfat.free-clusters", "1024")
            .with_property("exfat.bytes-per-sector", "512")
            .with_property("exfat.sectors-per-cluster", "64");
        assert_eq!(
            usage_details(&exfat),
            "guid=01234567-89ab-cdef-0123-456789abcdef serial=0x6eef953b clusters=49984 free-clusters=1024 sector-bytes=512 sectors-per-cluster=64"
        );

        let bcache = Node::new("block:/dev/bcache0", NodeKind::CacheDevice, "bcache0")
            .with_property("bcache.role", "backing")
            .with_property("bcache.kind", "cache-set")
            .with_property("bcache.set-uuid", "cache-set-uuid")
            .with_property("bcache.label", "fast-cache")
            .with_property("bcache.state", "clean")
            .with_property("bcache.cache-mode", "writeback")
            .with_property("bcache.cache-replacement-policy", "lru")
            .with_property("bcache.dirty-data", "64.0M")
            .with_property("bcache.readahead", "0")
            .with_property("bcache.sequential-cutoff", "4.0M")
            .with_property("bcache.writeback-percent", "10")
            .with_property("bcache.writeback-rate", "1.0M/sec");
        assert_eq!(
            usage_details(&bcache),
            "role=backing kind=cache-set set-uuid=cache-set-uuid label=fast-cache state=clean cache-mode=writeback replacement=lru dirty=64.0M readahead=0 sequential-cutoff=4.0M writeback-percent=10 writeback-rate=1.0M/sec"
        );

        let swap = Node::new("swap:/dev/zram0", NodeKind::Swap, "/dev/zram0")
            .with_property("swap.active", "true")
            .with_property("swap.type", "partition")
            .with_property("swap.priority", "100");
        assert_eq!(
            usage_details(&swap),
            "swap-active=true swap-type=partition swap-priority=100"
        );

        let loop_device = Node::new("block:/dev/loop0", NodeKind::LoopDevice, "/dev/loop0")
            .with_property("loop.back-file", "/var/lib/images/root.img")
            .with_property("loop.major-minor", "7:0")
            .with_property("loop.offset", "1048576")
            .with_property("loop.sizelimit", "1073741824")
            .with_property("loop.logical-sector-size", "512")
            .with_property("loop.autoclear", "true")
            .with_property("loop.read-only", "false")
            .with_property("loop.direct-io", "true");
        assert_eq!(
            usage_details(&loop_device),
            "back-file=/var/lib/images/root.img major-minor=7:0 offset=1048576 sizelimit=1073741824 logical-sector=512 autoclear=true ro=false dio=true"
        );

        let nvme = Node::new(
            "block:/dev/nvme0n1",
            NodeKind::NvmeNamespace,
            "/dev/nvme0n1",
        )
        .with_property("nvme.model", "Example NVMe")
        .with_property("nvme.firmware", "1.0")
        .with_property("nvme.index", "0")
        .with_property("nvme.maximum-lba", "1953125")
        .with_property("nvme.sector-size", "512");
        assert_eq!(
            usage_details(&nvme),
            "nvme-model=Example NVMe firmware=1.0 ns-index=0 max-lba=1953125 sector-size=512"
        );
    }

    #[test]
    fn usage_table_includes_details_column() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
                .with_size_bytes(100)
                .with_usage(Usage {
                    used_bytes: Some(50),
                    free_bytes: Some(50),
                    allocated_bytes: None,
                })
                .with_property("vdo.storage-device", "/dev/sdb")
                .with_property("vdo.logical-size", "100G")
                .with_property("vdo.physical-size", "50G")
                .with_property("vdo.use-percent", "50%")
                .with_property("vdo.space-saving-percent", "20%")
                .with_property("vdo.operating-mode", "normal")
                .with_property("vdo.write-policy", "sync"),
        );

        let mut output = Vec::new();
        print_usage(&mut output, &graph).expect("usage table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("DETAILS"));
        assert!(output.contains("backing=/dev/sdb logical=100G physical=50G"));
        assert!(output.contains("vdo-use=50% saving=20% mode=normal write-policy=sync"));
    }

    #[test]
    fn filesystems_table_includes_metadata_details() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("fs-source:/dev/mapper/vg-root", NodeKind::Filesystem, "xfs")
                .with_usage(Usage {
                    used_bytes: Some(512),
                    free_bytes: Some(512),
                    allocated_bytes: None,
                })
                .with_property("xfs.data.blocks", "262144")
                .with_property("xfs.meta-data.reflink", "1"),
        );

        let mut output = Vec::new();
        print_filesystems(&mut output, &graph).expect("filesystems table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("DETAILS"));
        assert!(output.contains("xfs-blocks=262144 reflink=1"));
    }

    #[test]
    fn volumes_table_includes_domain_metadata_details() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("lvm-lv:vg/root-snap", NodeKind::LvmSnapshot, "vg/root-snap")
                .with_property("lvm.origin", "root")
                .with_property("lvm.pool", "thinpool")
                .with_property("lvm.data-percent", "12.50"),
        );
        graph.add_node(
            Node::new("md:/dev/md/root", NodeKind::MdRaid, "/dev/md/root")
                .with_property("md.level", "raid1")
                .with_property("md.state", "clean")
                .with_property("md.raid-devices", "2"),
        );
        graph.add_node(
            Node::new("iscsi-lun:iqn.example:0", NodeKind::Lun, "0")
                .with_property("iscsi.attached-disk", "sdb"),
        );
        graph.add_node(
            Node::new(
                "nfs-export:storage.example:/export/home",
                NodeKind::NfsExport,
                "storage.example:/export/home",
            )
            .with_property("nfs.server", "storage.example")
            .with_property("nfs.export", "/export/home"),
        );

        let mut output = Vec::new();
        print_volumes(&mut output, &graph).expect("volumes table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("DETAILS"));
        assert!(output.contains("data=12.50 origin=root pool=thinpool"));
        assert!(output.contains("level=raid1 state=clean raid-devices=2"));
        assert!(output.contains("attached-disk=sdb"));
        assert!(output.contains("server=storage.example export=/export/home"));
    }

    #[test]
    fn network_storage_table_includes_domain_metadata_details() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("iscsi-session:1", NodeKind::IscsiSession, "iscsi-session:1")
                .with_property("iscsi.portal", "10.0.0.10:3260,1")
                .with_property("iscsi.persistent-portal", "10.0.0.11:3260,1")
                .with_property("iscsi.connection-state", "LOGGED IN"),
        );
        graph.add_node(Node::new(
            "iscsi-target:iqn.2026-06.example:storage",
            NodeKind::IscsiTarget,
            "iqn.2026-06.example:storage",
        ));
        graph.add_node(
            Node::new(
                "iscsi-lun:iqn.2026-06.example:storage:0",
                NodeKind::Lun,
                "0",
            )
            .with_size_bytes(1_073_741_824)
            .with_property("iscsi.attached-disk", "sdb"),
        );
        graph.add_node(
            Node::new(
                "nfs-export:storage.example:/export/home",
                NodeKind::NfsExport,
                "storage.example:/export/home",
            )
            .with_property("nfs.server", "storage.example")
            .with_property("nfs.export", "/export/home"),
        );
        graph.add_node(
            Node::new("mount:/home", NodeKind::NfsMount, "/home")
                .with_property("nfs.source", "storage.example:/export/home")
                .with_property("nfs.server", "storage.example")
                .with_property("nfs.export", "/export/home")
                .with_property("nfs.vers", "4.2")
                .with_property("nfs.proto", "tcp")
                .with_property("nfs.sec", "sys")
                .with_property("nfs.clientaddr", "10.0.0.20")
                .with_property("nfs.addr", "10.0.0.10")
                .with_property("nfs.rsize", "1048576")
                .with_property("nfs.wsize", "1048576"),
        );

        let mut output = Vec::new();
        print_network_storage(&mut output, &graph).expect("network storage table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("DETAILS"));
        assert!(output.contains("portal=10.0.0.10:3260,1"));
        assert!(output.contains("persistent-portal=10.0.0.11:3260,1"));
        assert!(output.contains("connection-state=LOGGED IN"));
        assert!(output.contains("attached-disk=sdb"));
        assert!(output.contains("server=storage.example export=/export/home"));
        assert!(output.contains(
            "source=storage.example:/export/home server=storage.example export=/export/home vers=4.2"
        ));
        assert!(output.contains("proto=tcp sec=sys clientaddr=10.0.0.20 addr=10.0.0.10"));
        assert!(output.contains("rsize=1048576 wsize=1048576"));
    }

    #[test]
    fn snapshots_table_includes_domain_metadata_details() {
        let mut graph = StorageGraph::empty();
        graph.add_node(Node::new(
            "dataset:tank/home",
            NodeKind::ZfsDataset,
            "tank/home",
        ));
        graph.add_node(
            Node::new(
                "zfs-snapshot:tank/home@daily",
                NodeKind::ZfsSnapshot,
                "tank/home@daily",
            )
            .with_size_bytes(1_073_741_824)
            .with_property("zfs.userrefs", "2")
            .with_property("zfs.compression", "zstd")
            .with_property("zfs.encryption", "aes-256-gcm")
            .with_property("zfs.keystatus", "available"),
        );
        graph.add_edge(Edge::new(
            "zfs-snapshot:tank/home@daily",
            "dataset:tank/home",
            Relationship::SnapshotOf,
        ));
        graph.add_node(
            Node::new("lvm-lv:vg/root-snap", NodeKind::LvmSnapshot, "vg/root-snap")
                .with_property("lvm.origin", "root")
                .with_property("lvm.pool", "thinpool")
                .with_property("lvm.data-percent", "12.50"),
        );
        graph.add_node(
            Node::new(
                "btrfs-subvolume:fs:@/.snapshots/1/snapshot",
                NodeKind::BtrfsSnapshot,
                "@/.snapshots/1/snapshot",
            )
            .with_property("btrfs.id", "257")
            .with_property("btrfs.parent-uuid", "subvol-root"),
        );

        let mut output = Vec::new();
        print_snapshots(&mut output, &graph).expect("snapshots table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("DETAILS"));
        assert!(output.contains("tank/home"));
        assert!(
            output
                .contains("userrefs=2 compression=zstd encryption=aes-256-gcm keystatus=available")
        );
        assert!(output.contains("data=12.50 origin=root pool=thinpool"));
        assert!(output.contains("subvol-id=257 parent-uuid=subvol-root"));
    }

    #[test]
    fn pools_table_includes_domain_metadata_details() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("zpool:tank", NodeKind::ZfsPool, "tank")
                .with_property("zfs.health", "ONLINE")
                .with_property("zfs.state", "ONLINE"),
        );
        graph.add_node(
            Node::new(
                "zfs-vdev:tank:cache0",
                NodeKind::ZfsVdev,
                "/dev/disk/by-id/cache0",
            )
            .with_property("zfs.vdev-role", "cache")
            .with_property("zfs.vdev-state", "ONLINE")
            .with_property("zfs.read-errors", "0")
            .with_property("zfs.write-errors", "1")
            .with_property("zfs.checksum-errors", "2"),
        );
        graph.add_node(
            Node::new("lvm-vg:vg0", NodeKind::LvmVolumeGroup, "vg0")
                .with_property("lvm.extent-size", "4.00m")
                .with_property("lvm.pv-count", "2")
                .with_property("lvm.lv-count", "8"),
        );
        graph.add_node(
            Node::new("btrfs-qgroup:0/257", NodeKind::BtrfsQgroup, "0/257")
                .with_property("btrfs.qgroup-id", "0/257")
                .with_property("btrfs.max-referenced", "25GiB"),
        );
        graph.add_node(
            Node::new("md:/dev/md/root", NodeKind::MdRaid, "/dev/md/root")
                .with_property("md.level", "raid1")
                .with_property("md.state", "clean"),
        );

        let mut output = Vec::new();
        print_pools(&mut output, &graph).expect("pools table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("DETAILS"));
        assert!(output.contains("health=ONLINE state=ONLINE"));
        assert!(output.contains(
            "vdev-role=cache vdev-state=ONLINE read-errors=0 write-errors=1 checksum-errors=2"
        ));
        assert!(output.contains("extent=4.00m pvs=2 lvs=8"));
        assert!(output.contains("qgroup=0/257 max-rfer=25GiB"));
        assert!(output.contains("level=raid1 state=clean"));
    }

    #[test]
    fn mappings_table_includes_domain_metadata_details() {
        let mut graph = StorageGraph::empty();
        graph.add_node(Node::new(
            "block:/dev/nvme0n1p2",
            NodeKind::Partition,
            "/dev/nvme0n1p2",
        ));
        graph.add_node(
            Node::new(
                "block:/dev/mapper/cryptroot",
                NodeKind::LuksContainer,
                "cryptroot",
            )
            .with_path("/dev/mapper/cryptroot")
            .with_property("dm.uuid", "CRYPT-LUKS2-crypt-uuid-cryptroot")
            .with_property("dm.open-count", "1")
            .with_property("cryptsetup.active", "true")
            .with_property("cryptsetup.in-use", "true")
            .with_property("cryptsetup.cipher", "aes-xts-plain64")
            .with_property("cryptsetup.luks-version", "2")
            .with_property("cryptsetup.luks-epoch", "7")
            .with_property("cryptsetup.luks-metadata-area", "16384 [bytes]")
            .with_property("cryptsetup.luks-keyslots-area", "16744448 [bytes]")
            .with_property("cryptsetup.luks-subsystem", "(no subsystem)")
            .with_property("cryptsetup.luks-flags", "allow-discards")
            .with_property("cryptsetup.luks-keyslot-count", "2")
            .with_property("cryptsetup.luks-token-count", "1")
            .with_property("cryptsetup.luks-keyslots", "0,1")
            .with_property("cryptsetup.luks-tokens", "0")
            .with_property("cryptsetup.luks-data-cipher", "aes-xts-plain64")
            .with_property("cryptsetup.luks-data-offset", "32768 [bytes]")
            .with_property("cryptsetup.luks-data-length", "(whole device)")
            .with_property("cryptsetup.luks-data-sector", "4096 [bytes]"),
        );
        graph.add_edge(Edge::new(
            "block:/dev/nvme0n1p2",
            "block:/dev/mapper/cryptroot",
            Relationship::Backs,
        ));
        graph.add_node(
            Node::new("multipath:mpatha", NodeKind::MultipathDevice, "mpatha")
                .with_path("/dev/mapper/mpatha")
                .with_property("multipath.dm", "dm-2")
                .with_property("multipath.wwid", "3600508b400105e210000900000490000")
                .with_property("multipath.vendor-product", "IBM,2145")
                .with_property("multipath.size", "100G")
                .with_property("multipath.features", "'1 queue_if_no_path'")
                .with_property("multipath.write-protect", "rw"),
        );
        graph.add_node(
            Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
                .with_property("vdo.storage-device", "/dev/sdb")
                .with_property("vdo.logical-size", "1T")
                .with_property("vdo.physical-size", "250G")
                .with_property("vdo.operating-mode", "normal")
                .with_property("vdo.write-policy", "sync")
                .with_property("vdo.compression", "enabled")
                .with_property("vdo.deduplication", "disabled"),
        );
        graph.add_node(
            Node::new("block:/dev/bcache0", NodeKind::CacheDevice, "bcache0")
                .with_path("/dev/bcache0")
                .with_property("bcache.role", "backing")
                .with_property("bcache.kind", "cache-set")
                .with_property("bcache.label", "fast-cache")
                .with_property("bcache.state", "clean")
                .with_property("bcache.cache-mode", "writeback")
                .with_property("bcache.readahead", "0")
                .with_property("bcache.sequential-cutoff", "4.0M")
                .with_property("bcache.writeback-rate", "1.0M/sec"),
        );

        let mut output = Vec::new();
        print_mappings(&mut output, &graph).expect("mappings table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("DETAILS"));
        assert!(output.contains("cryptroot"));
        assert!(output.contains("       1 dm-uuid=CRYPT-LUKS2-crypt-uuid-cryptroot"));
        assert!(
            output.contains(
                "active=true in-use=true cipher=aes-xts-plain64 luks=2 epoch=7 metadata-area=16384 [bytes] keyslots-area=16744448 [bytes] subsystem=(no subsystem) flags=allow-discards keyslots=2 tokens=1 keyslot-ids=0,1 token-ids=0 data-cipher=aes-xts-plain64 data-offset=32768 [bytes] data-length=(whole device) data-sector=4096 [bytes]"
            )
        );
        assert!(
            output.contains(
                "dm=dm-2 wwid=3600508b400105e210000900000490000 vendor=IBM,2145 size=100G"
            )
        );
        assert!(
            output.contains(
                "backing=/dev/sdb logical=1T physical=250G mode=normal write-policy=sync compression=enabled deduplication=disabled"
            )
        );
        assert!(output.contains(
            "role=backing kind=cache-set label=fast-cache state=clean cache-mode=writeback readahead=0 sequential-cutoff=4.0M writeback-rate=1.0M/sec"
        ));
    }

    #[test]
    fn mounts_table_includes_source_and_pseudo_mount_details() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("mount:/run", NodeKind::Mountpoint, "/run")
                .with_property("filesystem.type", "tmpfs")
                .with_property("mount.source", "tmpfs")
                .with_property("mount.read-write", "true")
                .with_property("tmpfs.size", "64M")
                .with_property("tmpfs.mode", "0755"),
        );
        graph.add_node(
            Node::new("mount:/srv/cache", NodeKind::Mountpoint, "/srv/cache")
                .with_property("filesystem.type", "none")
                .with_property("mount.source", "/var/cache/disk-nix")
                .with_property("mount.bind", "true"),
        );
        graph.add_node(
            Node::new("mount:/merged", NodeKind::Mountpoint, "/merged")
                .with_property("filesystem.type", "overlay")
                .with_property("mount.source", "overlay")
                .with_property("overlay.lowerdir", "/lower")
                .with_property("overlay.upperdir", "/upper")
                .with_property("overlay.workdir", "/work")
                .with_property("overlay.index", "off"),
        );

        assert_eq!(
            mount_details(
                graph
                    .nodes
                    .iter()
                    .find(|node| node.name == "/run")
                    .expect("tmpfs mount fixture should exist")
            ),
            "source=tmpfs rw=true tmpfs-size=64M mode=0755"
        );

        let mut output = Vec::new();
        print_mounts(&mut output, &graph).expect("mount table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("DETAILS"));
        assert!(output.contains("source=/var/cache/disk-nix bind=true"));
        assert!(
            output
                .contains("source=overlay lowerdir=/lower upperdir=/upper workdir=/work index=off")
        );
    }
}
