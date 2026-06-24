#![recursion_limit = "512"]

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
    /// List complex filesystem objects across Btrfs, bcachefs, and ZFS.
    ComplexFilesystems {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List ZFS pools, vdevs, datasets, snapshots, and zvols.
    Zfs {
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
    /// List discovered disk encryption mappings and header metadata.
    Encryption {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered cache layers and cache device metadata.
    Cache {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List LVM physical volumes, volume groups, logical volumes, and segments.
    Lvm {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered VDO volumes and LVM VDO segment metadata.
    Vdo {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered multipath maps and path metadata.
    Multipath {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered NVMe namespaces and namespace metadata.
    Nvme {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered MD RAID arrays and member metadata.
    Raid {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered loop devices and backing file mappings.
    Loop {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered active swap devices and files.
    Swap {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered iSCSI sessions, targets, and LUNs.
    Iscsi {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List discovered NFS exports and client mounts.
    Nfs {
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
        Command::ComplexFilesystems { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_complex_filesystem_node)?;
            } else {
                print_complex_filesystems(output, &graph)?;
            }
            Ok(())
        }
        Command::Zfs { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_zfs_node)?;
            } else {
                print_zfs(output, &graph)?;
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
        Command::Encryption { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_encryption_node)?;
            } else {
                print_encryption(output, &graph)?;
            }
            Ok(())
        }
        Command::Cache { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_cache_node)?;
            } else {
                print_cache(output, &graph)?;
            }
            Ok(())
        }
        Command::Lvm { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_lvm_node)?;
            } else {
                print_lvm(output, &graph)?;
            }
            Ok(())
        }
        Command::Vdo { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_vdo_node)?;
            } else {
                print_vdo(output, &graph)?;
            }
            Ok(())
        }
        Command::Multipath { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_multipath_node)?;
            } else {
                print_multipath(output, &graph)?;
            }
            Ok(())
        }
        Command::Nvme { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_nvme_node)?;
            } else {
                print_nvme(output, &graph)?;
            }
            Ok(())
        }
        Command::Raid { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_raid_node)?;
            } else {
                print_raid(output, &graph)?;
            }
            Ok(())
        }
        Command::Loop { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_loop_node)?;
            } else {
                print_loop(output, &graph)?;
            }
            Ok(())
        }
        Command::Swap { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_swap_node)?;
            } else {
                print_swap(output, &graph)?;
            }
            Ok(())
        }
        Command::Iscsi { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_iscsi_node)?;
            } else {
                print_iscsi(output, &graph)?;
            }
            Ok(())
        }
        Command::Nfs { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_nfs_node)?;
            } else {
                print_nfs(output, &graph)?;
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
                    "neededForBoot": { "type": "boolean" },
                    "destroy": { "type": "boolean" },
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
                    "metadata": {
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
                    "metadata": {
                        "type": "object",
                        "additionalProperties": true
                    },
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
                    "renameTo": { "type": "string" },
                    "renameTarget": { "type": "string" },
                    "newName": { "type": "string" },
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
                    "key-slot": { "type": ["string", "number"] },
                    "slot": { "type": ["string", "number"] },
                    "keyFile": { "type": "string" },
                    "key-file": { "type": "string" },
                    "currentKeyFile": { "type": "string" },
                    "newKeyFile": { "type": "string" },
                    "new-key-file": { "type": "string" },
                    "tokenId": { "type": ["string", "number"] },
                    "token-id": { "type": ["string", "number"] },
                    "token": { "type": ["string", "number"] },
                    "tokenFile": { "type": "string" },
                    "token-file": { "type": "string" },
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
                    "readOnly": { "type": "boolean" },
                    "readonly": { "type": "boolean" },
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
                    "path": { "type": "string" },
                    "snapshotPath": { "type": "string" },
                    "snapshot-path": { "type": "string" },
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

fn print_complex_filesystems(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:>12} {:>12} {:>7} {:>7} DETAILS",
        "KIND", "NAME", "SIZE", "USED", "FREE", "USE%", "BACKING"
    )?;
    for node in graph
        .nodes
        .iter()
        .filter(|node| is_complex_filesystem_node(node))
    {
        let usage = node.usage.as_ref();
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:>12} {:>12} {:>7} {:>7} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            human_bytes(usage.and_then(|usage| usage.used_bytes)),
            human_bytes(usage.and_then(|usage| usage.free_bytes)),
            usage_percent(node),
            backing_count(graph, node),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_zfs(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:>12} {:>12} {:<12} {:<24} {:>8} DETAILS",
        "KIND", "NAME", "SIZE", "USED", "FREE", "HEALTH", "ORIGIN", "CHILDREN"
    )?;
    for node in graph.nodes.iter().filter(|node| is_zfs_node(node)) {
        let usage = node.usage.as_ref();
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:>12} {:>12} {:<12} {:<24} {:>8} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            human_bytes(usage.and_then(|usage| usage.used_bytes)),
            human_bytes(usage.and_then(|usage| usage.free_bytes)),
            property_value(node, "zfs.health")
                .or_else(|| property_value(node, "zfs.state"))
                .or_else(|| property_value(node, "zfs.vdev-state"))
                .unwrap_or("-"),
            property_value(node, "zfs.origin").unwrap_or("-"),
            zfs_child_count(graph, node),
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

fn print_encryption(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:<12} {:<10} {:<10} DETAILS",
        "KIND", "NAME", "CIPHER", "KEYSLOTS", "TOKENS"
    )?;
    for node in graph.nodes.iter().filter(|node| is_encryption_node(node)) {
        writeln!(
            output,
            "{:<22} {:<38} {:<12} {:<10} {:<10} {}",
            node.kind,
            node.name,
            property_value(node, "cryptsetup.cipher")
                .or_else(|| property_value(node, "cryptsetup.luks-data-cipher"))
                .unwrap_or("-"),
            property_value(node, "cryptsetup.luks-keyslot-count").unwrap_or("-"),
            property_value(node, "cryptsetup.luks-token-count").unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_cache(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:<14} {:<14} {:<14} DETAILS",
        "KIND", "NAME", "MODE", "POLICY", "DIRTY"
    )?;
    for node in graph.nodes.iter().filter(|node| is_cache_node(node)) {
        writeln!(
            output,
            "{:<22} {:<38} {:<14} {:<14} {:<14} {}",
            node.kind,
            node.name,
            property_value(node, "bcache.cache-mode")
                .or_else(|| property_value(node, "lvm.cache-mode"))
                .or_else(|| property_value(node, "lvm.segment-cache-mode"))
                .unwrap_or("-"),
            property_value(node, "bcache.cache-replacement-policy")
                .or_else(|| property_value(node, "lvm.cache-policy"))
                .or_else(|| property_value(node, "lvm.segment-cache-policy"))
                .unwrap_or("-"),
            property_value(node, "bcache.dirty-data")
                .or_else(|| property_value(node, "lvm.writecache-writeback-blocks"))
                .unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_lvm(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:<12} {:<12} {:<12} {:>7} DETAILS",
        "KIND", "NAME", "SIZE", "DATA%", "META%", "ACTIVE", "BACKING"
    )?;
    for node in graph.nodes.iter().filter(|node| is_lvm_node(node)) {
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:<12} {:<12} {:<12} {:>7} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            property_value(node, "lvm.data-percent").unwrap_or("-"),
            property_value(node, "lvm.metadata-percent").unwrap_or("-"),
            property_value(node, "lvm.active").unwrap_or("-"),
            backing_count(graph, node),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_vdo(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:>12} {:<12} {:<12} DETAILS",
        "KIND", "NAME", "LOGICAL", "PHYSICAL", "MODE", "WRITE"
    )?;
    for node in graph.nodes.iter().filter(|node| is_vdo_node(node)) {
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:>12} {:<12} {:<12} {}",
            node.kind,
            node.name,
            property_value(node, "vdo.logical-size")
                .or_else(|| property_value(node, "lvm.vdo-logical-size"))
                .unwrap_or("-"),
            property_value(node, "vdo.physical-size")
                .or_else(|| property_value(node, "lvm.vdo-physical-size"))
                .unwrap_or("-"),
            property_value(node, "vdo.operating-mode")
                .or_else(|| property_value(node, "lvm.vdo-operating-mode"))
                .unwrap_or("-"),
            property_value(node, "vdo.write-policy")
                .or_else(|| property_value(node, "lvm.vdo-write-policy"))
                .unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_multipath(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<32} {:<28} {:>5} {:<12} {:<20} DETAILS",
        "KIND", "NAME", "WWID", "PATHS", "GROUP", "PATH-STATE"
    )?;
    for node in graph.nodes.iter().filter(|node| is_multipath_node(node)) {
        writeln!(
            output,
            "{:<22} {:<32} {:<28} {:>5} {:<12} {:<20} {}",
            node.kind,
            node.name,
            property_value(node, "multipath.wwid").unwrap_or("-"),
            backing_count(graph, node),
            property_value(node, "multipath.group-status").unwrap_or("-"),
            property_value(node, "multipath.path-state").unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_nvme(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<24} {:>12} {:>12} {:>7} {:<20} {:<18} DETAILS",
        "KIND", "NAME", "SIZE", "USED", "USE%", "SERIAL", "CONTROLLER"
    )?;
    for node in graph.nodes.iter().filter(|node| is_nvme_node(node)) {
        let usage = node.usage.as_ref();
        writeln!(
            output,
            "{:<22} {:<24} {:>12} {:>12} {:>7} {:<20} {:<18} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            human_bytes(usage.and_then(|usage| usage.used_bytes)),
            usage_percent(node),
            node.identity.serial.as_deref().unwrap_or("-"),
            property_value(node, "nvme.controller").unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_raid(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<32} {:>12} {:<10} {:<14} {:>6} {:>6} {:>6} {:>7} DETAILS",
        "KIND", "NAME", "SIZE", "LEVEL", "STATE", "ACTIVE", "FAILED", "SPARE", "MEMBERS"
    )?;
    for node in graph.nodes.iter().filter(|node| is_raid_node(node)) {
        writeln!(
            output,
            "{:<22} {:<32} {:>12} {:<10} {:<14} {:>6} {:>6} {:>6} {:>7} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            property_value(node, "md.level").unwrap_or("-"),
            property_value(node, "md.state")
                .or_else(|| property_value(node, "md.member-state"))
                .unwrap_or("-"),
            property_value(node, "md.active-devices").unwrap_or("-"),
            property_value(node, "md.failed-devices").unwrap_or("-"),
            property_value(node, "md.spare-devices").unwrap_or("-"),
            member_count(graph, node),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_loop(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<28} {:>12} {:<32} {:<10} {:<8} DETAILS",
        "KIND", "NAME", "SIZE", "BACKING", "OFFSET", "RO"
    )?;
    for node in graph.nodes.iter().filter(|node| is_loop_node(node)) {
        writeln!(
            output,
            "{:<22} {:<28} {:>12} {:<32} {:<10} {:<8} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            property_value(node, "loop.back-file").unwrap_or("-"),
            property_value(node, "loop.offset").unwrap_or("-"),
            property_value(node, "loop.read-only").unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_swap(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<32} {:>12} {:>12} {:>12} {:>7} {:<10} {:<8} DETAILS",
        "KIND", "NAME", "SIZE", "USED", "FREE", "USE%", "TYPE", "PRIO"
    )?;
    for node in graph.nodes.iter().filter(|node| is_swap_node(node)) {
        let usage = node.usage.as_ref();
        writeln!(
            output,
            "{:<22} {:<32} {:>12} {:>12} {:>12} {:>7} {:<10} {:<8} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            human_bytes(usage.and_then(|usage| usage.used_bytes)),
            human_bytes(usage.and_then(|usage| usage.free_bytes)),
            usage_percent(node),
            property_value(node, "swap.type").unwrap_or("-"),
            property_value(node, "swap.priority").unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_iscsi(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<48} {:>12} {:<22} {:<14} {:>5} DETAILS",
        "KIND", "NAME", "SIZE", "PORTAL", "STATE", "LUNS"
    )?;
    for node in graph.nodes.iter().filter(|node| is_iscsi_node(node)) {
        writeln!(
            output,
            "{:<22} {:<48} {:>12} {:<22} {:<14} {:>5} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            property_value(node, "iscsi.portal")
                .or_else(|| property_value(node, "iscsi.node-portal"))
                .or_else(|| property_value(node, "iscsi.persistent-portal"))
                .or_else(|| property_value(node, "iscsi.node-persistent-portal"))
                .unwrap_or("-"),
            property_value(node, "iscsi.connection-state").unwrap_or("-"),
            iscsi_lun_count(graph, node),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_nfs(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<40} {:<34} {:<20} {:<22} {:>6} DETAILS",
        "KIND", "NAME", "SOURCE", "SERVER", "EXPORT", "MOUNTS"
    )?;
    for node in graph.nodes.iter().filter(|node| is_nfs_node(node)) {
        writeln!(
            output,
            "{:<22} {:<40} {:<34} {:<20} {:<22} {:>6} {}",
            node.kind,
            node.name,
            property_value(node, "nfs.source").unwrap_or("-"),
            property_value(node, "nfs.server").unwrap_or("-"),
            property_value(node, "nfs.export").unwrap_or("-"),
            nfs_mount_count(graph, node),
            usage_details(node)
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
            | NodeKind::BcachefsDevice
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
            | NodeKind::BcachefsFilesystem
            | NodeKind::ZfsDataset
            | NodeKind::ZfsSnapshot
            | NodeKind::NfsExport
    )
}

fn is_complex_filesystem_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::BtrfsFilesystem
            | NodeKind::BtrfsSubvolume
            | NodeKind::BtrfsSnapshot
            | NodeKind::BtrfsQgroup
            | NodeKind::BcachefsFilesystem
            | NodeKind::BcachefsDevice
            | NodeKind::ZfsPool
            | NodeKind::ZfsVdev
            | NodeKind::ZfsDataset
            | NodeKind::ZfsSnapshot
            | NodeKind::Zvol
    ) || node.properties.iter().any(|property| {
        property.key.starts_with("btrfs.")
            || property.key.starts_with("bcachefs.")
            || property.key.starts_with("zfs.")
    })
}

fn is_zfs_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::ZfsPool
            | NodeKind::ZfsVdev
            | NodeKind::ZfsDataset
            | NodeKind::ZfsSnapshot
            | NodeKind::Zvol
    ) || node
        .properties
        .iter()
        .any(|property| property.key.starts_with("zfs."))
}

fn is_volume_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::LvmVolumeGroup
            | NodeKind::LvmLogicalVolume
            | NodeKind::LvmSegment
            | NodeKind::LvmThinPool
            | NodeKind::LvmSnapshot
            | NodeKind::LvmCache
            | NodeKind::VdoVolume
            | NodeKind::MdRaid
            | NodeKind::BtrfsFilesystem
            | NodeKind::BtrfsSubvolume
            | NodeKind::BtrfsSnapshot
            | NodeKind::BtrfsQgroup
            | NodeKind::BcachefsFilesystem
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
            | NodeKind::BcachefsFilesystem
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
            | NodeKind::LvmSegment
            | NodeKind::LvmThinPool
            | NodeKind::LvmSnapshot
            | NodeKind::LvmCache
            | NodeKind::VdoVolume
            | NodeKind::MdRaid
            | NodeKind::MultipathDevice
            | NodeKind::LoopDevice
            | NodeKind::CacheDevice
            | NodeKind::BcachefsDevice
    )
}

fn is_encryption_node(node: &Node) -> bool {
    node.kind == NodeKind::LuksContainer
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("cryptsetup."))
}

fn is_cache_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::LvmCache | NodeKind::CacheDevice | NodeKind::BcachefsDevice
    ) || node.properties.iter().any(|property| {
        property.key.starts_with("bcache.")
            || property.key.starts_with("bcachefs.device-")
            || property.key == "lvm.cache-mode"
            || property.key == "lvm.cache-policy"
            || property.key == "lvm.kernel-cache-mode"
            || property.key == "lvm.kernel-cache-policy"
            || property.key == "lvm.cache-metadata-format"
            || property.key == "lvm.segment-cache-mode"
            || property.key == "lvm.segment-cache-policy"
            || property.key == "lvm.cache-settings"
            || property.key.starts_with("lvm.writecache-")
            || (property.key == "zfs.vdev-role" && property.value == "cache")
    })
}

fn is_lvm_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::LvmPhysicalVolume
            | NodeKind::LvmVolumeGroup
            | NodeKind::LvmLogicalVolume
            | NodeKind::LvmSegment
            | NodeKind::LvmThinPool
            | NodeKind::LvmSnapshot
            | NodeKind::LvmCache
    ) || node
        .properties
        .iter()
        .any(|property| property.key.starts_with("lvm."))
}

fn is_vdo_node(node: &Node) -> bool {
    node.kind == NodeKind::VdoVolume
        || node.properties.iter().any(|property| {
            property.key.starts_with("vdo.") || property.key.starts_with("lvm.vdo-")
        })
}

fn is_multipath_node(node: &Node) -> bool {
    node.kind == NodeKind::MultipathDevice
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("multipath."))
}

fn is_nvme_node(node: &Node) -> bool {
    node.kind == NodeKind::NvmeNamespace
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("nvme."))
}

fn is_raid_node(node: &Node) -> bool {
    node.kind == NodeKind::MdRaid
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("md."))
}

fn is_loop_node(node: &Node) -> bool {
    matches!(node.kind, NodeKind::LoopDevice | NodeKind::BackingFile)
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("loop."))
}

fn is_swap_node(node: &Node) -> bool {
    node.kind == NodeKind::Swap
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("swap."))
}

fn is_iscsi_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::IscsiSession | NodeKind::IscsiTarget | NodeKind::Lun
    ) || node
        .properties
        .iter()
        .any(|property| property.key.starts_with("iscsi."))
}

fn is_nfs_node(node: &Node) -> bool {
    matches!(node.kind, NodeKind::NfsExport | NodeKind::NfsMount)
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("nfs."))
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
        ("nvme.generic-path", "generic"),
        ("nvme.model", "nvme-model"),
        ("nvme.product", "product"),
        ("nvme.firmware", "firmware"),
        ("nvme.index", "ns-index"),
        ("nvme.namespace", "namespace"),
        ("nvme.subsystem", "subsystem"),
        ("nvme.controller", "controller"),
        ("nvme.address", "address"),
        ("nvme.transport", "transport"),
        ("nvme.controller-id", "controller-id"),
        ("nvme.namespace-capacity", "namespace-capacity"),
        ("nvme.lba-format", "lba-format"),
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
        ("udev.devname", "udev-devname"),
        ("udev.devtype", "udev-devtype"),
        ("udev.id-fs-type", "udev-fstype"),
        ("udev.id-fs-version", "udev-fs-version"),
        ("udev.id-fs-usage", "udev-fs-usage"),
        ("udev.id-fs-uuid", "udev-fs-uuid"),
        ("udev.id-fs-uuid-enc", "udev-fs-uuid-enc"),
        ("udev.id-fs-uuid-sub", "udev-fs-uuid-sub"),
        ("udev.id-fs-label", "udev-label"),
        ("udev.id-fs-label-enc", "udev-label-enc"),
        ("udev.id-fs-label-safe", "udev-label-safe"),
        ("udev.id-bus", "udev-bus"),
        ("udev.id-type", "udev-type"),
        ("udev.id-model", "udev-model"),
        ("udev.id-model-id", "udev-model-id"),
        ("udev.id-vendor", "udev-vendor"),
        ("udev.id-vendor-id", "udev-vendor-id"),
        ("udev.id-revision", "udev-revision"),
        ("udev.id-serial", "udev-serial"),
        ("udev.id-serial-short", "udev-serial-short"),
        ("udev.id-wwn", "udev-wwn"),
        ("udev.id-path", "udev-path"),
        ("udev.id-path-tag", "udev-path-tag"),
        ("udev.id-part-entry-disk", "udev-part-disk"),
        ("udev.id-part-entry-number", "udev-part-number"),
        ("udev.id-part-entry-offset", "udev-part-offset"),
        ("udev.id-part-entry-size", "udev-part-size"),
        ("udev.id-part-entry-scheme", "udev-part-scheme"),
        ("udev.id-part-entry-type", "udev-part-type"),
        ("udev.id-part-entry-name", "udev-part-name"),
        ("udev.id-part-entry-uuid", "udev-part-uuid"),
        ("udev.id-part-entry-flags", "udev-part-flags"),
        ("udev.id-part-table-type", "udev-table-type"),
        ("udev.id-part-table-uuid", "udev-table-uuid"),
        ("udev.dm-name", "dm-name"),
        ("udev.dm-uuid", "dm-uuid"),
        ("udev.dm-vg-name", "dm-vg"),
        ("udev.dm-lv-name", "dm-lv"),
        ("udev.dm-udev-rules-vsn", "dm-rules"),
        ("udev.dm-udev-primary-source-flag", "dm-primary-source"),
        (
            "udev.dm-udev-disable-other-rules-flag",
            "dm-disable-other-rules",
        ),
        ("udev.dm-subsystem-udev-flag0", "dm-subsystem-flag0"),
        ("udev.dm-subsystem-udev-flag1", "dm-subsystem-flag1"),
        ("udev.major", "major"),
        ("udev.minor", "minor"),
        ("udev.subsystem", "subsystem"),
        ("lvm.data-percent", "data"),
        ("lvm.metadata-percent", "metadata"),
        ("lvm.snap-percent", "snap"),
        ("lvm.copy-percent", "copy"),
        ("lvm.sync-percent", "sync"),
        ("lvm.attr", "attr"),
        ("lvm.layout", "layout"),
        ("lvm.segment-type", "segment-type"),
        ("lvm.segment-stripes", "stripes"),
        ("lvm.segment-data-stripes", "data-stripes"),
        ("lvm.reshape-length", "reshape-length"),
        ("lvm.reshape-length-extents", "reshape-extents"),
        ("lvm.data-copies", "data-copies"),
        ("lvm.data-offset", "data-offset"),
        ("lvm.new-data-offset", "new-data-offset"),
        ("lvm.parity-chunks", "parity-chunks"),
        ("lvm.stripe-size", "stripe-size"),
        ("lvm.region-size", "region-size"),
        ("lvm.segment-start", "segment-start"),
        ("lvm.segment-start-extent", "segment-start-pe"),
        ("lvm.segment-size", "segment-size"),
        ("lvm.segment-size-extents", "segment-size-pe"),
        ("lvm.segment-tags", "segment-tags"),
        ("lvm.chunk-size", "chunk-size"),
        ("lvm.thin-count", "thin-count"),
        ("lvm.discards", "discards"),
        ("lvm.zero", "zero"),
        ("lvm.transaction-id", "transaction-id"),
        ("lvm.thin-id", "thin-id"),
        ("lvm.devices", "devices"),
        ("lvm.metadata-devices", "metadata-devices"),
        ("lvm.segment-pe-ranges", "pe-ranges"),
        ("lvm.segment-le-ranges", "le-ranges"),
        ("lvm.segment-metadata-le-ranges", "metadata-le-ranges"),
        ("lvm.segment-monitor", "segment-monitor"),
        ("lvm.cache-metadata-format", "cache-metadata-format"),
        ("lvm.segment-cache-mode", "segment-cache-mode"),
        ("lvm.segment-cache-policy", "segment-cache-policy"),
        ("lvm.cache-settings", "cache-settings"),
        ("lvm.integrity-settings", "integrity-settings"),
        ("lvm.vdo-compression", "vdo-compression"),
        ("lvm.vdo-deduplication", "vdo-deduplication"),
        ("lvm.vdo-minimum-io-size", "vdo-min-io"),
        ("lvm.vdo-block-map-cache-size", "vdo-block-map-cache"),
        ("lvm.vdo-block-map-era-length", "vdo-block-map-era"),
        ("lvm.vdo-use-sparse-index", "vdo-sparse-index"),
        ("lvm.vdo-index-memory-size", "vdo-index-memory"),
        ("lvm.vdo-slab-size", "vdo-slab"),
        ("lvm.vdo-ack-threads", "vdo-ack-threads"),
        ("lvm.vdo-bio-threads", "vdo-bio-threads"),
        ("lvm.vdo-bio-rotation", "vdo-bio-rotation"),
        ("lvm.vdo-cpu-threads", "vdo-cpu-threads"),
        ("lvm.vdo-hash-zone-threads", "vdo-hash-zone-threads"),
        ("lvm.vdo-logical-threads", "vdo-logical-threads"),
        ("lvm.vdo-physical-threads", "vdo-physical-threads"),
        ("lvm.vdo-max-discard", "vdo-max-discard"),
        ("lvm.vdo-header-size", "vdo-header"),
        ("lvm.vdo-use-metadata-hints", "vdo-metadata-hints"),
        ("lvm.vdo-write-policy", "vdo-write-policy"),
        ("lvm.vdo-operating-mode", "vdo-mode"),
        ("lvm.vdo-compression-state", "vdo-compression-state"),
        ("lvm.vdo-index-state", "vdo-index-state"),
        ("lvm.vdo-used-size", "vdo-used"),
        ("lvm.vdo-saving-percent", "vdo-saving"),
        ("lvm.origin", "origin"),
        ("lvm.pool", "pool"),
        ("lvm.pv-format", "pv-format"),
        ("lvm.dev-size", "dev-size"),
        ("lvm.pv-major", "pv-major"),
        ("lvm.pv-minor", "pv-minor"),
        ("lvm.pe-start", "pe-start"),
        ("lvm.pv-attr", "pv-attr"),
        ("lvm.pv-allocatable", "pv-allocatable"),
        ("lvm.pv-exported", "pv-exported"),
        ("lvm.pv-missing", "pv-missing"),
        ("lvm.pv-pe-count", "pv-extents"),
        ("lvm.pv-pe-allocated", "pv-extents-used"),
        ("lvm.pv-tags", "pv-tags"),
        ("lvm.pv-mda-count", "pv-mda-count"),
        ("lvm.pv-mda-used-count", "pv-mda-used"),
        ("lvm.pv-mda-free", "pv-mda-free"),
        ("lvm.pv-mda-size", "pv-mda-size"),
        ("lvm.pv-bootloader-area-start", "pv-ba-start"),
        ("lvm.pv-bootloader-area-size", "pv-ba-size"),
        ("lvm.pv-in-use", "pv-in-use"),
        ("lvm.pv-duplicate", "pv-duplicate"),
        ("lvm.pv-device-id", "pv-device-id"),
        ("lvm.pv-device-id-type", "pv-device-id-type"),
        ("lvm.vg-format", "vg-format"),
        ("lvm.vg-attr", "vg-attr"),
        ("lvm.vg-extendable", "vg-extendable"),
        ("lvm.vg-exported", "vg-exported"),
        ("lvm.vg-autoactivation", "vg-autoactivation"),
        ("lvm.vg-partial", "vg-partial"),
        ("lvm.allocation-policy", "allocation"),
        ("lvm.vg-clustered", "vg-clustered"),
        ("lvm.vg-shared", "vg-shared"),
        ("lvm.vg-system-id", "system-id"),
        ("lvm.vg-lock-type", "lock-type"),
        ("lvm.vg-lock-args", "lock-args"),
        ("lvm.extent-size", "extent"),
        ("lvm.extent-count", "extents"),
        ("lvm.free-count", "free-extents"),
        ("lvm.max-lvs", "max-lvs"),
        ("lvm.max-pvs", "max-pvs"),
        ("lvm.pv-count", "pvs"),
        ("lvm.missing-pv-count", "missing-pvs"),
        ("lvm.lv-count", "lvs"),
        ("lvm.snapshot-count", "snapshots"),
        ("lvm.vg-seqno", "seqno"),
        ("lvm.vg-profile", "vg-profile"),
        ("lvm.vg-mda-count", "vg-mda-count"),
        ("lvm.vg-mda-used-count", "vg-mda-used"),
        ("lvm.vg-mda-free", "vg-mda-free"),
        ("lvm.vg-mda-size", "vg-mda-size"),
        ("lvm.vg-mda-copies", "vg-mda-copies"),
        ("lvm.active", "active"),
        ("lvm.active-locally", "active-local"),
        ("lvm.active-remotely", "active-remote"),
        ("lvm.active-exclusively", "active-exclusive"),
        ("lvm.permissions", "permissions"),
        ("lvm.health", "health"),
        ("lvm.when-full", "when-full"),
        ("lvm.metadata-size", "metadata-size"),
        ("lvm.tags", "tags"),
        ("lvm.dm-path", "dm-path"),
        ("lvm.parent", "parent"),
        ("lvm.read-ahead", "read-ahead"),
        ("lvm.kernel-read-ahead", "kernel-read-ahead"),
        ("lvm.suspended", "suspended"),
        ("lvm.live-table", "live-table"),
        ("lvm.inactive-table", "inactive-table"),
        ("lvm.modules", "modules"),
        ("lvm.host", "host"),
        ("lvm.historical", "historical"),
        ("lvm.kernel-major", "kernel-major"),
        ("lvm.kernel-minor", "kernel-minor"),
        ("lvm.device-open", "device-open"),
        ("lvm.check-needed", "check-needed"),
        ("lvm.role", "role"),
        ("lvm.time", "time"),
        ("lvm.raid-mismatch-count", "raid-mismatches"),
        ("lvm.raid-sync-action", "raid-sync"),
        ("lvm.raid-write-behind", "raid-write-behind"),
        ("lvm.raid-min-recovery-rate", "raid-min-recovery"),
        ("lvm.raid-max-recovery-rate", "raid-max-recovery"),
        ("lvm.raid-integrity-mode", "raid-integrity"),
        ("lvm.raid-integrity-block-size", "raid-integrity-block"),
        ("lvm.raid-integrity-mismatches", "raid-integrity-mismatches"),
        ("lvm.cache-total-blocks", "cache-total"),
        ("lvm.cache-used-blocks", "cache-used"),
        ("lvm.cache-dirty-blocks", "cache-dirty"),
        ("lvm.cache-read-hits", "cache-read-hits"),
        ("lvm.cache-read-misses", "cache-read-misses"),
        ("lvm.cache-write-hits", "cache-write-hits"),
        ("lvm.cache-write-misses", "cache-write-misses"),
        ("lvm.cache-promotions", "cache-promotions"),
        ("lvm.cache-demotions", "cache-demotions"),
        ("lvm.cache-mode", "cache-mode"),
        ("lvm.cache-policy", "cache-policy"),
        ("lvm.kernel-cache-settings", "kernel-cache-settings"),
        ("lvm.kernel-cache-mode", "kernel-cache-mode"),
        ("lvm.kernel-cache-policy", "kernel-cache-policy"),
        ("lvm.kernel-metadata-format", "kernel-metadata-format"),
        ("lvm.kernel-discards", "kernel-discards"),
        ("lvm.writecache-total-blocks", "writecache-total"),
        ("lvm.writecache-free-blocks", "writecache-free"),
        ("lvm.writecache-writeback-blocks", "writecache-writeback"),
        ("lvm.writecache-block-size", "writecache-block-size"),
        ("lvm.writecache-error", "writecache-error"),
        ("btrfs.qgroup-id", "qgroup"),
        ("btrfs.mount-target", "mount-target"),
        ("btrfs.device-id", "device-id"),
        ("btrfs.id", "subvol-id"),
        ("btrfs.generation", "generation"),
        ("btrfs.created-generation", "created-generation"),
        ("btrfs.parent-id", "parent-id"),
        ("btrfs.top-level", "top-level"),
        ("btrfs.parent-uuid", "parent-uuid"),
        ("btrfs.received-uuid", "received-uuid"),
        ("btrfs.data-profile", "data-profile"),
        ("btrfs.data-size", "data-size"),
        ("btrfs.data-used", "data-used"),
        ("btrfs.metadata-profile", "metadata-profile"),
        ("btrfs.metadata-size", "metadata-size"),
        ("btrfs.metadata-used", "metadata-used"),
        ("btrfs.system-profile", "system-profile"),
        ("btrfs.system-size", "system-size"),
        ("btrfs.system-used", "system-used"),
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
        ("vdo.configured-write-policy", "configured-write-policy"),
        ("vdo.index-memory-setting", "index-memory"),
        ("vdo.block-map-cache-size", "block-map-cache"),
        ("vdo.compression", "compression"),
        ("vdo.deduplication", "deduplication"),
        ("vdo.version", "vdo-version"),
        ("vdo.release-version", "vdo-release"),
        ("vdo.data-blocks-used", "data-blocks"),
        ("vdo.overhead-blocks-used", "overhead-blocks"),
        ("vdo.logical-blocks-used", "logical-blocks"),
        ("dm.name", "dm-name"),
        ("dm.uuid", "dm-uuid"),
        ("dm.major", "dm-major"),
        ("dm.minor", "dm-minor"),
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
        ("cryptsetup.luks-keyslot-0-type", "keyslot-0"),
        ("cryptsetup.luks-keyslot-0-priority", "keyslot-0-priority"),
        ("cryptsetup.luks-keyslot-0-cipher", "keyslot-0-cipher"),
        (
            "cryptsetup.luks-keyslot-0-cipher-key",
            "keyslot-0-cipher-key",
        ),
        ("cryptsetup.luks-keyslot-0-pbkdf", "keyslot-0-pbkdf"),
        ("cryptsetup.luks-keyslot-0-time-cost", "keyslot-0-time"),
        ("cryptsetup.luks-keyslot-0-memory", "keyslot-0-memory"),
        ("cryptsetup.luks-keyslot-0-threads", "keyslot-0-threads"),
        ("cryptsetup.luks-keyslot-1-type", "keyslot-1"),
        ("cryptsetup.luks-keyslot-1-priority", "keyslot-1-priority"),
        ("cryptsetup.luks-keyslot-1-cipher", "keyslot-1-cipher"),
        (
            "cryptsetup.luks-keyslot-1-cipher-key",
            "keyslot-1-cipher-key",
        ),
        ("cryptsetup.luks-keyslot-1-pbkdf", "keyslot-1-pbkdf"),
        ("cryptsetup.luks-keyslot-1-time-cost", "keyslot-1-time"),
        ("cryptsetup.luks-keyslot-1-memory", "keyslot-1-memory"),
        ("cryptsetup.luks-keyslot-1-threads", "keyslot-1-threads"),
        ("cryptsetup.luks-token-0-type", "token-0"),
        ("cryptsetup.luks-token-0-keyslot", "token-0-keyslot"),
        ("cryptsetup.luks-token-1-type", "token-1"),
        ("cryptsetup.luks-token-1-keyslot", "token-1-keyslot"),
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
        ("multipath.scsi-host", "scsi-host"),
        ("multipath.scsi-channel", "scsi-channel"),
        ("multipath.scsi-id", "scsi-id"),
        ("multipath.scsi-lun", "scsi-lun"),
        ("major-minor", "major-minor"),
        ("multipath.group-policy", "group-policy"),
        ("multipath.group-prio", "group-prio"),
        ("multipath.group-status", "group-status"),
        ("multipath.dm-state", "dm-state"),
        ("multipath.checker-state", "checker-state"),
        ("multipath.online-state", "online-state"),
        ("multipath.path-state", "path-state"),
        ("md.version", "md-version"),
        ("md.level", "level"),
        ("md.state", "state"),
        ("md.raid-devices", "raid-devices"),
        ("md.total-devices", "total-devices"),
        ("md.array-devices", "array-devices"),
        ("md.active-devices", "active-devices"),
        ("md.working-devices", "working-devices"),
        ("md.failed-devices", "failed-devices"),
        ("md.spare-devices", "spare-devices"),
        ("md.degraded-devices", "degraded-devices"),
        ("md.name", "md-name"),
        ("md.events", "events"),
        ("md.chunk-size", "chunk"),
        ("md.layout", "layout"),
        ("md.consistency-policy", "consistency"),
        ("md.rebuild-status", "rebuild"),
        ("md.resync-status", "resync"),
        ("md.check-status", "check"),
        ("md.intent-bitmap", "bitmap"),
        ("md.creation-time", "created"),
        ("md.update-time", "updated"),
        ("md.scan-metadata", "scan-metadata"),
        ("md.scan-name", "scan-name"),
        ("md.scan-spares", "scan-spares"),
        ("md.scan-devices", "scan-devices"),
        ("md.member-number", "member-number"),
        ("md.member-major", "member-major"),
        ("md.member-minor", "member-minor"),
        ("md.member-raid-device", "member-raid-device"),
        ("md.member-state", "member-state"),
        ("iscsi.portal", "portal"),
        ("iscsi.portal-address", "portal-address"),
        ("iscsi.portal-port", "portal-port"),
        ("iscsi.portal-tpgt", "portal-tpgt"),
        ("iscsi.persistent-portal", "persistent-portal"),
        (
            "iscsi.persistent-portal-address",
            "persistent-portal-address",
        ),
        ("iscsi.persistent-portal-port", "persistent-portal-port"),
        ("iscsi.persistent-portal-tpgt", "persistent-portal-tpgt"),
        ("iscsi.target-portal-group-tag", "tpgt"),
        ("iscsi.connection-state", "connection-state"),
        ("iscsi.session-state", "session-state"),
        ("iscsi.internal-session-state", "internal-session-state"),
        ("iscsi.iface-name", "iface"),
        ("iscsi.iface-transport", "transport"),
        ("iscsi.iface-initiator-name", "initiator"),
        ("iscsi.iface-ip-address", "iface-ip"),
        ("iscsi.iface-netdev", "netdev"),
        ("iscsi.host-number", "host"),
        ("iscsi.host-state", "host-state"),
        ("iscsi.connection-cid", "cid"),
        ("iscsi.connection-detail-state", "connection-detail-state"),
        ("iscsi.connection-local-address", "local-address"),
        ("iscsi.connection-peer-address", "peer-address"),
        ("iscsi.headerdigest", "header-digest"),
        ("iscsi.datadigest", "data-digest"),
        ("iscsi.maxrecvdatasegmentlength", "max-recv-data-segment"),
        ("iscsi.maxxmitdatasegmentlength", "max-xmit-data-segment"),
        ("iscsi.firstburstlength", "first-burst"),
        ("iscsi.maxburstlength", "max-burst"),
        ("iscsi.immediatedata", "immediate-data"),
        ("iscsi.initialr2t", "initial-r2t"),
        ("iscsi.maxoutstandingr2t", "max-outstanding-r2t"),
        ("iscsi.scsi-channel", "scsi-channel"),
        ("iscsi.scsi-id", "scsi-id"),
        ("iscsi.attached-disk", "attached-disk"),
        ("iscsi.attached-disk-state", "attached-disk-state"),
        ("iscsi.node-configured", "configured"),
        ("iscsi.node-portal", "node-portal"),
        ("iscsi.node-portal-address", "node-portal-address"),
        ("iscsi.node-portal-port", "node-portal-port"),
        ("iscsi.node-portal-tpgt", "node-portal-tpgt"),
        ("iscsi.node-persistent-portal", "node-persistent-portal"),
        (
            "iscsi.node-persistent-portal-address",
            "node-persistent-portal-address",
        ),
        (
            "iscsi.node-persistent-portal-port",
            "node-persistent-portal-port",
        ),
        (
            "iscsi.node-persistent-portal-tpgt",
            "node-persistent-portal-tpgt",
        ),
        ("iscsi.node-tpgt", "node-tpgt"),
        ("iscsi.node-iface-name", "node-iface"),
        ("iscsi.node-startup", "startup"),
        ("iscsi.node-leading-login", "leading-login"),
        ("iscsi.node-auth-method", "auth-method"),
        ("iscsi.node-auth-username", "auth-username"),
        ("nfs.source", "source"),
        ("nfs.server", "server"),
        ("nfs.export", "export"),
        ("nfs.export-client", "export-client"),
        ("nfs.exportfs", "exportfs"),
        ("nfs.export-option-rw", "export-rw"),
        ("nfs.export-option-ro", "export-ro"),
        ("nfs.export-option-sync", "export-sync"),
        ("nfs.export-option-async", "export-async"),
        ("nfs.export-option-wdelay", "export-wdelay"),
        ("nfs.export-option-no-wdelay", "export-no-wdelay"),
        ("nfs.export-option-hide", "export-hide"),
        ("nfs.export-option-nohide", "export-nohide"),
        (
            "nfs.export-option-no-subtree-check",
            "export-no-subtree-check",
        ),
        ("nfs.export-option-subtree-check", "export-subtree-check"),
        ("nfs.export-option-sec", "export-sec"),
        ("nfs.export-option-secure", "export-secure"),
        ("nfs.export-option-insecure", "export-insecure"),
        ("nfs.export-option-root-squash", "export-root-squash"),
        ("nfs.export-option-no-root-squash", "export-no-root-squash"),
        ("nfs.export-option-all-squash", "export-all-squash"),
        ("nfs.export-option-no-all-squash", "export-no-all-squash"),
        ("nfs.export-option-fsid", "export-fsid"),
        ("nfs.vers", "vers"),
        ("nfs.proto", "proto"),
        ("nfs.sec", "sec"),
        ("nfs.clientaddr", "clientaddr"),
        ("nfs.addr", "addr"),
        ("nfs.port", "port"),
        ("nfs.mountaddr", "mountaddr"),
        ("nfs.mountvers", "mountvers"),
        ("nfs.mountproto", "mountproto"),
        ("nfs.rsize", "rsize"),
        ("nfs.wsize", "wsize"),
        ("nfs.timeo", "timeo"),
        ("nfs.retrans", "retrans"),
        ("nfs.local-lock", "local-lock"),
        ("nfs.lookupcache", "lookupcache"),
        ("nfs.fsc", "fsc"),
        ("nfs.age", "age"),
        ("nfs.namlen", "namlen"),
        ("nfs.caps", "caps"),
        ("nfs.wtmult", "wtmult"),
        ("nfs.dtsize", "dtsize"),
        ("nfs.bsize", "bsize"),
        ("nfs.flavor", "flavor"),
        ("nfs.pseudoflavor", "pseudoflavor"),
        ("nfs.hard", "hard"),
        ("nfs.soft", "soft"),
        ("nfs.noresvport", "noresvport"),
        ("ext.state", "ext-state"),
        ("ext.errors-behavior", "errors"),
        ("ext.os-type", "os"),
        ("ext.block-count", "blocks"),
        ("ext.reserved-block-count", "reserved-blocks"),
        ("ext.overhead-clusters", "overhead-clusters"),
        ("ext.free-blocks", "free-blocks"),
        ("ext.block-size", "block-size"),
        ("ext.fragment-size", "fragment-size"),
        ("ext.blocks-per-group", "blocks-per-group"),
        ("ext.fragments-per-group", "fragments-per-group"),
        ("ext.inode-count", "inodes"),
        ("ext.free-inodes", "free-inodes"),
        ("ext.inodes-per-group", "inodes-per-group"),
        ("ext.features", "features"),
        ("ext.flags", "flags"),
        ("ext.default-directory-hash", "dir-hash"),
        ("ext.directory-hash-seed", "dir-hash-seed"),
        ("ext.default-mount-options", "default-mount"),
        ("ext.created", "created"),
        ("ext.last-mount-time", "last-mounted"),
        ("ext.last-write-time", "last-written"),
        ("ext.mount-count", "mount-count"),
        ("ext.maximum-mount-count", "max-mount-count"),
        ("ext.last-checked", "last-checked"),
        ("ext.check-interval", "check-interval"),
        ("ext.lifetime-writes", "lifetime-writes"),
        ("ext.reserved-blocks-uid", "reserved-uid"),
        ("ext.reserved-blocks-gid", "reserved-gid"),
        ("ext.first-inode", "first-inode"),
        ("ext.inode-size", "inode-size"),
        ("ext.journal-inode", "journal-inode"),
        ("ext.journal-uuid", "journal-uuid"),
        ("ext.journal-backup", "journal-backup"),
        ("ext.journal-features", "journal-features"),
        ("ext.journal-size", "journal-size"),
        ("ext.checksum-type", "checksum-type"),
        ("ext.checksum", "checksum"),
        ("exfat.guid", "guid"),
        ("exfat.exfatprogs-version", "exfatprogs"),
        ("exfat.volume-serial", "serial"),
        ("exfat.volume-length-sectors", "sectors"),
        ("exfat.fat-offset-sector-offset", "fat-offset"),
        ("exfat.fat-length-sectors", "fat-length"),
        (
            "exfat.cluster-heap-offset-sector-offset",
            "cluster-heap-offset",
        ),
        ("exfat.cluster-count", "clusters"),
        ("exfat.used-clusters", "used-clusters"),
        ("exfat.free-clusters", "free-clusters"),
        ("exfat.root-cluster-cluster-offset", "root-cluster"),
        ("exfat.bytes-per-sector", "sector-bytes"),
        ("exfat.sectors-per-cluster", "sectors-per-cluster"),
        ("exfat.bytes-per-cluster", "cluster-bytes"),
        ("exfat.sector-size-bits", "sector-bits"),
        ("exfat.sector-per-cluster-bits", "cluster-bits"),
        ("ntfs.volume-name", "ntfs-name"),
        ("ntfs.volume-state", "ntfs-state"),
        ("ntfs.volume-serial", "ntfs-serial"),
        ("ntfs.volume-flags", "ntfs-flags"),
        ("ntfs.version", "ntfs-version"),
        ("ntfs.sector-size", "ntfs-sector"),
        ("ntfs.cluster-size", "ntfs-cluster"),
        ("ntfs.volume-size-clusters", "ntfs-clusters"),
        ("ntfs.index-block-size", "ntfs-index-block"),
        ("ntfs.mft-record-size", "ntfs-mft-record"),
        ("f2fs.filesystem-uuid", "f2fs-uuid"),
        ("f2fs.filesystem-volume-name", "f2fs-name"),
        ("f2fs.block-size", "f2fs-block-size"),
        ("f2fs.block-count", "f2fs-blocks"),
        ("f2fs.user-block-count", "f2fs-user-blocks"),
        ("f2fs.valid-block-count", "f2fs-valid-blocks"),
        ("f2fs.total-valid-block-count", "f2fs-total-valid-blocks"),
        ("f2fs.valid-node-count", "f2fs-valid-nodes"),
        ("f2fs.valid-inode-count", "f2fs-valid-inodes"),
        ("f2fs.segment-count", "f2fs-segments"),
        ("f2fs.segment-count-main", "f2fs-main-segments"),
        ("f2fs.segment-count-ckpt", "f2fs-ckpt-segments"),
        ("f2fs.segment-count-sit", "f2fs-sit-segments"),
        ("f2fs.segment-count-nat", "f2fs-nat-segments"),
        ("f2fs.segment-count-ssa", "f2fs-ssa-segments"),
        ("f2fs.overprov-segment-count", "f2fs-overprov"),
        ("f2fs.section-count", "f2fs-sections"),
        ("f2fs.segs-per-sec", "f2fs-segs-per-sec"),
        ("f2fs.secs-per-zone", "f2fs-secs-per-zone"),
        ("f2fs.log-sectorsize", "f2fs-log-sector"),
        ("f2fs.log-sectors-per-block", "f2fs-log-sectors-block"),
        ("f2fs.log-blocksize", "f2fs-log-block"),
        ("f2fs.log-blocks-per-seg", "f2fs-log-blocks-seg"),
        ("f2fs.cp-payload", "f2fs-cp-payload"),
        ("f2fs.version", "f2fs-version"),
        ("f2fs.init-version", "f2fs-init-version"),
        ("f2fs.extension-count", "f2fs-extensions"),
        ("f2fs.hot-ext-count", "f2fs-hot-extensions"),
        ("bcachefs.external-uuid", "bcachefs-uuid"),
        ("bcachefs.internal-uuid", "bcachefs-internal"),
        ("bcachefs.magic-number", "bcachefs-magic"),
        ("bcachefs.device", "bcachefs-super-device"),
        ("bcachefs.member-device", "bcachefs-member"),
        ("bcachefs.mount-target", "bcachefs-mount"),
        ("bcachefs.device-index", "bcachefs-device"),
        ("bcachefs.version", "bcachefs-version"),
        (
            "bcachefs.version-upgrade-complete",
            "bcachefs-upgrade-complete",
        ),
        ("bcachefs.online-reserved", "bcachefs-reserved"),
        ("bcachefs.device-count", "bcachefs-devices"),
        ("bcachefs.data-sb", "bcachefs-sb"),
        ("bcachefs.data-journal", "bcachefs-journal"),
        ("bcachefs.data-btree", "bcachefs-btree"),
        ("bcachefs.data-user", "bcachefs-user"),
        ("bcachefs.data-cached", "bcachefs-cached"),
        ("bcachefs.data-parity", "bcachefs-parity"),
        ("bcachefs.device-label", "bcachefs-label"),
        ("bcachefs.device-state", "bcachefs-state"),
        ("bcachefs.device-free", "bcachefs-device-free"),
        ("bcachefs.device-capacity", "bcachefs-device-capacity"),
        ("bcachefs.device-data-sb", "bcachefs-device-sb"),
        ("bcachefs.device-data-journal", "bcachefs-device-journal"),
        ("bcachefs.device-data-btree", "bcachefs-device-btree"),
        ("bcachefs.device-data-user", "bcachefs-device-user"),
        ("bcachefs.device-data-cached", "bcachefs-device-cached"),
        ("bcache.role", "role"),
        ("bcache.kind", "kind"),
        ("bcache.backing-device", "backing-device"),
        ("bcache.set-uuid", "set-uuid"),
        ("bcache.label", "label"),
        ("bcache.state", "state"),
        ("bcache.running", "running"),
        ("bcache.cache-available-percent", "available-percent"),
        ("bcache.cache-mode", "cache-mode"),
        ("bcache.cache-replacement-policy", "replacement"),
        ("bcache.discard", "discard"),
        ("bcache.dirty-data", "dirty"),
        ("bcache.io-errors", "io-errors"),
        ("bcache.metadata-written", "metadata-written"),
        ("bcache.priority-stats", "priority-stats"),
        ("bcache.readahead", "readahead"),
        ("bcache.sequential-cutoff", "sequential-cutoff"),
        ("bcache.written", "written"),
        ("bcache.writeback-delay", "writeback-delay"),
        ("bcache.writeback-metadata", "writeback-metadata"),
        ("bcache.writeback-percent", "writeback-percent"),
        ("bcache.writeback-rate", "writeback-rate"),
        ("bcache.writeback-running", "writeback-running"),
        ("zfs.health", "health"),
        ("zfs.state", "state"),
        ("zfs.status", "status"),
        ("zfs.action", "action"),
        ("zfs.scan", "scan"),
        ("zfs.errors", "errors"),
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
        ("xfs.meta-data.meta-data", "xfs-source"),
        ("xfs.meta-data.isize", "xfs-isize"),
        ("xfs.meta-data.agcount", "xfs-agcount"),
        ("xfs.meta-data.agsize", "xfs-agsize"),
        ("xfs.meta-data.sectsz", "xfs-sectsz"),
        ("xfs.meta-data.attr", "xfs-attr"),
        ("xfs.meta-data.projid32bit", "xfs-projid32bit"),
        ("xfs.meta-data.crc", "xfs-crc"),
        ("xfs.meta-data.finobt", "xfs-finobt"),
        ("xfs.meta-data.sparse", "xfs-sparse"),
        ("xfs.meta-data.rmapbt", "xfs-rmapbt"),
        ("xfs.data.blocks", "xfs-blocks"),
        ("xfs.data.bsize", "xfs-bsize"),
        ("xfs.data.imaxpct", "xfs-imaxpct"),
        ("xfs.data.sunit", "xfs-sunit"),
        ("xfs.data.swidth", "xfs-swidth"),
        ("xfs.meta-data.reflink", "reflink"),
        ("xfs.meta-data.bigtime", "bigtime"),
        ("xfs.meta-data.inobtcount", "xfs-inobtcount"),
        ("xfs.meta-data.nrext64", "xfs-nrext64"),
        ("xfs.naming.version", "xfs-naming-version"),
        ("xfs.naming.bsize", "xfs-naming-bsize"),
        ("xfs.naming.ascii-ci", "xfs-ascii-ci"),
        ("xfs.naming.ftype", "xfs-ftype"),
        ("xfs.log.type", "xfs-log-type"),
        ("xfs.log.bsize", "xfs-log-bsize"),
        ("xfs.log.blocks", "log-blocks"),
        ("xfs.log.version", "xfs-log-version"),
        ("xfs.log.sectsz", "xfs-log-sectsz"),
        ("xfs.log.sunit", "xfs-log-sunit"),
        ("xfs.log.lazy-count", "xfs-log-lazy-count"),
        ("xfs.realtime.type", "xfs-realtime-type"),
        ("xfs.realtime.extsz", "xfs-realtime-extsz"),
        ("xfs.realtime.blocks", "xfs-realtime-blocks"),
        ("xfs.realtime.rtextents", "xfs-realtime-rtextents"),
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

fn member_count(graph: &StorageGraph, node: &Node) -> usize {
    graph
        .edges
        .iter()
        .filter(|edge| {
            edge.to == node.id && edge.relationship == disk_nix_model::Relationship::MemberOf
        })
        .count()
}

fn iscsi_lun_count(graph: &StorageGraph, node: &Node) -> usize {
    match node.kind {
        NodeKind::IscsiSession => {
            let direct_luns = graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.from == node.id
                        && edge.relationship == disk_nix_model::Relationship::Contains
                        && graph.nodes.iter().any(|candidate| {
                            candidate.id == edge.to && candidate.kind == NodeKind::Lun
                        })
                })
                .count();
            let target_luns = graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.from == node.id
                        && edge.relationship == disk_nix_model::Relationship::ImportedFrom
                })
                .map(|edge| {
                    graph
                        .edges
                        .iter()
                        .filter(|candidate_edge| {
                            candidate_edge.from == edge.to
                                && candidate_edge.relationship
                                    == disk_nix_model::Relationship::Contains
                                && graph.nodes.iter().any(|candidate_node| {
                                    candidate_node.id == candidate_edge.to
                                        && candidate_node.kind == NodeKind::Lun
                                })
                        })
                        .count()
                })
                .sum::<usize>();
            direct_luns + target_luns
        }
        NodeKind::IscsiTarget => graph
            .edges
            .iter()
            .filter(|edge| {
                edge.from == node.id
                    && edge.relationship == disk_nix_model::Relationship::Contains
                    && graph
                        .nodes
                        .iter()
                        .any(|candidate| candidate.id == edge.to && candidate.kind == NodeKind::Lun)
            })
            .count(),
        _ => 0,
    }
}

fn nfs_mount_count(graph: &StorageGraph, node: &Node) -> usize {
    if node.kind != NodeKind::NfsExport {
        return 0;
    }

    graph
        .edges
        .iter()
        .filter(|edge| {
            edge.from == node.id
                && edge.relationship == disk_nix_model::Relationship::MountedAt
                && graph.nodes.iter().any(|candidate| {
                    candidate.id == edge.to && candidate.kind == NodeKind::NfsMount
                })
        })
        .count()
}

fn zfs_child_count(graph: &StorageGraph, node: &Node) -> usize {
    graph
        .edges
        .iter()
        .filter(|edge| {
            edge.from == node.id
                && matches!(
                    edge.relationship,
                    disk_nix_model::Relationship::Contains
                        | disk_nix_model::Relationship::MountedAt
                        | disk_nix_model::Relationship::SnapshotOf
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
        confirmation_file_accepts, is_cache_node, is_complex_filesystem_node, is_device_node,
        is_encryption_node, is_filesystem_node, is_iscsi_node, is_loop_node, is_lvm_node,
        is_mapping_node, is_multipath_node, is_network_storage_node, is_nfs_node, is_nvme_node,
        is_partition_node, is_pool_node, is_raid_node, is_snapshot_node, is_swap_node, is_vdo_node,
        is_volume_node, is_zfs_node, iscsi_lun_count, mount_details, nfs_mount_count, print_cache,
        print_complex_filesystems, print_devices, print_encryption, print_filesystems, print_iscsi,
        print_loop, print_lvm, print_mappings, print_mounts, print_multipath,
        print_network_storage, print_nfs, print_nvme, print_partitions, print_pools, print_raid,
        print_snapshots, print_swap, print_usage, print_vdo, print_volumes, print_zfs,
        snapshot_source, usage_details, usage_percent, zfs_child_count,
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
        assert!(is_lvm_node(&Node::new(
            "lvm-vg:root",
            NodeKind::LvmVolumeGroup,
            "root"
        )));
        assert!(is_lvm_node(
            &Node::new(
                "block:/dev/mapper/vg-root",
                NodeKind::DeviceMapper,
                "vg-root"
            )
            .with_property("lvm.active", "active")
        ));
        let bcachefs = Node::new(
            "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3",
            NodeKind::BcachefsFilesystem,
            "archive",
        );
        assert!(is_filesystem_node(&bcachefs));
        assert!(is_complex_filesystem_node(&bcachefs));
        assert!(is_volume_node(&bcachefs));
        assert!(is_pool_node(&bcachefs));
        assert!(is_complex_filesystem_node(&Node::new(
            "btrfs:/mnt/persist",
            NodeKind::BtrfsFilesystem,
            "/mnt/persist"
        )));
        assert!(is_complex_filesystem_node(&Node::new(
            "zpool:tank",
            NodeKind::ZfsPool,
            "tank"
        )));
        assert!(is_zfs_node(&Node::new(
            "zpool:tank",
            NodeKind::ZfsPool,
            "tank"
        )));
        assert!(is_zfs_node(
            &Node::new("filesystem:tank/home", NodeKind::Filesystem, "tank/home")
                .with_property("zfs.compression", "zstd")
        ));
        assert!(is_complex_filesystem_node(
            &Node::new("filesystem:/data", NodeKind::Filesystem, "/data")
                .with_property("btrfs.data-profile", "single")
        ));
        assert!(is_device_node(&Node::new(
            "bcachefs-device:a2d6fc04-efd0-4e36-aece-2475941d09a3:6",
            NodeKind::BcachefsDevice,
            "sdc"
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
        assert!(is_iscsi_node(&Node::new(
            "iscsi-session:1",
            NodeKind::IscsiSession,
            "iscsi-session:1"
        )));
        assert!(is_iscsi_node(
            &Node::new("block:/dev/sdb", NodeKind::PhysicalDisk, "/dev/sdb")
                .with_property("iscsi.attached-disk", "sdb")
        ));
        assert!(is_network_storage_node(&Node::new(
            "nfs:server:/export",
            NodeKind::NfsExport,
            "server:/export"
        )));
        assert!(is_nfs_node(&Node::new(
            "nfs:server:/export",
            NodeKind::NfsExport,
            "server:/export"
        )));
        assert!(is_nfs_node(
            &Node::new("mount:/home", NodeKind::Mountpoint, "/home")
                .with_property("nfs.source", "server:/export")
        ));
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
        assert!(is_encryption_node(&Node::new(
            "block:/dev/mapper/cryptroot",
            NodeKind::LuksContainer,
            "cryptroot"
        )));
        assert!(is_encryption_node(
            &Node::new("dm:cryptroot", NodeKind::DeviceMapper, "cryptroot")
                .with_property("cryptsetup.active", "true")
        ));
        assert!(is_cache_node(&Node::new(
            "block:/dev/bcache0",
            NodeKind::CacheDevice,
            "bcache0"
        )));
        assert!(is_cache_node(
            &Node::new("lvm-lv:vg/root", NodeKind::LvmLogicalVolume, "vg/root")
                .with_property("lvm.cache-mode", "writeback")
        ));
        assert!(is_cache_node(
            &Node::new(
                "zfs-vdev:tank:cache0",
                NodeKind::ZfsVdev,
                "/dev/disk/by-id/cache0"
            )
            .with_property("zfs.vdev-role", "cache")
        ));
        assert!(is_vdo_node(&Node::new(
            "vdo:archive",
            NodeKind::VdoVolume,
            "archive"
        )));
        assert!(is_vdo_node(
            &Node::new(
                "lvm-seg:vg0/archive:0",
                NodeKind::LvmSegment,
                "vg0/archive:0"
            )
            .with_property("lvm.vdo-write-policy", "auto")
        ));
        assert!(is_multipath_node(&Node::new(
            "multipath:mpatha",
            NodeKind::MultipathDevice,
            "mpatha"
        )));
        assert!(is_multipath_node(
            &Node::new("block:/dev/sdb", NodeKind::PhysicalDisk, "/dev/sdb")
                .with_property("multipath.path-state", "active ready running")
        ));
        assert!(is_multipath_node(
            &Node::new("block:/dev/sdc", NodeKind::PhysicalDisk, "/dev/sdc")
                .with_property("multipath.group-policy", "service-time 0")
        ));
        assert!(is_nvme_node(&Node::new(
            "block:/dev/nvme0n1",
            NodeKind::NvmeNamespace,
            "/dev/nvme0n1"
        )));
        assert!(is_nvme_node(
            &Node::new("block:/dev/nvme1n1", NodeKind::PhysicalDisk, "/dev/nvme1n1")
                .with_property("nvme.model", "Example NVMe")
        ));
        assert!(is_nvme_node(
            &Node::new("block:/dev/nvme2n1", NodeKind::PhysicalDisk, "/dev/nvme2n1")
                .with_property("nvme.subsystem", "nvme-subsys0")
        ));
        assert!(is_raid_node(&Node::new(
            "md:/dev/md0",
            NodeKind::MdRaid,
            "/dev/md0"
        )));
        assert!(is_raid_node(
            &Node::new("block:/dev/sda1", NodeKind::Partition, "/dev/sda1")
                .with_property("md.member-state", "active sync")
        ));
        assert!(is_loop_node(&Node::new(
            "block:/dev/loop0",
            NodeKind::LoopDevice,
            "/dev/loop0"
        )));
        assert!(is_loop_node(&Node::new(
            "file:/var/lib/images/root.img",
            NodeKind::BackingFile,
            "/var/lib/images/root.img"
        )));
        assert!(is_swap_node(&Node::new(
            "swap:/dev/sda3",
            NodeKind::Swap,
            "/dev/sda3"
        )));
        assert!(is_swap_node(
            &Node::new("block:/swapfile", NodeKind::BackingFile, "/swapfile")
                .with_property("swap.active", "true")
        ));
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
                .with_property("nvme.product", "Example Controller")
                .with_property("nvme.firmware", "1.0")
                .with_property("nvme.index", "0")
                .with_property("nvme.namespace", "1")
                .with_property("nvme.subsystem", "nvme-subsys0")
                .with_property("nvme.controller", "nvme0")
                .with_property("nvme.transport", "pcie")
                .with_property("nvme.controller-id", "1")
                .with_property("nvme.namespace-capacity", "900000000000")
                .with_property("nvme.lba-format", "512 B + 0 B")
                .with_property("nvme.maximum-lba", "1953125")
                .with_property("nvme.sector-size", "512")
                .with_property("partition.table", "gpt")
                .with_property("udev.symlink", "disk/by-id/nvme-Acme_FastDisk")
                .with_property("udev.devname", "/dev/nvme0n1")
                .with_property("udev.devtype", "disk")
                .with_property("udev.id-bus", "nvme")
                .with_property("udev.id-model", "FastDisk")
                .with_property("udev.id-model-id", "a808")
                .with_property("udev.id-vendor", "Acme")
                .with_property("udev.id-vendor-id", "144d")
                .with_property("udev.id-revision", "1.0")
                .with_property("udev.id-serial", "Acme_FastDisk_SERIAL")
                .with_property("udev.id-serial-short", "SERIAL")
                .with_property("udev.id-wwn", "eui.1234")
                .with_property("udev.id-path", "pci-0000:01:00.0-nvme-1")
                .with_property("udev.id-path-tag", "pci-0000_01_00_0-nvme-1")
                .with_property("udev.major", "259")
                .with_property("udev.minor", "0")
                .with_property("udev.subsystem", "block"),
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
            .with_property("udev.id-fs-type", "vfat")
            .with_property("udev.id-fs-version", "FAT32")
            .with_property("udev.id-fs-usage", "filesystem")
            .with_property("udev.id-fs-uuid", "AAAA-BBBB")
            .with_property("udev.id-fs-uuid-enc", "AAAA-BBBB")
            .with_property("udev.id-fs-uuid-sub", "CCCC-DDDD")
            .with_property("udev.id-fs-label", "EFI")
            .with_property("udev.id-fs-label-enc", "EFI")
            .with_property("udev.id-fs-label-safe", "EFI")
            .with_property("udev.id-part-entry-disk", "259:0")
            .with_property("udev.id-part-entry-number", "1")
            .with_property("udev.id-part-entry-offset", "2048")
            .with_property("udev.id-part-entry-size", "1048576")
            .with_property("udev.id-part-entry-scheme", "gpt")
            .with_property("udev.id-part-entry-type", "uefi")
            .with_property("udev.id-part-entry-name", "EFI System Partition")
            .with_property("udev.id-part-entry-uuid", "part-uuid")
            .with_property("udev.id-part-entry-flags", "0x1")
            .with_property("udev.id-part-table-type", "gpt")
            .with_property("udev.id-part-table-uuid", "table-uuid"),
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
            Node::new("block:/dev/dm-0", NodeKind::DeviceMapper, "/dev/dm-0")
                .with_path("/dev/dm-0")
                .with_property("udev.dm-name", "cryptroot")
                .with_property("udev.dm-uuid", "CRYPT-LUKS2-luks-uuid-cryptroot")
                .with_property("udev.dm-vg-name", "vg0")
                .with_property("udev.dm-lv-name", "root")
                .with_property("udev.dm-udev-rules-vsn", "3")
                .with_property("udev.dm-udev-primary-source-flag", "1")
                .with_property("udev.dm-udev-disable-other-rules-flag", "0")
                .with_property("udev.dm-subsystem-udev-flag0", "1")
                .with_property("udev.dm-subsystem-udev-flag1", "0"),
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
        assert!(output.contains("model=FastDisk vendor=Acme transport=nvme rotational=false"));
        assert!(output.contains("nvme-model=Example NVMe product=Example Controller firmware=1.0"));
        assert!(output.contains("ns-index=0 namespace=1 subsystem=nvme-subsys0 controller=nvme0"));
        assert!(output.contains(
            "transport=pcie controller-id=1 namespace-capacity=900000000000 lba-format=512 B + 0 B"
        ));
        assert!(output.contains("max-lba=1953125 sector-size=512 ptable=gpt"));
        assert!(output.contains("udev-link=disk/by-id/nvme-Acme_FastDisk"));
        assert!(output.contains("udev-devname=/dev/nvme0n1 udev-devtype=disk"));
        assert!(output.contains("udev-bus=nvme udev-model=FastDisk udev-model-id=a808"));
        assert!(output.contains("udev-vendor=Acme udev-vendor-id=144d udev-revision=1.0"));
        assert!(output.contains("udev-serial=Acme_FastDisk_SERIAL udev-serial-short=SERIAL"));
        assert!(output.contains("udev-wwn=eui.1234 udev-path=pci-0000:01:00.0-nvme-1"));
        assert!(output.contains("udev-path-tag=pci-0000_01_00_0-nvme-1"));
        assert!(output.contains("major=259 minor=0 subsystem=block"));
        assert!(output.contains("lsblk-type=part fstype=vfat partno=1 udev-fstype=vfat"));
        assert!(output.contains("udev-fs-version=FAT32 udev-fs-usage=filesystem"));
        assert!(output.contains("udev-fs-uuid=AAAA-BBBB udev-fs-uuid-enc=AAAA-BBBB"));
        assert!(output.contains("udev-fs-uuid-sub=CCCC-DDDD"));
        assert!(output.contains("udev-label=EFI udev-label-enc=EFI udev-label-safe=EFI"));
        assert!(output.contains("udev-part-disk=259:0 udev-part-number=1"));
        assert!(output.contains("udev-part-offset=2048 udev-part-size=1048576"));
        assert!(output.contains("udev-part-scheme=gpt udev-part-type=uefi"));
        assert!(output.contains("udev-part-name=EFI System Partition udev-part-uuid=part-uuid"));
        assert!(output.contains("udev-part-flags=0x1 udev-table-type=gpt"));
        assert!(output.contains("udev-table-uuid=table-uuid"));
        assert!(output.contains("dm-name=cryptroot dm-uuid=CRYPT-LUKS2-luks-uuid-cryptroot"));
        assert!(output.contains("dm-vg=vg0 dm-lv=root dm-rules=3"));
        assert!(output.contains("dm-primary-source=1 dm-disable-other-rules=0"));
        assert!(output.contains("dm-subsystem-flag0=1 dm-subsystem-flag1=0"));
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
            .with_property("lvm.metadata-percent", "3.00")
            .with_property("lvm.snap-percent", "4.00")
            .with_property("lvm.copy-percent", "99.00")
            .with_property("lvm.active", "active")
            .with_property("lvm.layout", "thin")
            .with_property("lvm.health", "ok")
            .with_property("lvm.when-full", "queue")
            .with_property("lvm.metadata-size", "128.00m")
            .with_property("lvm.role", "public")
            .with_property("lvm.cache-mode", "writeback")
            .with_property("lvm.cache-policy", "smq")
            .with_property("lvm.kernel-discards", "passdown")
            .with_property("lvm.writecache-writeback-blocks", "16");
        assert_eq!(
            usage_details(&lv),
            "data=12.50 metadata=3.00 snap=4.00 copy=99.00 layout=thin active=active health=ok when-full=queue metadata-size=128.00m role=public cache-mode=writeback cache-policy=smq kernel-discards=passdown writecache-writeback=16"
        );

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
            .with_property("xfs.meta-data.meta-data", "/dev/mapper/vg-root")
            .with_property("xfs.meta-data.isize", "512")
            .with_property("xfs.meta-data.agcount", "4")
            .with_property("xfs.meta-data.agsize", "65536")
            .with_property("xfs.meta-data.sectsz", "512")
            .with_property("xfs.meta-data.attr", "2")
            .with_property("xfs.meta-data.projid32bit", "1")
            .with_property("xfs.meta-data.crc", "1")
            .with_property("xfs.meta-data.finobt", "1")
            .with_property("xfs.meta-data.sparse", "1")
            .with_property("xfs.meta-data.rmapbt", "0")
            .with_property("xfs.data.blocks", "262144")
            .with_property("xfs.data.bsize", "4096")
            .with_property("xfs.data.imaxpct", "25")
            .with_property("xfs.data.sunit", "0")
            .with_property("xfs.data.swidth", "0")
            .with_property("xfs.meta-data.reflink", "1")
            .with_property("xfs.meta-data.bigtime", "1")
            .with_property("xfs.meta-data.inobtcount", "1")
            .with_property("xfs.meta-data.nrext64", "0")
            .with_property("xfs.naming.version", "2")
            .with_property("xfs.naming.bsize", "4096")
            .with_property("xfs.naming.ascii-ci", "0")
            .with_property("xfs.naming.ftype", "1")
            .with_property("xfs.log.type", "internal log")
            .with_property("xfs.log.bsize", "4096")
            .with_property("xfs.log.blocks", "2560")
            .with_property("xfs.log.version", "2")
            .with_property("xfs.log.sectsz", "512")
            .with_property("xfs.log.sunit", "0")
            .with_property("xfs.log.lazy-count", "1")
            .with_property("xfs.realtime.type", "none")
            .with_property("xfs.realtime.extsz", "4096")
            .with_property("xfs.realtime.blocks", "0")
            .with_property("xfs.realtime.rtextents", "0");
        assert_eq!(
            usage_details(&xfs),
            "xfs-source=/dev/mapper/vg-root xfs-isize=512 xfs-agcount=4 xfs-agsize=65536 xfs-sectsz=512 xfs-attr=2 xfs-projid32bit=1 xfs-crc=1 xfs-finobt=1 xfs-sparse=1 xfs-rmapbt=0 xfs-blocks=262144 xfs-bsize=4096 xfs-imaxpct=25 xfs-sunit=0 xfs-swidth=0 reflink=1 bigtime=1 xfs-inobtcount=1 xfs-nrext64=0 xfs-naming-version=2 xfs-naming-bsize=4096 xfs-ascii-ci=0 xfs-ftype=1 xfs-log-type=internal log xfs-log-bsize=4096 log-blocks=2560 xfs-log-version=2 xfs-log-sectsz=512 xfs-log-sunit=0 xfs-log-lazy-count=1 xfs-realtime-type=none xfs-realtime-extsz=4096 xfs-realtime-blocks=0 xfs-realtime-rtextents=0"
        );

        let ext = Node::new("fs:/dev/sda2", NodeKind::Filesystem, "ext4")
            .with_property("filesystem.type", "ext4")
            .with_property("blkid.version", "1.0")
            .with_property("blkid.block-size", "4096")
            .with_property("blkid.usage", "filesystem")
            .with_property("blkid.uuid-sub", "subvol-uuid")
            .with_property("ext.state", "clean")
            .with_property("ext.errors-behavior", "Continue")
            .with_property("ext.os-type", "Linux")
            .with_property("ext.block-count", "122096646")
            .with_property("ext.reserved-block-count", "6104832")
            .with_property("ext.overhead-clusters", "123456")
            .with_property("ext.free-blocks", "73328197")
            .with_property("ext.block-size", "4096")
            .with_property("ext.fragment-size", "4096")
            .with_property("ext.blocks-per-group", "32768")
            .with_property("ext.fragments-per-group", "32768")
            .with_property("ext.inode-count", "30531584")
            .with_property("ext.free-inodes", "27187554")
            .with_property("ext.inodes-per-group", "8192")
            .with_property("ext.features", "has_journal extent metadata_csum")
            .with_property("ext.flags", "signed_directory_hash")
            .with_property("ext.default-directory-hash", "half_md4")
            .with_property(
                "ext.directory-hash-seed",
                "11111111-2222-3333-4444-555555555555",
            )
            .with_property("ext.default-mount-options", "user_xattr acl")
            .with_property("ext.created", "Mon Jan 01 00:00:00 2024")
            .with_property("ext.last-mount-time", "Mon Jun 22 12:00:00 2026")
            .with_property("ext.last-write-time", "Mon Jun 22 12:00:00 2026")
            .with_property("ext.mount-count", "12")
            .with_property("ext.maximum-mount-count", "-1")
            .with_property("ext.last-checked", "Mon Jan 01 00:00:00 2024")
            .with_property("ext.check-interval", "0 (<none>)")
            .with_property("ext.lifetime-writes", "189 GB")
            .with_property("ext.reserved-blocks-uid", "0 (user root)")
            .with_property("ext.reserved-blocks-gid", "0 (group root)")
            .with_property("ext.first-inode", "11")
            .with_property("ext.inode-size", "256")
            .with_property("ext.journal-inode", "8")
            .with_property("ext.journal-uuid", "99999999-aaaa-bbbb-cccc-dddddddddddd")
            .with_property("ext.journal-backup", "inode blocks")
            .with_property("ext.journal-features", "journal_incompat_revoke")
            .with_property("ext.journal-size", "1024M")
            .with_property("ext.checksum-type", "crc32c")
            .with_property("ext.checksum", "0x12345678");
        assert_eq!(
            usage_details(&ext),
            "fstype=ext4 version=1.0 blkid-block-size=4096 usage=filesystem uuid-sub=subvol-uuid ext-state=clean errors=Continue os=Linux blocks=122096646 reserved-blocks=6104832 overhead-clusters=123456 free-blocks=73328197 block-size=4096 fragment-size=4096 blocks-per-group=32768 fragments-per-group=32768 inodes=30531584 free-inodes=27187554 inodes-per-group=8192 features=has_journal extent metadata_csum flags=signed_directory_hash dir-hash=half_md4 dir-hash-seed=11111111-2222-3333-4444-555555555555 default-mount=user_xattr acl created=Mon Jan 01 00:00:00 2024 last-mounted=Mon Jun 22 12:00:00 2026 last-written=Mon Jun 22 12:00:00 2026 mount-count=12 max-mount-count=-1 last-checked=Mon Jan 01 00:00:00 2024 check-interval=0 (<none>) lifetime-writes=189 GB reserved-uid=0 (user root) reserved-gid=0 (group root) first-inode=11 inode-size=256 journal-inode=8 journal-uuid=99999999-aaaa-bbbb-cccc-dddddddddddd journal-backup=inode blocks journal-features=journal_incompat_revoke journal-size=1024M checksum-type=crc32c checksum=0x12345678"
        );

        let exfat = Node::new("fs:/dev/sdb1", NodeKind::Filesystem, "exfat")
            .with_property("exfat.guid", "01234567-89ab-cdef-0123-456789abcdef")
            .with_property("exfat.exfatprogs-version", "1.2.4")
            .with_property("exfat.volume-serial", "0x6eef953b")
            .with_property("exfat.volume-length-sectors", "3203072")
            .with_property("exfat.fat-offset-sector-offset", "2048")
            .with_property("exfat.fat-length-sectors", "448")
            .with_property("exfat.cluster-heap-offset-sector-offset", "4096")
            .with_property("exfat.cluster-count", "49984")
            .with_property("exfat.used-clusters", "48960")
            .with_property("exfat.free-clusters", "1024")
            .with_property("exfat.root-cluster-cluster-offset", "4")
            .with_property("exfat.bytes-per-sector", "512")
            .with_property("exfat.sectors-per-cluster", "64")
            .with_property("exfat.bytes-per-cluster", "32768");
        assert_eq!(
            usage_details(&exfat),
            "guid=01234567-89ab-cdef-0123-456789abcdef exfatprogs=1.2.4 serial=0x6eef953b sectors=3203072 fat-offset=2048 fat-length=448 cluster-heap-offset=4096 clusters=49984 used-clusters=48960 free-clusters=1024 root-cluster=4 sector-bytes=512 sectors-per-cluster=64 cluster-bytes=32768"
        );

        let ntfs = Node::new("fs:/dev/sda1", NodeKind::Filesystem, "ntfs")
            .with_property("ntfs.volume-name", "Windows")
            .with_property("ntfs.volume-serial", "01234567-89abcdef")
            .with_property("ntfs.version", "3.1")
            .with_property("ntfs.sector-size", "512")
            .with_property("ntfs.cluster-size", "4096")
            .with_property("ntfs.volume-size-clusters", "262144")
            .with_property("ntfs.mft-record-size", "1024");
        assert_eq!(
            usage_details(&ntfs),
            "ntfs-name=Windows ntfs-serial=01234567-89abcdef ntfs-version=3.1 ntfs-sector=512 ntfs-cluster=4096 ntfs-clusters=262144 ntfs-mft-record=1024"
        );

        let f2fs = Node::new("fs:/dev/sdb2", NodeKind::Filesystem, "f2fs")
            .with_property("f2fs.filesystem-volume-name", "mobile")
            .with_property(
                "f2fs.filesystem-uuid",
                "01234567-89ab-cdef-0123-456789abcdef",
            )
            .with_property("f2fs.block-size", "4096")
            .with_property("f2fs.block-count", "262144")
            .with_property("f2fs.user-block-count", "245760")
            .with_property("f2fs.valid-block-count", "65536")
            .with_property("f2fs.total-valid-block-count", "65540")
            .with_property("f2fs.valid-node-count", "4096")
            .with_property("f2fs.valid-inode-count", "2048")
            .with_property("f2fs.segment-count", "2048")
            .with_property("f2fs.segment-count-main", "1984")
            .with_property("f2fs.segment-count-ckpt", "2")
            .with_property("f2fs.segment-count-sit", "2")
            .with_property("f2fs.segment-count-nat", "4")
            .with_property("f2fs.segment-count-ssa", "1")
            .with_property("f2fs.overprov-segment-count", "64")
            .with_property("f2fs.section-count", "1984")
            .with_property("f2fs.segs-per-sec", "1")
            .with_property("f2fs.secs-per-zone", "1")
            .with_property("f2fs.log-sectorsize", "9")
            .with_property("f2fs.log-sectors-per-block", "3")
            .with_property("f2fs.log-blocksize", "12")
            .with_property("f2fs.log-blocks-per-seg", "9")
            .with_property("f2fs.cp-payload", "0")
            .with_property("f2fs.version", "Linux version 6.12")
            .with_property("f2fs.init-version", "Linux version 6.1")
            .with_property("f2fs.extension-count", "29")
            .with_property("f2fs.hot-ext-count", "5");
        assert_eq!(
            usage_details(&f2fs),
            "f2fs-uuid=01234567-89ab-cdef-0123-456789abcdef f2fs-name=mobile f2fs-block-size=4096 f2fs-blocks=262144 f2fs-user-blocks=245760 f2fs-valid-blocks=65536 f2fs-total-valid-blocks=65540 f2fs-valid-nodes=4096 f2fs-valid-inodes=2048 f2fs-segments=2048 f2fs-main-segments=1984 f2fs-ckpt-segments=2 f2fs-sit-segments=2 f2fs-nat-segments=4 f2fs-ssa-segments=1 f2fs-overprov=64 f2fs-sections=1984 f2fs-segs-per-sec=1 f2fs-secs-per-zone=1 f2fs-log-sector=9 f2fs-log-sectors-block=3 f2fs-log-block=12 f2fs-log-blocks-seg=9 f2fs-cp-payload=0 f2fs-version=Linux version 6.12 f2fs-init-version=Linux version 6.1 f2fs-extensions=29 f2fs-hot-extensions=5"
        );

        let bcachefs = Node::new(
            "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3",
            NodeKind::BcachefsFilesystem,
            "archive",
        )
        .with_property(
            "bcachefs.external-uuid",
            "a2d6fc04-efd0-4e36-aece-2475941d09a3",
        )
        .with_property(
            "bcachefs.internal-uuid",
            "55083d1e-27cf-4929-ada4-3fe6e45cf02c",
        )
        .with_property(
            "bcachefs.magic-number",
            "c68573f6-66ce-90a9-d96a-60cf803df7ef",
        )
        .with_property("bcachefs.device", "ST12000NM001G-2M")
        .with_property("bcachefs.member-device", "/dev/sdc")
        .with_property("bcachefs.mount-target", "/mnt/archive")
        .with_property("bcachefs.device-index", "6")
        .with_property("bcachefs.version", "1.20: (unknown version)")
        .with_property(
            "bcachefs.version-upgrade-complete",
            "1.20: (unknown version)",
        )
        .with_property("bcachefs.online-reserved", "507957248")
        .with_property("bcachefs.device-count", "2")
        .with_property("bcachefs.data-sb", "3149824")
        .with_property("bcachefs.data-journal", "4294967296")
        .with_property("bcachefs.data-btree", "1048576")
        .with_property("bcachefs.data-user", "2147483648");
        assert_eq!(
            usage_details(&bcachefs),
            "bcachefs-uuid=a2d6fc04-efd0-4e36-aece-2475941d09a3 bcachefs-internal=55083d1e-27cf-4929-ada4-3fe6e45cf02c bcachefs-magic=c68573f6-66ce-90a9-d96a-60cf803df7ef bcachefs-super-device=ST12000NM001G-2M bcachefs-member=/dev/sdc bcachefs-mount=/mnt/archive bcachefs-device=6 bcachefs-version=1.20: (unknown version) bcachefs-upgrade-complete=1.20: (unknown version) bcachefs-reserved=507957248 bcachefs-devices=2 bcachefs-sb=3149824 bcachefs-journal=4294967296 bcachefs-btree=1048576 bcachefs-user=2147483648"
        );

        let bcachefs_device = Node::new(
            "bcachefs-device:a2d6fc04-efd0-4e36-aece-2475941d09a3:6",
            NodeKind::BcachefsDevice,
            "sdc",
        )
        .with_property("bcachefs.device-label", "hdd.archive")
        .with_property("bcachefs.device-state", "rw")
        .with_property("bcachefs.device-free", "1649975230464")
        .with_property("bcachefs.device-capacity", "16000900661248")
        .with_property("bcachefs.device-data-sb", "3149824")
        .with_property("bcachefs.device-data-journal", "4294967296")
        .with_property("bcachefs.device-data-btree", "890241024")
        .with_property("bcachefs.device-data-user", "0");
        assert_eq!(
            usage_details(&bcachefs_device),
            "bcachefs-label=hdd.archive bcachefs-state=rw bcachefs-device-free=1649975230464 bcachefs-device-capacity=16000900661248 bcachefs-device-sb=3149824 bcachefs-device-journal=4294967296 bcachefs-device-btree=890241024 bcachefs-device-user=0"
        );

        let bcache = Node::new("block:/dev/bcache0", NodeKind::CacheDevice, "bcache0")
            .with_property("bcache.role", "backing")
            .with_property("bcache.kind", "cache-set")
            .with_property("bcache.backing-device", "/dev/sdb1")
            .with_property("bcache.set-uuid", "cache-set-uuid")
            .with_property("bcache.label", "fast-cache")
            .with_property("bcache.state", "clean")
            .with_property("bcache.running", "1")
            .with_property("bcache.cache-available-percent", "78")
            .with_property("bcache.cache-mode", "writeback")
            .with_property("bcache.cache-replacement-policy", "lru")
            .with_property("bcache.discard", "true")
            .with_property("bcache.dirty-data", "64.0M")
            .with_property("bcache.io-errors", "0")
            .with_property("bcache.metadata-written", "128.0M")
            .with_property("bcache.priority-stats", "Unused: 0% Metadata: 1%")
            .with_property("bcache.readahead", "0")
            .with_property("bcache.sequential-cutoff", "4.0M")
            .with_property("bcache.written", "512.0M")
            .with_property("bcache.writeback-delay", "30")
            .with_property("bcache.writeback-metadata", "true")
            .with_property("bcache.writeback-percent", "10")
            .with_property("bcache.writeback-rate", "1.0M/sec")
            .with_property("bcache.writeback-running", "1");
        assert_eq!(
            usage_details(&bcache),
            "role=backing kind=cache-set backing-device=/dev/sdb1 set-uuid=cache-set-uuid label=fast-cache state=clean running=1 available-percent=78 cache-mode=writeback replacement=lru discard=true dirty=64.0M io-errors=0 metadata-written=128.0M priority-stats=Unused: 0% Metadata: 1% readahead=0 sequential-cutoff=4.0M written=512.0M writeback-delay=30 writeback-metadata=true writeback-percent=10 writeback-rate=1.0M/sec writeback-running=1"
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
        .with_property("nvme.generic-path", "/dev/ng0n1")
        .with_property("nvme.model", "Example NVMe")
        .with_property("nvme.product", "Example Controller")
        .with_property("nvme.firmware", "1.0")
        .with_property("nvme.index", "0")
        .with_property("nvme.namespace", "1")
        .with_property("nvme.subsystem", "nvme-subsys0")
        .with_property("nvme.controller", "nvme0")
        .with_property("nvme.address", "0000:01:00.0")
        .with_property("nvme.transport", "pcie")
        .with_property("nvme.controller-id", "1")
        .with_property("nvme.namespace-capacity", "900000000000")
        .with_property("nvme.lba-format", "512 B + 0 B")
        .with_property("nvme.maximum-lba", "1953125")
        .with_property("nvme.sector-size", "512");
        assert_eq!(
            usage_details(&nvme),
            "generic=/dev/ng0n1 nvme-model=Example NVMe product=Example Controller firmware=1.0 ns-index=0 namespace=1 subsystem=nvme-subsys0 controller=nvme0 address=0000:01:00.0 transport=pcie controller-id=1 namespace-capacity=900000000000 lba-format=512 B + 0 B max-lba=1953125 sector-size=512"
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
                .with_property("vdo.write-policy", "sync")
                .with_property("vdo.configured-write-policy", "auto")
                .with_property("vdo.block-map-cache-size", "128M")
                .with_property("vdo.data-blocks-used", "65536")
                .with_property("vdo.logical-blocks-used", "262144"),
        );

        let mut output = Vec::new();
        print_usage(&mut output, &graph).expect("usage table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("DETAILS"));
        assert!(output.contains("backing=/dev/sdb logical=100G physical=50G"));
        assert!(output.contains(
            "vdo-use=50% saving=20% mode=normal write-policy=sync configured-write-policy=auto"
        ));
        assert!(output.contains("block-map-cache=128M data-blocks=65536 logical-blocks=262144"));
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
                .with_property("xfs.meta-data.meta-data", "/dev/mapper/vg-root")
                .with_property("xfs.meta-data.isize", "512")
                .with_property("xfs.meta-data.agcount", "4")
                .with_property("xfs.meta-data.crc", "1")
                .with_property("xfs.data.blocks", "262144")
                .with_property("xfs.data.bsize", "4096")
                .with_property("xfs.data.imaxpct", "25")
                .with_property("xfs.meta-data.reflink", "1")
                .with_property("xfs.meta-data.bigtime", "1")
                .with_property("xfs.naming.version", "2")
                .with_property("xfs.naming.ftype", "1")
                .with_property("xfs.log.type", "internal log")
                .with_property("xfs.log.blocks", "2560")
                .with_property("xfs.realtime.type", "none")
                .with_property("xfs.realtime.blocks", "0"),
        );
        graph.add_node(
            Node::new("fs:/dev/sda2", NodeKind::Filesystem, "ext4")
                .with_property("filesystem.type", "ext4")
                .with_property("ext.state", "clean")
                .with_property("ext.errors-behavior", "Continue")
                .with_property("ext.os-type", "Linux")
                .with_property("ext.block-count", "122096646")
                .with_property("ext.reserved-block-count", "6104832")
                .with_property("ext.overhead-clusters", "123456")
                .with_property("ext.free-blocks", "73328197")
                .with_property("ext.block-size", "4096")
                .with_property("ext.blocks-per-group", "32768")
                .with_property("ext.inode-count", "30531584")
                .with_property("ext.free-inodes", "27187554")
                .with_property("ext.inodes-per-group", "8192")
                .with_property("ext.features", "has_journal extent metadata_csum")
                .with_property("ext.flags", "signed_directory_hash")
                .with_property("ext.default-directory-hash", "half_md4")
                .with_property(
                    "ext.directory-hash-seed",
                    "11111111-2222-3333-4444-555555555555",
                )
                .with_property("ext.default-mount-options", "user_xattr acl")
                .with_property("ext.mount-count", "12")
                .with_property("ext.maximum-mount-count", "-1")
                .with_property("ext.check-interval", "0 (<none>)")
                .with_property("ext.inode-size", "256")
                .with_property("ext.journal-inode", "8")
                .with_property("ext.journal-uuid", "99999999-aaaa-bbbb-cccc-dddddddddddd")
                .with_property("ext.journal-size", "1024M")
                .with_property("ext.checksum-type", "crc32c")
                .with_property("ext.checksum", "0x12345678"),
        );
        graph.add_node(
            Node::new("fs:/dev/sdb1", NodeKind::Filesystem, "exfat")
                .with_property("exfat.guid", "01234567-89ab-cdef-0123-456789abcdef")
                .with_property("exfat.exfatprogs-version", "1.2.4")
                .with_property("exfat.volume-serial", "0x6eef953b")
                .with_property("exfat.volume-length-sectors", "3203072")
                .with_property("exfat.fat-offset-sector-offset", "2048")
                .with_property("exfat.fat-length-sectors", "448")
                .with_property("exfat.cluster-heap-offset-sector-offset", "4096")
                .with_property("exfat.cluster-count", "49984")
                .with_property("exfat.used-clusters", "48960")
                .with_property("exfat.free-clusters", "1024")
                .with_property("exfat.root-cluster-cluster-offset", "4")
                .with_property("exfat.bytes-per-sector", "512")
                .with_property("exfat.sectors-per-cluster", "64")
                .with_property("exfat.bytes-per-cluster", "32768"),
        );
        graph.add_node(
            Node::new("btrfs:fs-uuid", NodeKind::BtrfsFilesystem, "data")
                .with_property("btrfs.mount-target", "/data")
                .with_property("btrfs.data-profile", "single")
                .with_property("btrfs.data-size", "512")
                .with_property("btrfs.data-used", "400")
                .with_property("btrfs.metadata-profile", "DUP")
                .with_property("btrfs.metadata-size", "128")
                .with_property("btrfs.metadata-used", "64")
                .with_property("btrfs.system-profile", "DUP")
                .with_property("btrfs.system-size", "64")
                .with_property("btrfs.system-used", "32"),
        );
        graph.add_node(
            Node::new("fs:/dev/sda1", NodeKind::Filesystem, "ntfs")
                .with_property("ntfs.volume-name", "Windows")
                .with_property("ntfs.volume-serial", "01234567-89abcdef")
                .with_property("ntfs.version", "3.1")
                .with_property("ntfs.cluster-size", "4096")
                .with_property("ntfs.mft-record-size", "1024"),
        );
        graph.add_node(
            Node::new("fs:/dev/sdb2", NodeKind::Filesystem, "f2fs")
                .with_property("f2fs.filesystem-volume-name", "mobile")
                .with_property("f2fs.block-size", "4096")
                .with_property("f2fs.block-count", "262144")
                .with_property("f2fs.user-block-count", "245760")
                .with_property("f2fs.valid-block-count", "65536")
                .with_property("f2fs.segment-count", "2048")
                .with_property("f2fs.segment-count-main", "1984")
                .with_property("f2fs.segment-count-ckpt", "2")
                .with_property("f2fs.segment-count-sit", "2")
                .with_property("f2fs.segment-count-nat", "4")
                .with_property("f2fs.segment-count-ssa", "1")
                .with_property("f2fs.overprov-segment-count", "64")
                .with_property("f2fs.section-count", "1984")
                .with_property("f2fs.segs-per-sec", "1")
                .with_property("f2fs.secs-per-zone", "1")
                .with_property("f2fs.version", "Linux version 6.12"),
        );
        graph.add_node(
            Node::new(
                "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3",
                NodeKind::BcachefsFilesystem,
                "archive",
            )
            .with_property(
                "bcachefs.external-uuid",
                "a2d6fc04-efd0-4e36-aece-2475941d09a3",
            )
            .with_property("bcachefs.member-device", "/dev/sdc")
            .with_property("bcachefs.mount-target", "/mnt/archive")
            .with_property("bcachefs.device-index", "6")
            .with_property(
                "bcachefs.magic-number",
                "c68573f6-66ce-90a9-d96a-60cf803df7ef",
            )
            .with_property(
                "bcachefs.version-upgrade-complete",
                "1.20: (unknown version)",
            )
            .with_property("bcachefs.data-sb", "3149824")
            .with_property("bcachefs.data-journal", "4294967296")
            .with_property("bcachefs.data-user", "2147483648"),
        );

        let mut output = Vec::new();
        print_filesystems(&mut output, &graph).expect("filesystems table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("DETAILS"));
        assert!(output.contains("xfs-source=/dev/mapper/vg-root xfs-isize=512 xfs-agcount=4"));
        assert!(output.contains("xfs-crc=1 xfs-blocks=262144 xfs-bsize=4096"));
        assert!(output.contains("xfs-imaxpct=25 reflink=1 bigtime=1"));
        assert!(output.contains(
            "xfs-naming-version=2 xfs-ftype=1 xfs-log-type=internal log log-blocks=2560"
        ));
        assert!(output.contains("xfs-realtime-type=none xfs-realtime-blocks=0"));
        assert!(output.contains(
            "fstype=ext4 ext-state=clean errors=Continue os=Linux blocks=122096646 reserved-blocks=6104832 overhead-clusters=123456 free-blocks=73328197"
        ));
        assert!(output.contains(
            "block-size=4096 blocks-per-group=32768 inodes=30531584 free-inodes=27187554 inodes-per-group=8192"
        ));
        assert!(output.contains(
            "features=has_journal extent metadata_csum flags=signed_directory_hash dir-hash=half_md4 dir-hash-seed=11111111-2222-3333-4444-555555555555"
        ));
        assert!(output.contains("default-mount=user_xattr acl"));
        assert!(output.contains(
            "mount-count=12 max-mount-count=-1 check-interval=0 (<none>) inode-size=256 journal-inode=8 journal-uuid=99999999-aaaa-bbbb-cccc-dddddddddddd"
        ));
        assert!(output.contains("journal-size=1024M checksum-type=crc32c checksum=0x12345678"));
        assert!(output.contains(
            "guid=01234567-89ab-cdef-0123-456789abcdef exfatprogs=1.2.4 serial=0x6eef953b sectors=3203072"
        ));
        assert!(output.contains("fat-offset=2048 fat-length=448 cluster-heap-offset=4096"));
        assert!(
            output.contains("clusters=49984 used-clusters=48960 free-clusters=1024 root-cluster=4")
        );
        assert!(output.contains("sector-bytes=512 sectors-per-cluster=64 cluster-bytes=32768"));
        assert!(output.contains(
            "mount-target=/data data-profile=single data-size=512 data-used=400 metadata-profile=DUP"
        ));
        assert!(output.contains(
            "metadata-size=128 metadata-used=64 system-profile=DUP system-size=64 system-used=32"
        ));
        assert!(
            output.contains(
                "ntfs-name=Windows ntfs-serial=01234567-89abcdef ntfs-version=3.1 ntfs-cluster=4096 ntfs-mft-record=1024"
            )
        );
        assert!(output.contains(
            "f2fs-name=mobile f2fs-block-size=4096 f2fs-blocks=262144 f2fs-user-blocks=245760 f2fs-valid-blocks=65536"
        ));
        assert!(output.contains(
            "f2fs-segments=2048 f2fs-main-segments=1984 f2fs-ckpt-segments=2 f2fs-sit-segments=2 f2fs-nat-segments=4 f2fs-ssa-segments=1"
        ));
        assert!(output.contains(
            "f2fs-overprov=64 f2fs-sections=1984 f2fs-segs-per-sec=1 f2fs-secs-per-zone=1 f2fs-version=Linux version 6.12"
        ));
        assert!(output.contains(
            "bcachefs-uuid=a2d6fc04-efd0-4e36-aece-2475941d09a3 bcachefs-magic=c68573f6-66ce-90a9-d96a-60cf803df7ef bcachefs-member=/dev/sdc bcachefs-mount=/mnt/archive"
        ));
        assert!(output.contains(
            "bcachefs-device=6 bcachefs-upgrade-complete=1.20: (unknown version) bcachefs-sb=3149824 bcachefs-journal=4294967296 bcachefs-user=2147483648"
        ));
    }

    #[test]
    fn complex_filesystems_table_includes_topology_and_domain_details() {
        let mut graph = StorageGraph::empty();
        graph.add_node(Node::new(
            "block:/dev/nvme0n1p2",
            NodeKind::Partition,
            "/dev/nvme0n1p2",
        ));
        graph.add_node(
            Node::new("btrfs:fs-uuid", NodeKind::BtrfsFilesystem, "/mnt/persist")
                .with_size_bytes(536_870_912_000)
                .with_usage(Usage {
                    used_bytes: Some(214_748_364_800),
                    free_bytes: Some(322_122_547_200),
                    allocated_bytes: None,
                })
                .with_property("btrfs.mount-target", "/mnt/persist")
                .with_property("btrfs.data-profile", "single")
                .with_property("btrfs.metadata-profile", "DUP"),
        );
        graph.add_node(
            Node::new(
                "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3",
                NodeKind::BcachefsFilesystem,
                "archive",
            )
            .with_usage(Usage {
                used_bytes: Some(2_147_483_648),
                free_bytes: Some(8_589_934_592),
                allocated_bytes: Some(10_737_418_240),
            })
            .with_property("bcachefs.mount-target", "/mnt/archive")
            .with_property("bcachefs.device-count", "2")
            .with_property("bcachefs.data-user", "2147483648"),
        );
        graph.add_node(
            Node::new("zpool:tank", NodeKind::ZfsPool, "tank")
                .with_size_bytes(1_099_511_627_776)
                .with_usage(Usage {
                    used_bytes: Some(274_877_906_944),
                    free_bytes: Some(824_633_720_832),
                    allocated_bytes: None,
                })
                .with_property("zfs.health", "ONLINE"),
        );
        graph.add_node(
            Node::new("zfs-dataset:tank/home", NodeKind::ZfsDataset, "tank/home")
                .with_property("zfs.compression", "zstd")
                .with_property("zfs.encryption", "aes-256-gcm")
                .with_property("zfs.keystatus", "available"),
        );
        graph.add_node(
            Node::new(
                "bcachefs-device:a2d6fc04-efd0-4e36-aece-2475941d09a3:0",
                NodeKind::BcachefsDevice,
                "/dev/sdc",
            )
            .with_property("bcachefs.device-state", "rw")
            .with_property("bcachefs.device-free", "8589934592"),
        );
        graph.add_edge(Edge::new(
            "block:/dev/nvme0n1p2",
            "btrfs:fs-uuid",
            Relationship::Backs,
        ));
        graph.add_edge(Edge::new(
            "bcachefs-device:a2d6fc04-efd0-4e36-aece-2475941d09a3:0",
            "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3",
            Relationship::MemberOf,
        ));

        let mut output = Vec::new();
        print_complex_filesystems(&mut output, &graph).expect("complex filesystems table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("BACKING"));
        assert!(output.contains("/mnt/persist"));
        assert!(output.contains("500.0 GiB"));
        assert!(output.contains("40.0%"));
        assert!(output.contains("data-profile=single metadata-profile=DUP"));
        assert!(output.contains("archive"));
        assert!(output.contains("20.0%"));
        assert!(output.contains("bcachefs-mount=/mnt/archive bcachefs-devices=2"));
        assert!(output.contains("tank"));
        assert!(output.contains("health=ONLINE"));
        assert!(output.contains("tank/home"));
        assert!(output.contains("compression=zstd encryption=aes-256-gcm keystatus=available"));
        assert!(output.contains("bcachefs-state=rw bcachefs-device-free=8589934592"));
    }

    #[test]
    fn zfs_table_includes_pool_vdev_dataset_snapshot_and_zvol_details() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("zfs-pool:tank", NodeKind::ZfsPool, "tank")
                .with_size_bytes(1_099_511_627_776)
                .with_usage(Usage {
                    used_bytes: Some(274_877_906_944),
                    free_bytes: Some(824_633_720_832),
                    allocated_bytes: Some(274_877_906_944),
                })
                .with_property("zfs.health", "ONLINE")
                .with_property("zfs.state", "ONLINE")
                .with_property("zfs.status", "some devices need attention")
                .with_property("zfs.action", "replace the faulted device")
                .with_property("zfs.scan", "scrub repaired 0B")
                .with_property("zfs.errors", "No known data errors"),
        );
        graph.add_node(
            Node::new(
                "zfs-vdev:tank:/dev/disk/by-id/nvme-tank-a",
                NodeKind::ZfsVdev,
                "/dev/disk/by-id/nvme-tank-a",
            )
            .with_path("/dev/disk/by-id/nvme-tank-a")
            .with_property("zfs.vdev-role", "data")
            .with_property("zfs.vdev-state", "ONLINE")
            .with_property("zfs.read-errors", "0")
            .with_property("zfs.write-errors", "1")
            .with_property("zfs.checksum-errors", "2"),
        );
        graph.add_node(
            Node::new("zfs-dataset:tank/home", NodeKind::ZfsDataset, "tank/home")
                .with_usage(Usage {
                    used_bytes: Some(107_374_182_400),
                    free_bytes: Some(805_306_368_000),
                    allocated_bytes: Some(107_374_182_400),
                })
                .with_property("zfs.compression", "zstd")
                .with_property("zfs.quota", "500G")
                .with_property("zfs.reservation", "10G")
                .with_property("zfs.encryption", "aes-256-gcm")
                .with_property("zfs.keystatus", "available"),
        );
        graph.add_node(
            Node::new(
                "zfs-snapshot:tank/home@daily",
                NodeKind::ZfsSnapshot,
                "tank/home@daily",
            )
            .with_property("zfs.userrefs", "2")
            .with_property("zfs.compression", "zstd"),
        );
        graph.add_node(
            Node::new("zvol:tank/vm/root", NodeKind::Zvol, "tank/vm/root")
                .with_size_bytes(85_899_345_920)
                .with_property("zfs.origin", "tank/vm/base@clean")
                .with_property("zfs.volsize", "80G"),
        );
        graph.add_edge(Edge::new(
            "zfs-pool:tank",
            "zfs-vdev:tank:/dev/disk/by-id/nvme-tank-a",
            Relationship::Contains,
        ));
        graph.add_edge(Edge::new(
            "zfs-pool:tank",
            "zfs-dataset:tank/home",
            Relationship::Contains,
        ));
        graph.add_edge(Edge::new(
            "zfs-pool:tank",
            "zvol:tank/vm/root",
            Relationship::Contains,
        ));
        graph.add_edge(Edge::new(
            "zfs-snapshot:tank/home@daily",
            "zfs-dataset:tank/home",
            Relationship::SnapshotOf,
        ));

        let pool = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::ZfsPool)
            .expect("pool fixture exists");
        let snapshot = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::ZfsSnapshot)
            .expect("snapshot fixture exists");
        assert_eq!(zfs_child_count(&graph, pool), 3);
        assert_eq!(zfs_child_count(&graph, snapshot), 1);

        let mut output = Vec::new();
        print_zfs(&mut output, &graph).expect("zfs table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("HEALTH"));
        assert!(output.contains("ORIGIN"));
        assert!(output.contains("CHILDREN"));
        assert!(output.contains("tank"));
        assert!(output.contains("ONLINE"));
        assert!(output.contains(
            "status=some devices need attention action=replace the faulted device scan=scrub repaired 0B errors=No known data errors"
        ));
        assert!(
            output
                .contains("data vdev-state=ONLINE read-errors=0 write-errors=1 checksum-errors=2")
        );
        assert!(output.contains("tank/home"));
        assert!(output.contains(
            "compression=zstd quota=500G reservation=10G encryption=aes-256-gcm keystatus=available"
        ));
        assert!(output.contains("tank/home@daily"));
        assert!(output.contains("userrefs=2 compression=zstd"));
        assert!(output.contains("tank/vm/root"));
        assert!(output.contains("tank/vm/base@clean"));
        assert!(output.contains("volsize=80G"));
    }

    #[test]
    fn volumes_table_includes_domain_metadata_details() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("lvm-lv:vg/root-snap", NodeKind::LvmSnapshot, "vg/root-snap")
                .with_property("lvm.origin", "root")
                .with_property("lvm.pool", "thinpool")
                .with_property("lvm.data-percent", "12.50")
                .with_property("lvm.active", "active")
                .with_property("lvm.layout", "snapshot")
                .with_property("lvm.health", "partial")
                .with_property("lvm.tags", "backup,snapshot")
                .with_property("lvm.cache-mode", "writeback")
                .with_property("lvm.cache-policy", "smq"),
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
        assert!(output.contains(
            "data=12.50 layout=snapshot origin=root pool=thinpool active=active health=partial tags=backup,snapshot cache-mode=writeback cache-policy=smq"
        ));
        assert!(output.contains("level=raid1 state=clean raid-devices=2"));
        assert!(output.contains("attached-disk=sdb"));
        assert!(output.contains("server=storage.example export=/export/home"));
    }

    #[test]
    fn lvm_table_includes_volume_group_and_segment_details() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "lvm-pv:/dev/nvme0n1p3",
                NodeKind::LvmPhysicalVolume,
                "/dev/nvme0n1p3",
            )
            .with_path("/dev/nvme0n1p3")
            .with_size_bytes(536_870_912_000)
            .with_property("lvm.active", "active")
            .with_property("lvm.pv-format", "lvm2")
            .with_property("lvm.dev-size", "500.00g")
            .with_property("lvm.pe-start", "1.00m")
            .with_property("lvm.pv-pe-count", "128000")
            .with_property("lvm.pv-pe-allocated", "102400")
            .with_property("lvm.pv-mda-free", "1020.00k")
            .with_property("lvm.pv-device-id", "wwn-0x1234")
            .with_property("lvm.tags", "ssd,system"),
        );
        graph.add_node(
            Node::new("lvm-vg:vg0", NodeKind::LvmVolumeGroup, "vg0")
                .with_size_bytes(1_099_511_627_776)
                .with_property("lvm.vg-format", "lvm2")
                .with_property("lvm.permissions", "writeable")
                .with_property("lvm.vg-autoactivation", "enabled")
                .with_property("lvm.allocation-policy", "normal")
                .with_property("lvm.vg-system-id", "host-a")
                .with_property("lvm.vg-lock-type", "none")
                .with_property("lvm.extent-size", "4.00m")
                .with_property("lvm.extent-count", "262144")
                .with_property("lvm.free-count", "5120")
                .with_property("lvm.pv-count", "2")
                .with_property("lvm.lv-count", "5")
                .with_property("lvm.snapshot-count", "1")
                .with_property("lvm.vg-seqno", "17")
                .with_property("lvm.vg-mda-free", "1020.00k")
                .with_property("lvm.vg-mda-copies", "unmanaged"),
        );
        graph.add_node(
            Node::new("lvm-thin-pool:vg0/pool", NodeKind::LvmThinPool, "vg0/pool")
                .with_size_bytes(858_993_459_200)
                .with_property("lvm.data-percent", "42.00")
                .with_property("lvm.metadata-percent", "7.50")
                .with_property("lvm.active", "active")
                .with_property("lvm.when-full", "queue")
                .with_property("lvm.metadata-size", "8.00g"),
        );
        graph.add_node(
            Node::new("lvm-lv:vg0/root", NodeKind::LvmLogicalVolume, "vg0/root")
                .with_size_bytes(214_748_364_800)
                .with_property("lvm.active", "active")
                .with_property("lvm.active-locally", "active locally")
                .with_property("lvm.active-exclusively", "active exclusively")
                .with_property("lvm.layout", "thin")
                .with_property("lvm.pool", "pool")
                .with_property("lvm.dm-path", "/dev/mapper/vg0-root")
                .with_property("lvm.read-ahead", "auto")
                .with_property("lvm.kernel-read-ahead", "256")
                .with_property("lvm.suspended", "not suspended")
                .with_property("lvm.live-table", "live")
                .with_property("lvm.modules", "thin")
                .with_property("lvm.host", "host-a")
                .with_property("lvm.health", "ok"),
        );
        graph.add_node(
            Node::new(
                "lvm-snapshot:vg0/root-snap",
                NodeKind::LvmSnapshot,
                "vg0/root-snap",
            )
            .with_property("lvm.origin", "root")
            .with_property("lvm.snap-percent", "12.50")
            .with_property("lvm.active", "active"),
        );
        graph.add_node(
            Node::new("lvm-cache:vg0/root", NodeKind::LvmCache, "vg0/root")
                .with_property("lvm.cache-mode", "writeback")
                .with_property("lvm.cache-policy", "smq")
                .with_property("lvm.raid-mismatch-count", "2")
                .with_property("lvm.raid-sync-action", "repair")
                .with_property("lvm.raid-write-behind", "256")
                .with_property("lvm.raid-min-recovery-rate", "1024")
                .with_property("lvm.raid-max-recovery-rate", "8192")
                .with_property("lvm.raid-integrity-mode", "journal")
                .with_property("lvm.raid-integrity-block-size", "4096")
                .with_property("lvm.raid-integrity-mismatches", "1")
                .with_property("lvm.writecache-block-size", "4096")
                .with_property("lvm.writecache-writeback-blocks", "16"),
        );
        graph.add_node(
            Node::new("lvm-segment:vg0/root:0", NodeKind::LvmSegment, "vg0/root:0")
                .with_property("lvm.segment-type", "thin")
                .with_property("lvm.segment-stripes", "2")
                .with_property("lvm.segment-data-stripes", "2")
                .with_property("lvm.reshape-length", "128.00m")
                .with_property("lvm.data-copies", "2")
                .with_property("lvm.stripe-size", "64.00k")
                .with_property("lvm.segment-start", "0")
                .with_property("lvm.segment-size", "200.00g")
                .with_property("lvm.segment-size-extents", "51200")
                .with_property("lvm.devices", "pool(0)")
                .with_property("lvm.segment-le-ranges", "0-51199")
                .with_property("lvm.segment-metadata-le-ranges", "pool_tmeta:0-31")
                .with_property("lvm.integrity-settings", "journal_sectors=2048")
                .with_property("lvm.vdo-block-map-cache-size", "128.00m")
                .with_property("lvm.vdo-use-sparse-index", "enabled")
                .with_property("lvm.vdo-bio-threads", "4")
                .with_property("lvm.vdo-max-discard", "4.00m"),
        );
        graph.add_edge(Edge::new(
            "lvm-pv:/dev/nvme0n1p3",
            "lvm-vg:vg0",
            Relationship::MemberOf,
        ));
        graph.add_edge(Edge::new(
            "lvm-thin-pool:vg0/pool",
            "lvm-lv:vg0/root",
            Relationship::Backs,
        ));

        let mut output = Vec::new();
        print_lvm(&mut output, &graph).expect("lvm table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("DATA%"));
        assert!(output.contains("META%"));
        assert!(output.contains("/dev/nvme0n1p3"));
        assert!(output.contains("active"));
        assert!(output.contains("tags=ssd,system"));
        assert!(output.contains("pv-format=lvm2 dev-size=500.00g"));
        assert!(output.contains("pe-start=1.00m pv-extents=128000"));
        assert!(output.contains("pv-extents-used=102400 pv-mda-free=1020.00k"));
        assert!(output.contains("pv-device-id=wwn-0x1234"));
        assert!(output.contains("vg-format=lvm2"));
        assert!(output.contains("permissions=writeable"));
        assert!(output.contains("vg-autoactivation=enabled allocation=normal"));
        assert!(output.contains("system-id=host-a lock-type=none"));
        assert!(output.contains("extent=4.00m extents=262144 free-extents=5120"));
        assert!(output.contains("pvs=2 lvs=5 snapshots=1 seqno=17"));
        assert!(output.contains("vg-mda-free=1020.00k vg-mda-copies=unmanaged"));
        assert!(output.contains("42.00"));
        assert!(output.contains("7.50"));
        assert!(output.contains("when-full=queue metadata-size=8.00g"));
        assert!(output.contains("layout=thin pool=pool active=active active-local=active locally"));
        assert!(output.contains("active-exclusive=active exclusively"));
        assert!(output.contains("dm-path=/dev/mapper/vg0-root read-ahead=auto"));
        assert!(output.contains("kernel-read-ahead=256 suspended=not suspended"));
        assert!(output.contains("live-table=live modules=thin host=host-a"));
        assert!(output.contains("health=ok"));
        assert!(output.contains("snap=12.50 origin=root active=active"));
        assert!(output.contains("raid-mismatches=2 raid-sync=repair"));
        assert!(output.contains("raid-write-behind=256 raid-min-recovery=1024"));
        assert!(output.contains("raid-max-recovery=8192 raid-integrity=journal"));
        assert!(output.contains("raid-integrity-block=4096 raid-integrity-mismatches=1"));
        assert!(output.contains("cache-mode=writeback cache-policy=smq"));
        assert!(output.contains("writecache-writeback=16 writecache-block-size=4096"));
        assert!(output.contains("segment-type=thin stripes=2 data-stripes=2"));
        assert!(output.contains("reshape-length=128.00m data-copies=2"));
        assert!(output.contains("stripe-size=64.00k segment-start=0 segment-size=200.00g"));
        assert!(output.contains("segment-size-pe=51200 devices=pool(0) le-ranges=0-51199"));
        assert!(output.contains("metadata-le-ranges=pool_tmeta:0-31"));
        assert!(output.contains("integrity-settings=journal_sectors=2048"));
        assert!(output.contains("vdo-block-map-cache=128.00m vdo-sparse-index=enabled"));
        assert!(output.contains("vdo-bio-threads=4 vdo-max-discard=4.00m"));
    }

    #[test]
    fn iscsi_table_includes_session_target_lun_and_disk_details() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "iscsi-session:12",
                NodeKind::IscsiSession,
                "iscsi-session:12",
            )
            .with_property("iscsi.portal", "10.0.0.10:3260,1")
            .with_property("iscsi.portal-address", "10.0.0.10")
            .with_property("iscsi.portal-port", "3260")
            .with_property("iscsi.portal-tpgt", "1")
            .with_property("iscsi.persistent-portal", "10.0.0.11:3260,1")
            .with_property("iscsi.persistent-portal-address", "10.0.0.11")
            .with_property("iscsi.persistent-portal-port", "3260")
            .with_property("iscsi.persistent-portal-tpgt", "1")
            .with_property("iscsi.target-portal-group-tag", "1")
            .with_property("iscsi.connection-state", "LOGGED IN")
            .with_property("iscsi.connection-cid", "0")
            .with_property("iscsi.connection-detail-state", "LOGGED IN")
            .with_property("iscsi.connection-local-address", "10.0.0.20")
            .with_property("iscsi.connection-peer-address", "10.0.0.10"),
        );
        graph.add_node(
            Node::new(
                "iscsi-target:iqn.2026-06.example:storage",
                NodeKind::IscsiTarget,
                "iqn.2026-06.example:storage",
            )
            .with_property("iscsi.node-configured", "true")
            .with_property("iscsi.node-portal", "10.0.0.10:3260,1")
            .with_property("iscsi.node-portal-address", "10.0.0.10")
            .with_property("iscsi.node-portal-port", "3260")
            .with_property("iscsi.node-portal-tpgt", "1")
            .with_property("iscsi.node-startup", "automatic")
            .with_property("iscsi.node-iface-name", "default")
            .with_property("iscsi.node-auth-method", "CHAP"),
        );
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
            Node::new("block:/dev/sdb", NodeKind::PhysicalDisk, "/dev/sdb")
                .with_path("/dev/sdb")
                .with_property("iscsi.attached-disk", "sdb"),
        );
        graph.add_edge(Edge::new(
            "iscsi-session:12",
            "iscsi-target:iqn.2026-06.example:storage",
            Relationship::ImportedFrom,
        ));
        graph.add_edge(Edge::new(
            "iscsi-target:iqn.2026-06.example:storage",
            "iscsi-lun:iqn.2026-06.example:storage:0",
            Relationship::Contains,
        ));
        graph.add_edge(Edge::new(
            "iscsi-lun:iqn.2026-06.example:storage:0",
            "block:/dev/sdb",
            Relationship::Backs,
        ));

        let session = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "iscsi-session:12")
            .expect("session fixture exists");
        let target = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::IscsiTarget)
            .expect("target fixture exists");
        assert_eq!(iscsi_lun_count(&graph, session), 1);
        assert_eq!(iscsi_lun_count(&graph, target), 1);

        let mut output = Vec::new();
        print_iscsi(&mut output, &graph).expect("iscsi table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("PORTAL"));
        assert!(output.contains("STATE"));
        assert!(output.contains("LUNS"));
        assert!(output.contains("iscsi-session:12"));
        assert!(output.contains("10.0.0.10:3260,1"));
        assert!(output.contains("LOGGED IN"));
        assert!(output.contains("portal-address=10.0.0.10 portal-port=3260 portal-tpgt=1"));
        assert!(
            output
                .contains("persistent-portal=10.0.0.11:3260,1 persistent-portal-address=10.0.0.11")
        );
        assert!(output.contains("persistent-portal-port=3260 persistent-portal-tpgt=1"));
        assert!(output.contains("tpgt=1 connection-state=LOGGED IN"));
        assert!(output.contains("cid=0 connection-detail-state=LOGGED IN"));
        assert!(output.contains("local-address=10.0.0.20 peer-address=10.0.0.10"));
        assert!(output.contains("iqn.2026-06.example:storage"));
        assert!(output.contains("configured=true node-portal=10.0.0.10:3260,1"));
        assert!(output.contains("node-portal-address=10.0.0.10 node-portal-port=3260"));
        assert!(output.contains("node-portal-tpgt=1 node-iface=default startup=automatic"));
        assert!(output.contains("auth-method=CHAP"));
        assert!(output.contains("1.0 GiB"));
        assert!(output.contains("attached-disk=sdb"));
    }

    #[test]
    fn nfs_table_includes_exports_mounts_and_transport_details() {
        let mut graph = StorageGraph::empty();
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
            Node::new(
                "nfs-export:/srv/share:192.0.2.0/24",
                NodeKind::NfsExport,
                "/srv/share",
            )
            .with_property("nfs.export", "/srv/share")
            .with_property("nfs.export-client", "192.0.2.0/24")
            .with_property("nfs.exportfs", "true")
            .with_property("nfs.export-option-rw", "true")
            .with_property("nfs.export-option-sync", "true")
            .with_property("nfs.export-option-no-subtree-check", "true")
            .with_property("nfs.export-option-sec", "sys")
            .with_property("nfs.export-option-root-squash", "true"),
        );
        graph.add_node(
            Node::new("mount:/home", NodeKind::NfsMount, "/home")
                .with_size_bytes(1_099_511_627_776)
                .with_usage(Usage {
                    used_bytes: Some(274_877_906_944),
                    free_bytes: Some(824_633_720_832),
                    allocated_bytes: None,
                })
                .with_property("nfs.source", "storage.example:/export/home")
                .with_property("nfs.server", "storage.example")
                .with_property("nfs.export", "/export/home")
                .with_property("nfs.vers", "4.2")
                .with_property("nfs.proto", "tcp")
                .with_property("nfs.sec", "sys")
                .with_property("nfs.clientaddr", "10.0.0.20")
                .with_property("nfs.addr", "10.0.0.10")
                .with_property("nfs.port", "2049")
                .with_property("nfs.mountaddr", "10.0.0.10")
                .with_property("nfs.mountvers", "3")
                .with_property("nfs.mountproto", "tcp")
                .with_property("nfs.rsize", "1048576")
                .with_property("nfs.wsize", "1048576")
                .with_property("nfs.timeo", "600")
                .with_property("nfs.retrans", "2")
                .with_property("nfs.local-lock", "none")
                .with_property("nfs.lookupcache", "positive")
                .with_property("nfs.fsc", "true")
                .with_property("nfs.caps", "0x3fffdf")
                .with_property("nfs.wtmult", "512")
                .with_property("nfs.dtsize", "32768")
                .with_property("nfs.bsize", "0")
                .with_property("nfs.flavor", "1")
                .with_property("nfs.pseudoflavor", "1")
                .with_property("nfs.age", "123"),
        );
        graph.add_edge(Edge::new(
            "nfs-export:storage.example:/export/home",
            "mount:/home",
            Relationship::MountedAt,
        ));

        let export = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::NfsExport)
            .expect("export fixture exists");
        let mount = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::NfsMount)
            .expect("mount fixture exists");
        assert_eq!(nfs_mount_count(&graph, export), 1);
        assert_eq!(nfs_mount_count(&graph, mount), 0);

        let mut output = Vec::new();
        print_nfs(&mut output, &graph).expect("nfs table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("SOURCE"));
        assert!(output.contains("SERVER"));
        assert!(output.contains("EXPORT"));
        assert!(output.contains("MOUNTS"));
        assert!(output.contains("storage.example:/export/home"));
        assert!(output.contains("storage.example"));
        assert!(output.contains("/export/home"));
        assert!(output.contains("/home"));
        assert!(output.contains("source=storage.example:/export/home"));
        assert!(output.contains("vers=4.2 proto=tcp sec=sys"));
        assert!(output.contains("clientaddr=10.0.0.20 addr=10.0.0.10 port=2049"));
        assert!(output.contains("mountaddr=10.0.0.10 mountvers=3 mountproto=tcp"));
        assert!(output.contains("rsize=1048576 wsize=1048576 timeo=600 retrans=2"));
        assert!(output.contains("local-lock=none lookupcache=positive fsc=true age=123"));
        assert!(output.contains("caps=0x3fffdf wtmult=512 dtsize=32768 bsize=0"));
        assert!(output.contains("flavor=1 pseudoflavor=1"));
        assert!(output.contains("/srv/share"));
        assert!(output.contains("export-client=192.0.2.0/24 exportfs=true"));
        assert!(output.contains("export-rw=true export-sync=true"));
        assert!(output.contains("export-no-subtree-check=true export-sec=sys"));
        assert!(output.contains("export-root-squash=true"));
    }

    #[test]
    fn network_storage_table_includes_domain_metadata_details() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("iscsi-session:1", NodeKind::IscsiSession, "iscsi-session:1")
                .with_property("iscsi.portal", "10.0.0.10:3260,1")
                .with_property("iscsi.portal-address", "10.0.0.10")
                .with_property("iscsi.portal-port", "3260")
                .with_property("iscsi.portal-tpgt", "1")
                .with_property("iscsi.persistent-portal", "10.0.0.11:3260,1")
                .with_property("iscsi.persistent-portal-address", "10.0.0.11")
                .with_property("iscsi.persistent-portal-port", "3260")
                .with_property("iscsi.persistent-portal-tpgt", "1")
                .with_property("iscsi.connection-state", "LOGGED IN")
                .with_property("iscsi.session-state", "LOGGED_IN")
                .with_property("iscsi.internal-session-state", "NO CHANGE")
                .with_property("iscsi.iface-name", "default")
                .with_property("iscsi.iface-transport", "tcp")
                .with_property("iscsi.iface-initiator-name", "iqn.2026-06.client:node1")
                .with_property("iscsi.iface-ip-address", "10.0.0.20")
                .with_property("iscsi.iface-netdev", "eno1")
                .with_property("iscsi.host-number", "4")
                .with_property("iscsi.host-state", "running")
                .with_property("iscsi.headerdigest", "None")
                .with_property("iscsi.datadigest", "None")
                .with_property("iscsi.maxrecvdatasegmentlength", "262144")
                .with_property("iscsi.maxburstlength", "262144"),
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
            .with_property("iscsi.host-number", "4")
            .with_property("iscsi.scsi-channel", "00")
            .with_property("iscsi.scsi-id", "0")
            .with_property("iscsi.attached-disk", "sdb")
            .with_property("iscsi.attached-disk-state", "running"),
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
                .with_property("nfs.port", "2049")
                .with_property("nfs.mountaddr", "10.0.0.10")
                .with_property("nfs.mountvers", "3")
                .with_property("nfs.mountproto", "tcp")
                .with_property("nfs.rsize", "1048576")
                .with_property("nfs.wsize", "1048576")
                .with_property("nfs.timeo", "600")
                .with_property("nfs.retrans", "2")
                .with_property("nfs.local-lock", "none")
                .with_property("nfs.lookupcache", "positive")
                .with_property("nfs.fsc", "true")
                .with_property("nfs.caps", "0x3fffdf")
                .with_property("nfs.wtmult", "512")
                .with_property("nfs.dtsize", "32768")
                .with_property("nfs.bsize", "0")
                .with_property("nfs.flavor", "1")
                .with_property("nfs.pseudoflavor", "1")
                .with_property("nfs.age", "123"),
        );

        let mut output = Vec::new();
        print_network_storage(&mut output, &graph).expect("network storage table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("DETAILS"));
        assert!(output.contains("portal=10.0.0.10:3260,1"));
        assert!(output.contains("portal-address=10.0.0.10 portal-port=3260 portal-tpgt=1"));
        assert!(output.contains("persistent-portal=10.0.0.11:3260,1"));
        assert!(output.contains(
            "persistent-portal-address=10.0.0.11 persistent-portal-port=3260 persistent-portal-tpgt=1"
        ));
        assert!(output.contains("connection-state=LOGGED IN"));
        assert!(output.contains("session-state=LOGGED_IN"));
        assert!(output.contains("internal-session-state=NO CHANGE"));
        assert!(output.contains("iface=default transport=tcp"));
        assert!(output.contains("initiator=iqn.2026-06.client:node1"));
        assert!(output.contains("iface-ip=10.0.0.20 netdev=eno1"));
        assert!(output.contains("host=4 host-state=running"));
        assert!(output.contains("header-digest=None data-digest=None"));
        assert!(output.contains("max-recv-data-segment=262144"));
        assert!(output.contains("max-burst=262144"));
        assert!(output.contains("scsi-channel=00 scsi-id=0"));
        assert!(output.contains("attached-disk=sdb attached-disk-state=running"));
        assert!(output.contains("server=storage.example export=/export/home"));
        assert!(output.contains(
            "source=storage.example:/export/home server=storage.example export=/export/home vers=4.2"
        ));
        assert!(output.contains("proto=tcp sec=sys clientaddr=10.0.0.20 addr=10.0.0.10"));
        assert!(output.contains("mountaddr=10.0.0.10 mountvers=3 mountproto=tcp"));
        assert!(output.contains("rsize=1048576 wsize=1048576 timeo=600 retrans=2"));
        assert!(output.contains("local-lock=none lookupcache=positive fsc=true age=123"));
        assert!(output.contains("caps=0x3fffdf wtmult=512 dtsize=32768 bsize=0"));
        assert!(output.contains("flavor=1 pseudoflavor=1"));
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
            .with_property("btrfs.generation", "11")
            .with_property("btrfs.created-generation", "8")
            .with_property("btrfs.parent-id", "256")
            .with_property("btrfs.top-level", "5")
            .with_property("btrfs.parent-uuid", "subvol-root")
            .with_property("btrfs.received-uuid", "received-snap"),
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
        assert!(output.contains("subvol-id=257 generation=11 created-generation=8 parent-id=256"));
        assert!(output.contains("top-level=5 parent-uuid=subvol-root received-uuid=received-snap"));
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
                .with_property("md.version", "1.2")
                .with_property("md.level", "raid1")
                .with_property("md.state", "clean")
                .with_property("md.raid-devices", "2")
                .with_property("md.total-devices", "2")
                .with_property("md.name", "host:root")
                .with_property("md.events", "17"),
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
        assert!(output.contains(
            "md-version=1.2 level=raid1 state=clean raid-devices=2 total-devices=2 md-name=host:root events=17"
        ));
    }

    #[test]
    fn encryption_table_includes_luks_header_details() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "block:/dev/mapper/cryptroot",
                NodeKind::LuksContainer,
                "cryptroot",
            )
            .with_path("/dev/mapper/cryptroot")
            .with_property("cryptsetup.active", "true")
            .with_property("cryptsetup.in-use", "true")
            .with_property("cryptsetup.cipher", "aes-xts-plain64")
            .with_property("cryptsetup.luks-version", "2")
            .with_property("cryptsetup.luks-epoch", "7")
            .with_property("cryptsetup.luks-metadata-area", "16384 [bytes]")
            .with_property("cryptsetup.luks-keyslots-area", "16744448 [bytes]")
            .with_property("cryptsetup.luks-keyslot-count", "2")
            .with_property("cryptsetup.luks-token-count", "1")
            .with_property("cryptsetup.luks-keyslots", "0,1")
            .with_property("cryptsetup.luks-tokens", "0")
            .with_property("cryptsetup.luks-keyslot-0-type", "luks2")
            .with_property("cryptsetup.luks-keyslot-0-priority", "normal")
            .with_property("cryptsetup.luks-keyslot-0-cipher", "aes-xts-plain64")
            .with_property("cryptsetup.luks-keyslot-0-cipher-key", "512 bits")
            .with_property("cryptsetup.luks-keyslot-0-pbkdf", "argon2id")
            .with_property("cryptsetup.luks-keyslot-0-time-cost", "4")
            .with_property("cryptsetup.luks-keyslot-0-memory", "1048576")
            .with_property("cryptsetup.luks-keyslot-0-threads", "4")
            .with_property("cryptsetup.luks-keyslot-1-type", "luks2")
            .with_property("cryptsetup.luks-keyslot-1-priority", "ignored")
            .with_property("cryptsetup.luks-token-0-type", "systemd-tpm2")
            .with_property("cryptsetup.luks-token-0-keyslot", "0"),
        );

        let mut output = Vec::new();
        print_encryption(&mut output, &graph).expect("encryption table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("CIPHER"));
        assert!(output.contains("KEYSLOTS"));
        assert!(output.contains("TOKENS"));
        assert!(output.contains("cryptroot"));
        assert!(output.contains("aes-xts-plain64"));
        assert!(output.contains(" 2         "));
        assert!(output.contains(" 1         "));
        assert!(output.contains("active=true in-use=true cipher=aes-xts-plain64"));
        assert!(output.contains("luks=2 epoch=7 metadata-area=16384 [bytes]"));
        assert!(output.contains("keyslot-ids=0,1 token-ids=0"));
        assert!(output.contains(
            "keyslot-0=luks2 keyslot-0-priority=normal keyslot-0-cipher=aes-xts-plain64"
        ));
        assert!(output.contains(
            "keyslot-0-cipher-key=512 bits keyslot-0-pbkdf=argon2id keyslot-0-time=4 keyslot-0-memory=1048576 keyslot-0-threads=4"
        ));
        assert!(output.contains(
            "keyslot-1=luks2 keyslot-1-priority=ignored token-0=systemd-tpm2 token-0-keyslot=0"
        ));
    }

    #[test]
    fn cache_table_includes_cache_layer_details() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("block:/dev/bcache0", NodeKind::CacheDevice, "bcache0")
                .with_path("/dev/bcache0")
                .with_property("bcache.role", "backing")
                .with_property("bcache.kind", "cache-set")
                .with_property("bcache.backing-device", "/dev/sdb1")
                .with_property("bcache.set-uuid", "cache-set-uuid")
                .with_property("bcache.state", "clean")
                .with_property("bcache.running", "1")
                .with_property("bcache.cache-available-percent", "78")
                .with_property("bcache.cache-mode", "writeback")
                .with_property("bcache.cache-replacement-policy", "lru")
                .with_property("bcache.dirty-data", "64.0M")
                .with_property("bcache.io-errors", "0")
                .with_property("bcache.metadata-written", "128.0M")
                .with_property("bcache.writeback-delay", "30")
                .with_property("bcache.writeback-running", "1"),
        );
        graph.add_node(
            Node::new("lvm-lv:vg/root", NodeKind::LvmLogicalVolume, "vg/root")
                .with_property("lvm.cache-mode", "writethrough")
                .with_property("lvm.cache-policy", "smq")
                .with_property("lvm.cache-total-blocks", "4096")
                .with_property("lvm.cache-used-blocks", "1024")
                .with_property("lvm.cache-dirty-blocks", "64")
                .with_property("lvm.cache-read-hits", "1000")
                .with_property("lvm.cache-read-misses", "25")
                .with_property("lvm.cache-write-hits", "900")
                .with_property("lvm.cache-write-misses", "30")
                .with_property("lvm.cache-promotions", "128")
                .with_property("lvm.cache-demotions", "32")
                .with_property("lvm.kernel-cache-settings", "migration_threshold=2048")
                .with_property("lvm.kernel-metadata-format", "2")
                .with_property("lvm.writecache-total-blocks", "1024")
                .with_property("lvm.writecache-free-blocks", "512")
                .with_property("lvm.writecache-writeback-blocks", "16")
                .with_property("lvm.writecache-error", "0"),
        );
        graph.add_node(
            Node::new(
                "zfs-vdev:tank:cache0",
                NodeKind::ZfsVdev,
                "/dev/disk/by-id/cache0",
            )
            .with_property("zfs.vdev-role", "cache")
            .with_property("zfs.vdev-state", "ONLINE"),
        );

        let mut output = Vec::new();
        print_cache(&mut output, &graph).expect("cache table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("MODE"));
        assert!(output.contains("POLICY"));
        assert!(output.contains("DIRTY"));
        assert!(output.contains("bcache0"));
        assert!(output.contains("writeback"));
        assert!(output.contains("lru"));
        assert!(output.contains("backing-device=/dev/sdb1"));
        assert!(output.contains("dirty=64.0M"));
        assert!(output.contains("running=1 available-percent=78"));
        assert!(output.contains("io-errors=0 metadata-written=128.0M"));
        assert!(output.contains("writeback-delay=30"));
        assert!(output.contains("writeback-running=1"));
        assert!(output.contains("vg/root"));
        assert!(output.contains("writethrough"));
        assert!(output.contains("cache-policy=smq"));
        assert!(output.contains("cache-total=4096"));
        assert!(output.contains("cache-used=1024"));
        assert!(output.contains("cache-dirty=64"));
        assert!(output.contains("cache-read-hits=1000"));
        assert!(output.contains("cache-read-misses=25"));
        assert!(output.contains("cache-write-hits=900"));
        assert!(output.contains("cache-write-misses=30"));
        assert!(output.contains("cache-promotions=128"));
        assert!(output.contains("cache-demotions=32"));
        assert!(output.contains("kernel-cache-settings=migration_threshold=2048"));
        assert!(output.contains("kernel-metadata-format=2"));
        assert!(output.contains("writecache-total=1024"));
        assert!(output.contains("writecache-free=512"));
        assert!(output.contains("writecache-writeback=16"));
        assert!(output.contains("writecache-error=0"));
        assert!(output.contains("/dev/disk/by-id/cache0"));
        assert!(output.contains("vdev-role=cache vdev-state=ONLINE"));
    }

    #[test]
    fn vdo_table_includes_vdo_reduction_details() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("vdo:archive", NodeKind::VdoVolume, "archive")
                .with_property("vdo.storage-device", "/dev/sdb")
                .with_property("vdo.logical-size", "1T")
                .with_property("vdo.physical-size", "250G")
                .with_property("vdo.stats-size", "268435456")
                .with_property("vdo.stats-used", "134217728")
                .with_property("vdo.stats-available", "134217728")
                .with_property("vdo.use-percent", "50%")
                .with_property("vdo.space-saving-percent", "75%")
                .with_property("vdo.operating-mode", "normal")
                .with_property("vdo.recovery-percentage", "100%")
                .with_property("vdo.write-policy", "sync")
                .with_property("vdo.configured-write-policy", "auto")
                .with_property("vdo.index-memory-setting", "0.25")
                .with_property("vdo.block-map-cache-size", "128M")
                .with_property("vdo.compression", "enabled")
                .with_property("vdo.deduplication", "enabled")
                .with_property("vdo.version", "47")
                .with_property("vdo.release-version", "133524")
                .with_property("vdo.data-blocks-used", "65536")
                .with_property("vdo.overhead-blocks-used", "4096")
                .with_property("vdo.logical-blocks-used", "262144"),
        );
        graph.add_node(
            Node::new(
                "lvm-seg:vg0/archive:0",
                NodeKind::LvmSegment,
                "vg0/archive:0",
            )
            .with_property("lvm.segment-type", "vdo")
            .with_property("lvm.vdo-operating-mode", "normal")
            .with_property("lvm.vdo-compression", "enabled")
            .with_property("lvm.vdo-compression-state", "online")
            .with_property("lvm.vdo-deduplication", "disabled")
            .with_property("lvm.vdo-index-state", "online")
            .with_property("lvm.vdo-used-size", "8.00g")
            .with_property("lvm.vdo-saving-percent", "42.00")
            .with_property("lvm.vdo-write-policy", "auto"),
        );

        let mut output = Vec::new();
        print_vdo(&mut output, &graph).expect("vdo table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("LOGICAL"));
        assert!(output.contains("PHYSICAL"));
        assert!(output.contains("WRITE"));
        assert!(output.contains("archive"));
        assert!(output.contains("          1T"));
        assert!(output.contains("        250G"));
        assert!(output.contains("normal"));
        assert!(output.contains("sync"));
        assert!(output.contains("backing=/dev/sdb logical=1T physical=250G"));
        assert!(output.contains("stats-size=268435456 stats-used=134217728"));
        assert!(output.contains("vdo-use=50% saving=75%"));
        assert!(output.contains("recovery=100% write-policy=sync configured-write-policy=auto"));
        assert!(output.contains("index-memory=0.25 block-map-cache=128M"));
        assert!(output.contains("compression=enabled deduplication=enabled"));
        assert!(output.contains("vdo-version=47 vdo-release=133524"));
        assert!(output.contains("data-blocks=65536 overhead-blocks=4096 logical-blocks=262144"));
        assert!(output.contains("vg0/archive:0"));
        assert!(output.contains("vdo-mode=normal"));
        assert!(output.contains("vdo-compression-state=online"));
        assert!(output.contains("vdo-index-state=online"));
        assert!(output.contains("vdo-used=8.00g"));
        assert!(output.contains("vdo-saving=42.00"));
        assert!(output.contains("vdo-write-policy=auto"));
    }

    #[test]
    fn multipath_table_includes_map_and_path_details() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("multipath:mpatha", NodeKind::MultipathDevice, "mpatha")
                .with_path("/dev/mapper/mpatha")
                .with_property("multipath.dm", "dm-2")
                .with_property("multipath.wwid", "3600508b400105e210000900000490000")
                .with_property("multipath.vendor-product", "IBM,2145")
                .with_property("multipath.size", "100G")
                .with_property("multipath.features", "1 queue_if_no_path")
                .with_property("multipath.hwhandler", "1 alua")
                .with_property("multipath.write-protect", "rw"),
        );
        graph.add_node(
            Node::new("block:/dev/sdb", NodeKind::PhysicalDisk, "/dev/sdb")
                .with_path("/dev/sdb")
                .with_property("multipath.host-path", "2:0:0:1")
                .with_property("multipath.scsi-host", "2")
                .with_property("multipath.scsi-channel", "0")
                .with_property("multipath.scsi-id", "0")
                .with_property("multipath.scsi-lun", "1")
                .with_property("major-minor", "8:16")
                .with_property("multipath.group-policy", "service-time 0")
                .with_property("multipath.group-prio", "50")
                .with_property("multipath.group-status", "active")
                .with_property("multipath.dm-state", "active")
                .with_property("multipath.checker-state", "ready")
                .with_property("multipath.online-state", "running")
                .with_property("multipath.path-state", "active ready running"),
        );
        graph.add_node(
            Node::new("block:/dev/sdc", NodeKind::PhysicalDisk, "/dev/sdc")
                .with_path("/dev/sdc")
                .with_property("multipath.host-path", "3:0:0:1")
                .with_property("multipath.scsi-host", "3")
                .with_property("multipath.scsi-channel", "0")
                .with_property("multipath.scsi-id", "0")
                .with_property("multipath.scsi-lun", "1")
                .with_property("major-minor", "8:32")
                .with_property("multipath.group-policy", "service-time 0")
                .with_property("multipath.group-prio", "10")
                .with_property("multipath.group-status", "enabled")
                .with_property("multipath.dm-state", "active")
                .with_property("multipath.checker-state", "ready")
                .with_property("multipath.online-state", "running")
                .with_property("multipath.path-state", "active ready running"),
        );
        graph.add_edge(Edge::new(
            "block:/dev/sdb",
            "multipath:mpatha",
            Relationship::Backs,
        ));
        graph.add_edge(Edge::new(
            "block:/dev/sdc",
            "multipath:mpatha",
            Relationship::Backs,
        ));

        let mut output = Vec::new();
        print_multipath(&mut output, &graph).expect("multipath table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("WWID"));
        assert!(output.contains("PATHS"));
        assert!(output.contains("GROUP"));
        assert!(output.contains("PATH-STATE"));
        assert!(output.contains("mpatha"));
        assert!(output.contains("3600508b400105e210000900000490000"));
        assert!(output.contains("dm=dm-2 wwid=3600508b400105e210000900000490000"));
        assert!(output.contains("vendor=IBM,2145 size=100G"));
        assert!(output.contains("features=1 queue_if_no_path handler=1 alua wp=rw"));
        assert!(output.contains("/dev/sdb"));
        assert!(output.contains("host-path=2:0:0:1 scsi-host=2"));
        assert!(output.contains("scsi-host=2 scsi-channel=0 scsi-id=0 scsi-lun=1"));
        assert!(output.contains("scsi-lun=1 major-minor=8:16"));
        assert!(output.contains("group-policy=service-time 0 group-prio=50 group-status=active"));
        assert!(output.contains("dm-state=active checker-state=ready online-state=running"));
        assert!(output.contains("path-state=active ready running"));
        assert!(output.contains("/dev/sdc"));
        assert!(output.contains("host-path=3:0:0:1 scsi-host=3"));
        assert!(output.contains("scsi-host=3 scsi-channel=0 scsi-id=0 scsi-lun=1"));
        assert!(output.contains("scsi-lun=1 major-minor=8:32"));
        assert!(output.contains("group-policy=service-time 0 group-prio=10 group-status=enabled"));
    }

    #[test]
    fn nvme_table_includes_namespace_identity_and_geometry() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "block:/dev/nvme0n1",
                NodeKind::NvmeNamespace,
                "/dev/nvme0n1",
            )
            .with_path("/dev/nvme0n1")
            .with_size_bytes(1_000_000_000_000)
            .with_usage(Usage {
                used_bytes: Some(400_000_000_000),
                free_bytes: Some(600_000_000_000),
                allocated_bytes: Some(400_000_000_000),
            })
            .with_identity(Identity {
                serial: Some("SERIAL123".to_string()),
                ..Identity::default()
            })
            .with_property("nvme.generic-path", "/dev/ng0n1")
            .with_property("nvme.model", "Example NVMe")
            .with_property("nvme.product", "Example Controller")
            .with_property("nvme.firmware", "1.0")
            .with_property("nvme.index", "0")
            .with_property("nvme.namespace", "1")
            .with_property("nvme.subsystem", "nvme-subsys0")
            .with_property("nvme.controller", "nvme0")
            .with_property("nvme.address", "0000:01:00.0")
            .with_property("nvme.transport", "pcie")
            .with_property("nvme.controller-id", "1")
            .with_property("nvme.namespace-capacity", "900000000000")
            .with_property("nvme.lba-format", "512 B + 0 B")
            .with_property("nvme.maximum-lba", "1953125")
            .with_property("nvme.sector-size", "512"),
        );

        let mut output = Vec::new();
        print_nvme(&mut output, &graph).expect("nvme table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("SERIAL"));
        assert!(output.contains("CONTROLLER"));
        assert!(output.contains("USE%"));
        assert!(output.contains("/dev/nvme0n1"));
        assert!(output.contains("SERIAL123"));
        assert!(output.contains("nvme0"));
        assert!(output.contains("40.0%"));
        assert!(output.contains("generic=/dev/ng0n1 nvme-model=Example NVMe"));
        assert!(output.contains("product=Example Controller firmware=1.0"));
        assert!(output.contains("ns-index=0 namespace=1 subsystem=nvme-subsys0"));
        assert!(output.contains("controller=nvme0 address=0000:01:00.0"));
        assert!(output.contains(
            "transport=pcie controller-id=1 namespace-capacity=900000000000 lba-format=512 B + 0 B"
        ));
        assert!(output.contains("max-lba=1953125 sector-size=512"));
    }

    #[test]
    fn raid_table_includes_array_and_member_details() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("md:/dev/md0", NodeKind::MdRaid, "/dev/md0")
                .with_path("/dev/md0")
                .with_size_bytes(1_071_644_672)
                .with_identity(Identity {
                    uuid: Some("aaaa:bbbb:cccc:dddd".to_string()),
                    ..Identity::default()
                })
                .with_property("md.version", "1.2")
                .with_property("md.level", "raid1")
                .with_property("md.state", "clean")
                .with_property("md.raid-devices", "2")
                .with_property("md.total-devices", "2")
                .with_property("md.array-devices", "2")
                .with_property("md.active-devices", "1")
                .with_property("md.working-devices", "2")
                .with_property("md.failed-devices", "1")
                .with_property("md.spare-devices", "1")
                .with_property("md.degraded-devices", "1")
                .with_property("md.name", "host:0")
                .with_property("md.creation-time", "Tue Jun 23 10:15:00 2026")
                .with_property("md.update-time", "Tue Jun 23 10:16:00 2026")
                .with_property("md.events", "17")
                .with_property("md.chunk-size", "512K")
                .with_property("md.layout", "near=2")
                .with_property("md.consistency-policy", "bitmap")
                .with_property("md.rebuild-status", "42% complete")
                .with_property("md.resync-status", "delayed")
                .with_property("md.check-status", "10% complete")
                .with_property("md.intent-bitmap", "Internal"),
        );
        graph.add_node(
            Node::new("md:/dev/md/root", NodeKind::MdRaid, "/dev/md/root")
                .with_path("/dev/md/root")
                .with_identity(Identity {
                    uuid: Some("eeee:ffff:1111:2222".to_string()),
                    ..Identity::default()
                })
                .with_property("md.scan-metadata", "1.2")
                .with_property("md.scan-name", "host:root")
                .with_property("md.scan-spares", "1")
                .with_property("md.scan-devices", "/dev/sdc1,/dev/sdd1"),
        );
        graph.add_node(
            Node::new("block:/dev/sda1", NodeKind::Partition, "/dev/sda1")
                .with_path("/dev/sda1")
                .with_property("md.member-number", "0")
                .with_property("md.member-major", "8")
                .with_property("md.member-minor", "1")
                .with_property("md.member-raid-device", "0")
                .with_property("md.member-state", "active sync"),
        );
        graph.add_node(
            Node::new("block:/dev/sdb1", NodeKind::Partition, "/dev/sdb1")
                .with_path("/dev/sdb1")
                .with_property("md.member-number", "1")
                .with_property("md.member-major", "8")
                .with_property("md.member-minor", "17")
                .with_property("md.member-raid-device", "1")
                .with_property("md.member-state", "active sync"),
        );
        graph.add_edge(Edge::new(
            "block:/dev/sda1",
            "md:/dev/md0",
            Relationship::MemberOf,
        ));
        graph.add_edge(Edge::new(
            "block:/dev/sdb1",
            "md:/dev/md0",
            Relationship::MemberOf,
        ));

        let mut output = Vec::new();
        print_raid(&mut output, &graph).expect("raid table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("LEVEL"));
        assert!(output.contains("STATE"));
        assert!(output.contains("ACTIVE"));
        assert!(output.contains("FAILED"));
        assert!(output.contains("SPARE"));
        assert!(output.contains("MEMBERS"));
        assert!(output.contains("/dev/md0"));
        assert!(output.contains("raid1"));
        assert!(output.contains("clean"));
        assert!(output.contains("md-version=1.2 level=raid1 state=clean"));
        assert!(output.contains("raid-devices=2 total-devices=2 array-devices=2"));
        assert!(output.contains("active-devices=1 working-devices=2 failed-devices=1"));
        assert!(output.contains("spare-devices=1 degraded-devices=1"));
        assert!(output.contains("md-name=host:0"));
        assert!(output.contains("created=Tue Jun 23 10:15:00 2026"));
        assert!(output.contains("updated=Tue Jun 23 10:16:00 2026"));
        assert!(output.contains("events=17"));
        assert!(output.contains("chunk=512K layout=near=2"));
        assert!(output.contains("consistency=bitmap rebuild=42% complete"));
        assert!(output.contains("resync=delayed check=10% complete bitmap=Internal"));
        assert!(output.contains("/dev/md/root"));
        assert!(output.contains("scan-metadata=1.2 scan-name=host:root"));
        assert!(output.contains("scan-spares=1 scan-devices=/dev/sdc1,/dev/sdd1"));
        assert!(output.contains("/dev/sda1"));
        assert!(output.contains("active sync"));
        assert!(
            output.contains("member-number=0 member-major=8 member-minor=1 member-raid-device=0")
        );
        assert!(output.contains("member-state=active sync"));
        assert!(output.contains("/dev/sdb1"));
        assert!(
            output.contains("member-number=1 member-major=8 member-minor=17 member-raid-device=1")
        );
    }

    #[test]
    fn loop_table_includes_mapping_and_backing_details() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("block:/dev/loop0", NodeKind::LoopDevice, "/dev/loop0")
                .with_path("/dev/loop0")
                .with_property("loop.back-file", "/var/lib/images/root.img")
                .with_property("loop.major-minor", "7:0")
                .with_property("loop.offset", "1048576")
                .with_property("loop.sizelimit", "0")
                .with_property("loop.logical-sector-size", "512")
                .with_property("loop.autoclear", "true")
                .with_property("loop.read-only", "false")
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
            Node::new("block:/dev/loop1", NodeKind::LoopDevice, "/dev/loop1")
                .with_path("/dev/loop1")
                .with_size_bytes(1_073_741_824)
                .with_property("loop.back-file", "/dev/disk/by-id/nvme-loop-backing")
                .with_property("loop.offset", "0")
                .with_property("loop.sizelimit", "1073741824")
                .with_property("loop.read-only", "true"),
        );
        graph.add_edge(Edge::new(
            "file:/var/lib/images/root.img",
            "block:/dev/loop0",
            Relationship::Backs,
        ));

        let mut output = Vec::new();
        print_loop(&mut output, &graph).expect("loop table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("BACKING"));
        assert!(output.contains("OFFSET"));
        assert!(output.contains("/dev/loop0"));
        assert!(output.contains("/var/lib/images/root.img"));
        assert!(output.contains("1048576"));
        assert!(output.contains("ro=false"));
        assert!(output.contains("back-file=/var/lib/images/root.img major-minor=7:0"));
        assert!(output.contains("logical-sector=512 autoclear=true ro=false dio=true"));
        assert!(output.contains("loop-backing=true"));
        assert!(output.contains("/dev/loop1"));
        assert!(output.contains("1.0 GiB"));
        assert!(output.contains("/dev/disk/by-id/nvme-loop-backing"));
        assert!(output.contains("sizelimit=1073741824"));
    }

    #[test]
    fn swap_table_includes_active_swap_usage_and_priority() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("block:/dev/sda3", NodeKind::Swap, "/dev/sda3")
                .with_path("/dev/sda3")
                .with_size_bytes(9_448_955_904)
                .with_usage(Usage {
                    used_bytes: Some(53_592_064),
                    free_bytes: Some(9_395_363_840),
                    allocated_bytes: Some(9_448_955_904),
                })
                .with_property("swap.active", "true")
                .with_property("swap.type", "partition")
                .with_property("swap.priority", "-2"),
        );
        graph.add_node(
            Node::new("swap:/dev/sda3", NodeKind::Swap, "/dev/sda3")
                .with_path("/dev/sda3")
                .with_size_bytes(9_448_955_904)
                .with_usage(Usage {
                    used_bytes: Some(53_592_064),
                    free_bytes: Some(9_395_363_840),
                    allocated_bytes: Some(9_448_955_904),
                })
                .with_property("swap.active", "true")
                .with_property("swap.type", "partition")
                .with_property("swap.priority", "-2"),
        );
        graph.add_node(
            Node::new("swap:/swapfile", NodeKind::Swap, "/swapfile")
                .with_path("/swapfile")
                .with_size_bytes(1_073_741_824)
                .with_usage(Usage {
                    used_bytes: Some(0),
                    free_bytes: Some(1_073_741_824),
                    allocated_bytes: Some(1_073_741_824),
                })
                .with_property("swap.active", "true")
                .with_property("swap.type", "file")
                .with_property("swap.priority", "10"),
        );
        graph.add_edge(Edge::new(
            "block:/dev/sda3",
            "swap:/dev/sda3",
            Relationship::Backs,
        ));

        let mut output = Vec::new();
        print_swap(&mut output, &graph).expect("swap table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("TYPE"));
        assert!(output.contains("PRIO"));
        assert!(output.contains("/dev/sda3"));
        assert!(output.contains("partition"));
        assert!(output.contains("-2"));
        assert!(output.contains("swap-active=true swap-type=partition swap-priority=-2"));
        assert!(output.contains("/swapfile"));
        assert!(output.contains("file"));
        assert!(output.contains("10"));
        assert!(output.contains("swap-active=true swap-type=file swap-priority=10"));
        assert!(output.contains("0.0%"));
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
            .with_property("dm.name", "cryptroot")
            .with_property("dm.uuid", "CRYPT-LUKS2-crypt-uuid-cryptroot")
            .with_property("dm.major", "253")
            .with_property("dm.minor", "0")
            .with_property("dm.open-count", "1")
            .with_property("dm.segments", "1")
            .with_property("dm.events", "0")
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
            .with_property("cryptsetup.luks-keyslot-0-type", "luks2")
            .with_property("cryptsetup.luks-keyslot-0-priority", "normal")
            .with_property("cryptsetup.luks-keyslot-0-cipher", "aes-xts-plain64")
            .with_property("cryptsetup.luks-keyslot-0-cipher-key", "512 bits")
            .with_property("cryptsetup.luks-keyslot-0-pbkdf", "argon2id")
            .with_property("cryptsetup.luks-keyslot-0-time-cost", "4")
            .with_property("cryptsetup.luks-keyslot-0-memory", "1048576")
            .with_property("cryptsetup.luks-keyslot-0-threads", "4")
            .with_property("cryptsetup.luks-keyslot-1-type", "luks2")
            .with_property("cryptsetup.luks-keyslot-1-priority", "ignored")
            .with_property("cryptsetup.luks-token-0-type", "systemd-tpm2")
            .with_property("cryptsetup.luks-token-0-keyslot", "0")
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
        let segment = Node::new(
            "lvm-seg:vg0/thinpool:0",
            NodeKind::LvmSegment,
            "vg0/thinpool:0",
        )
        .with_property("lvm.segment-type", "thin-pool")
        .with_property("lvm.segment-start", "0")
        .with_property("lvm.segment-size", "100.00g")
        .with_property("lvm.chunk-size", "64.00k")
        .with_property("lvm.thin-count", "3")
        .with_property("lvm.discards", "passdown")
        .with_property("lvm.zero", "zero")
        .with_property("lvm.transaction-id", "42")
        .with_property("lvm.devices", "thinpool_tdata(0)")
        .with_property("lvm.metadata-devices", "thinpool_tmeta(0)")
        .with_property("lvm.segment-monitor", "monitored")
        .with_property("lvm.cache-metadata-format", "2")
        .with_property("lvm.segment-cache-mode", "writeback")
        .with_property("lvm.segment-cache-policy", "smq")
        .with_property("lvm.cache-settings", "migration_threshold=2048")
        .with_property("lvm.vdo-compression", "enabled")
        .with_property("lvm.vdo-deduplication", "enabled")
        .with_property("lvm.vdo-write-policy", "auto");
        let segment_details = usage_details(&segment);
        assert!(segment_details.contains("segment-type=thin-pool"));
        assert!(segment_details.contains("metadata-devices=thinpool_tmeta(0)"));
        assert!(segment_details.contains("segment-cache-policy=smq"));
        assert!(segment_details.contains("vdo-write-policy=auto"));
        graph.add_node(segment);
        graph.add_node(
            Node::new("block:/dev/bcache0", NodeKind::CacheDevice, "bcache0")
                .with_path("/dev/bcache0")
                .with_property("bcache.role", "backing")
                .with_property("bcache.kind", "cache-set")
                .with_property("bcache.label", "fast-cache")
                .with_property("bcache.state", "clean")
                .with_property("bcache.running", "1")
                .with_property("bcache.cache-available-percent", "78")
                .with_property("bcache.cache-mode", "writeback")
                .with_property("bcache.discard", "true")
                .with_property("bcache.io-errors", "0")
                .with_property("bcache.readahead", "0")
                .with_property("bcache.sequential-cutoff", "4.0M")
                .with_property("bcache.written", "512.0M")
                .with_property("bcache.writeback-rate", "1.0M/sec"),
        );

        let mut output = Vec::new();
        print_mappings(&mut output, &graph).expect("mappings table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("DETAILS"));
        assert!(output.contains("cryptroot"));
        assert!(output.contains(
            "dm-name=cryptroot dm-uuid=CRYPT-LUKS2-crypt-uuid-cryptroot dm-major=253 dm-minor=0 open=1 segments=1 events=0"
        ));
        assert!(
            output.contains(
                "active=true in-use=true cipher=aes-xts-plain64 luks=2 epoch=7 metadata-area=16384 [bytes] keyslots-area=16744448 [bytes] subsystem=(no subsystem) flags=allow-discards keyslots=2 tokens=1 keyslot-ids=0,1 token-ids=0 keyslot-0=luks2 keyslot-0-priority=normal"
            )
        );
        assert!(output.contains(
            "keyslot-0-cipher=aes-xts-plain64 keyslot-0-cipher-key=512 bits keyslot-0-pbkdf=argon2id keyslot-0-time=4 keyslot-0-memory=1048576 keyslot-0-threads=4"
        ));
        assert!(output.contains(
            "keyslot-1=luks2 keyslot-1-priority=ignored token-0=systemd-tpm2 token-0-keyslot=0 data-cipher=aes-xts-plain64"
        ));
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
        assert!(output.contains("vg0/thinpool:0"));
        assert!(output.contains("segment-type=thin-pool"));
        assert!(output.contains("metadata-devices=thinpool_tmeta(0)"));
        assert!(output.contains("segment-cache-policy=smq"));
        assert!(output.contains("vdo-write-policy=auto"));
        assert!(output.contains(
            "role=backing kind=cache-set label=fast-cache state=clean running=1 available-percent=78 cache-mode=writeback discard=true io-errors=0 readahead=0 sequential-cutoff=4.0M written=512.0M writeback-rate=1.0M/sec"
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
