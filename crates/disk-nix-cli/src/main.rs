#![recursion_limit = "512"]

use std::{
    collections::{BTreeSet, VecDeque},
    fmt, fs,
    io::{self, Write},
    os::unix::fs::PermissionsExt,
    process::Command as ProcessCommand,
    process::ExitCode,
    time::{SystemTime, UNIX_EPOCH},
};

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use clap_mangen::Man;
use disk_nix_exec::{ExecutionMode, ExecutionReport, ExecutionStatus, prepare_execution};
use disk_nix_model::{Node, NodeKind, StorageGraph};
use disk_nix_plan::{
    ApplyPolicy, Plan, SUPPORTED_SPEC_VERSION, TopologyComparison, TopologyDiagnosticLevel,
    compare_plan_with_topology, default_capabilities, plan_and_policy_from_json_bytes,
    plan_from_json_bytes,
};
use disk_nix_probe::{
    LinuxProbe, ProbeAdapter, ProbeAdapterRemediation, ProbeIssueCategory, ProbeStatus,
    adapter_remediation,
};
use serde::Serialize;
use serde_json::Value;

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
        /// Include OS, kernel, and storage tool version preflight context.
        #[arg(long)]
        preflight: bool,
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
    /// List Btrfs filesystems, subvolumes, snapshots, qgroups, and members.
    Btrfs {
        /// Emit JSON for matching graph nodes.
        #[arg(long)]
        json: bool,
    },
    /// List bcachefs filesystems, member devices, and usage accounting.
    Bcachefs {
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
    /// List device-mapper maps and dmsetup table/status metadata.
    Dm {
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
    /// List discovered NVMe subsystems, controllers, namespaces, and path metadata.
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
    /// List discovered file-backed storage origins.
    BackingFiles {
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
    /// List discovered zram compressed swap devices.
    Zram {
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
    /// List discovered host-visible LUNs and SCSI path metadata.
    Luns {
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
        /// Relationship depth to include around matched nodes.
        #[arg(long, default_value_t = 1)]
        depth: usize,
        /// Emit JSON for matched nodes and relationship context.
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
        /// Write a JSON apply receipt with invocation metadata and the report.
        #[arg(long)]
        receipt_out: Option<String>,
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
        /// Write a JSON validation receipt with invocation metadata and the report.
        #[arg(long)]
        receipt_out: Option<String>,
        /// Emit JSON validation report.
        #[arg(long)]
        json: bool,
    },
    /// Normalize a desired storage spec to the current supported contract version.
    Migrate {
        /// Desired storage specification path.
        #[arg(long)]
        spec: String,
        /// Emit JSON migration report with the migrated spec.
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct MigrationReport {
    source_version: Option<u64>,
    target_version: u64,
    migrated: bool,
    version_migrations: Vec<VersionMigrationContract>,
    legacy_mappings: Vec<LegacyMigrationMapping>,
    applied_mappings: Vec<LegacyMigrationMapping>,
    changes: Vec<String>,
    warnings: Vec<String>,
    spec: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct VersionMigrationContract {
    source_version: Option<u64>,
    target_version: u64,
    status: String,
    mapping_scope: String,
    field_mappings: Vec<LegacyMigrationMapping>,
    safety_notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacyMigrationMapping {
    source: String,
    target: String,
    scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProbeStatusPreflightReport {
    environment: ProbePreflightEnvironment,
    preflight_checks: ProbePreflightChecks,
    reports: Vec<disk_nix_probe::ProbeReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProbePreflightEnvironment {
    os_id: Option<String>,
    os_version_id: Option<String>,
    os_pretty_name: Option<String>,
    kernel_release: Option<String>,
    effective_uid: Option<String>,
    tool_versions: Vec<ToolVersionReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProbePreflightChecks {
    status: ProbePreflightCheckStatus,
    root: bool,
    unavailable_tool_count: usize,
    failed_tool_count: usize,
    missing_tools: Vec<String>,
    failed_tools: Vec<String>,
    adapter_remediation: Vec<ProbeAdapterRemediation>,
    remediation: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ProbePreflightCheckStatus {
    Ready,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolVersionReport {
    tool: String,
    status: ToolVersionStatus,
    version: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ToolVersionStatus {
    Available,
    Unavailable,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApplyReceipt {
    receipt_version: u64,
    command: String,
    spec_path: String,
    probe_current: bool,
    execute_requested: bool,
    generated_at_unix_seconds: u64,
    report: ExecutionReport,
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
        Command::ProbeStatus { json, preflight } => {
            let probe = LinuxProbe::new();
            let result = probe
                .collect()
                .map_err(|error| AppError::Message(error.to_string()))?;
            if json {
                if preflight {
                    let environment = collect_probe_preflight_environment();
                    let preflight_checks = probe_preflight_checks(&environment);
                    let report = ProbeStatusPreflightReport {
                        environment,
                        preflight_checks,
                        reports: result.reports,
                    };
                    writeln!(
                        output,
                        "{}",
                        serde_json::to_string_pretty(&report)
                            .map_err(|error| AppError::Message(error.to_string()))?
                    )?;
                } else {
                    writeln!(
                        output,
                        "{}",
                        serde_json::to_string_pretty(&result.reports)
                            .map_err(|error| AppError::Message(error.to_string()))?
                    )?;
                }
            } else if preflight {
                let environment = collect_probe_preflight_environment();
                let preflight_checks = probe_preflight_checks(&environment);
                print_probe_preflight_environment(output, &environment)?;
                print_probe_preflight_checks(output, &preflight_checks)?;
                print_probe_reports(output, &result.reports)?;
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
        Command::Btrfs { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_btrfs_node)?;
            } else {
                print_btrfs(output, &graph)?;
            }
            Ok(())
        }
        Command::Bcachefs { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_bcachefs_node)?;
            } else {
                print_bcachefs(output, &graph)?;
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
        Command::Dm { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_dm_node)?;
            } else {
                print_dm(output, &graph)?;
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
        Command::BackingFiles { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_backing_file_node)?;
            } else {
                print_backing_files(output, &graph)?;
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
        Command::Zram { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_zram_node)?;
            } else {
                print_zram(output, &graph)?;
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
        Command::Luns { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_lun_node)?;
            } else {
                print_luns(output, &graph)?;
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
        Command::Inspect { query, depth, json } => {
            let graph = collect_graph()?;
            if json {
                print_inspect_json(output, &graph, &query, depth)?;
            } else {
                print_inspect(output, &graph, &query, depth)?;
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
            receipt_out,
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
            if let Some(receipt_out) = receipt_out.as_deref() {
                write_apply_receipt(
                    receipt_out,
                    apply_receipt(
                        "apply",
                        &spec,
                        probe_current,
                        execute,
                        current_unix_seconds()?,
                        &report,
                    ),
                )?;
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
            receipt_out,
            json,
        } => {
            let report = prepare_apply_report(&spec, probe_current, ExecutionMode::DryRun)?;
            if let Some(report_out) = report_out.as_deref() {
                write_execution_report(report_out, &report)?;
            }
            if let Some(receipt_out) = receipt_out.as_deref() {
                write_apply_receipt(
                    receipt_out,
                    apply_receipt(
                        "validate",
                        &spec,
                        probe_current,
                        false,
                        current_unix_seconds()?,
                        &report,
                    ),
                )?;
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
        Command::Migrate { spec, json } => {
            let bytes = std::fs::read(&spec)?;
            let report = migration_report_from_json_bytes(&bytes)
                .map_err(|error| AppError::Message(format!("failed to migrate {spec}: {error}")))?;
            if json {
                writeln!(
                    output,
                    "{}",
                    serde_json::to_string_pretty(&report)
                        .map_err(|error| AppError::Message(error.to_string()))?
                )?;
            } else {
                print_migration_report(output, &report)?;
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
                "const": SUPPORTED_SPEC_VERSION,
                "description": "Optional disk-nix spec contract version. Version 1 is the current supported contract."
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
            "zram": {
                "$ref": "#/$defs/zramSpec"
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
            "backingFiles": {
                "$ref": "#/$defs/lifecycleMap"
            },
            "dmMaps": {
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
            "targetLuns": {
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
                    "version": {
                        "type": "integer",
                        "const": SUPPORTED_SPEC_VERSION,
                        "description": "Optional disk-nix spec contract version. Version 1 is the current supported contract."
                    },
                    "filesystems": { "$ref": "#/$defs/filesystemMap" },
                    "swaps": { "$ref": "#/$defs/lifecycleMap" },
                    "zram": { "$ref": "#/$defs/zramSpec" },
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
                    "backingFiles": { "$ref": "#/$defs/lifecycleMap" },
                    "dmMaps": { "$ref": "#/$defs/lifecycleMap" },
                    "mdRaids": { "$ref": "#/$defs/lifecycleMap" },
                    "multipathMaps": { "$ref": "#/$defs/lifecycleMap" },
                    "pools": { "$ref": "#/$defs/lifecycleMap" },
                    "datasets": { "$ref": "#/$defs/lifecycleMap" },
                    "zvols": { "$ref": "#/$defs/lifecycleMap" },
                    "luns": { "$ref": "#/$defs/lifecycleMap" },
                    "targetLuns": { "$ref": "#/$defs/lifecycleMap" },
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
            "zramSpec": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "enable": { "type": "boolean" },
                    "operation": { "$ref": "#/$defs/operation" },
                    "action": { "$ref": "#/$defs/operation" },
                    "swapDevices": { "type": "integer", "minimum": 1 },
                    "memoryPercent": { "type": "integer", "minimum": 1 },
                    "memoryMax": { "type": ["integer", "null"] },
                    "priority": { "type": "integer" },
                    "algorithm": { "type": "string" },
                    "writebackDevice": { "type": ["string", "null"] },
                    "preserveData": { "type": "boolean", "default": true },
                    "properties": {
                        "type": "object",
                        "additionalProperties": true
                    }
                }
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
                    "paths": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "devicePaths": {
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
                    "cacheSetUuid": { "type": "string" },
                    "cacheSetUUID": { "type": "string" },
                    "cache-set-uuid": { "type": "string" },
                    "cache_set_uuid": { "type": "string" },
                    "properties": {
                        "type": "object",
                        "additionalProperties": true
                    },
                    "desiredSize": { "type": ["string", "number"] },
                    "targetSize": { "type": ["string", "number"] },
                    "size": { "type": ["string", "number"] },
                    "physicalSize": { "type": ["string", "number"] },
                    "vdoPhysicalSize": { "type": ["string", "number"] },
                    "physical-size": { "type": ["string", "number"] },
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
                    "initiators": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "initiatorIqns": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "clients": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "portal": { "type": "string" },
                    "provider": { "type": "string" },
                    "storageProvider": { "type": "string" },
                    "storage-provider": { "type": "string" },
                    "arrayProvider": { "type": "string" },
                    "array-provider": { "type": "string" },
                    "lun": { "type": ["string", "number"] },
                    "lunId": { "type": ["string", "number"] },
                    "lun-id": { "type": ["string", "number"] },
                    "lunNumber": { "type": ["string", "number"] },
                    "lun-number": { "type": ["string", "number"] },
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
                    },
                    "receiptOut": {
                        "type": ["string", "null"],
                        "description": "NixOS module helper that controls activation --receipt-out."
                    }
                }
            }
        }
    })
}

fn migration_report_from_json_bytes(bytes: &[u8]) -> Result<MigrationReport, AppError> {
    let mut value: Value =
        serde_json::from_slice(bytes).map_err(|error| AppError::Message(error.to_string()))?;
    let source_version = migration_source_version(&value)?;
    let target_version = SUPPORTED_SPEC_VERSION;
    if source_version.is_some_and(|version| version != target_version) {
        return Err(AppError::Message(format!(
            "unsupported disk-nix spec version {}; supported migration target is {target_version}",
            source_version.expect("checked")
        )));
    }

    let mut changes = Vec::new();
    let mut warnings = Vec::new();
    let mut applied_mappings = Vec::new();
    apply_legacy_pre_version_mappings(
        &mut value,
        source_version,
        &mut changes,
        &mut applied_mappings,
    )?;
    ensure_object_version(&mut value, "version", target_version, &mut changes)?;
    if let Some(spec) = value.get_mut("spec") {
        ensure_object_version(spec, "spec.version", target_version, &mut changes)?;
    }
    if changes.is_empty() {
        changes.push("spec already declares the current supported contract version".to_string());
    }
    warnings.push(
        "migration does not apply storage mutations; run plan or apply separately after review"
            .to_string(),
    );
    warnings.push(
        "version 1 migration only normalizes metadata and documented legacy pre-versioned field names"
            .to_string(),
    );

    let serialized =
        serde_json::to_vec(&value).map_err(|error| AppError::Message(error.to_string()))?;
    plan_from_json_bytes(&serialized)
        .map_err(|error| AppError::Message(format!("migrated spec is invalid: {error}")))?;

    Ok(MigrationReport {
        source_version,
        target_version,
        migrated: !changes
            .iter()
            .any(|change| change == "spec already declares the current supported contract version"),
        version_migrations: version_migration_contracts(),
        legacy_mappings: legacy_pre_version_mappings(),
        applied_mappings,
        changes,
        warnings,
        spec: value,
    })
}

fn apply_legacy_pre_version_mappings(
    value: &mut Value,
    source_version: Option<u64>,
    changes: &mut Vec<String>,
    applied_mappings: &mut Vec<LegacyMigrationMapping>,
) -> Result<(), AppError> {
    if source_version.is_some() {
        return Ok(());
    }
    normalize_legacy_pre_version_container(value, "", changes, applied_mappings)?;
    if let Some(spec) = value.get_mut("spec") {
        normalize_legacy_pre_version_container(spec, "spec.", changes, applied_mappings)?;
    }
    Ok(())
}

fn normalize_legacy_pre_version_container(
    value: &mut Value,
    prefix: &str,
    changes: &mut Vec<String>,
    applied_mappings: &mut Vec<LegacyMigrationMapping>,
) -> Result<(), AppError> {
    rename_legacy_field(
        value,
        prefix,
        "fileSystems",
        "filesystems",
        changes,
        applied_mappings,
    )?;
    rename_legacy_field(
        value,
        prefix,
        "swapDevices",
        "swaps",
        changes,
        applied_mappings,
    )?;
    move_legacy_nested_field(
        value,
        prefix,
        "luksDevices",
        "luks",
        "devices",
        changes,
        applied_mappings,
    )?;
    move_legacy_nested_field(
        value,
        prefix,
        "nfsMounts",
        "nfs",
        "mounts",
        changes,
        applied_mappings,
    )?;
    move_legacy_nested_field(
        value,
        prefix,
        "iscsiSessions",
        "iscsi",
        "sessions",
        changes,
        applied_mappings,
    )?;
    Ok(())
}

fn rename_legacy_field(
    value: &mut Value,
    prefix: &str,
    legacy: &str,
    current: &str,
    changes: &mut Vec<String>,
    applied_mappings: &mut Vec<LegacyMigrationMapping>,
) -> Result<(), AppError> {
    let Value::Object(object) = value else {
        return Ok(());
    };
    if !object.contains_key(legacy) {
        return Ok(());
    }
    if object.contains_key(current) {
        return Err(AppError::Message(format!(
            "legacy field {prefix}{legacy} conflicts with current field {prefix}{current}"
        )));
    }
    let Some(mapped) = object.remove(legacy) else {
        return Ok(());
    };
    object.insert(current.to_string(), mapped);
    changes.push(format!(
        "mapped legacy field {prefix}{legacy} to {prefix}{current}"
    ));
    applied_mappings.push(legacy_mapping(prefix, legacy, current));
    Ok(())
}

fn move_legacy_nested_field(
    value: &mut Value,
    prefix: &str,
    legacy: &str,
    parent: &str,
    child: &str,
    changes: &mut Vec<String>,
    applied_mappings: &mut Vec<LegacyMigrationMapping>,
) -> Result<(), AppError> {
    let Value::Object(object) = value else {
        return Ok(());
    };
    if !object.contains_key(legacy) {
        return Ok(());
    }

    if object
        .get(parent)
        .and_then(Value::as_object)
        .is_some_and(|parent| parent.contains_key(child))
    {
        return Err(AppError::Message(format!(
            "legacy field {prefix}{legacy} conflicts with current field {prefix}{parent}.{child}"
        )));
    }

    let Some(mapped) = object.remove(legacy) else {
        return Ok(());
    };
    let parent_value = object
        .entry(parent.to_string())
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    let Value::Object(parent_object) = parent_value else {
        return Err(AppError::Message(format!(
            "legacy field {prefix}{legacy} cannot be mapped because {prefix}{parent} is not an object"
        )));
    };
    parent_object.insert(child.to_string(), mapped);
    changes.push(format!(
        "mapped legacy field {prefix}{legacy} to {prefix}{parent}.{child}"
    ));
    applied_mappings.push(legacy_mapping(prefix, legacy, &format!("{parent}.{child}")));
    Ok(())
}

fn legacy_mapping(prefix: &str, source: &str, target: &str) -> LegacyMigrationMapping {
    LegacyMigrationMapping {
        source: format!("{prefix}{source}"),
        target: format!("{prefix}{target}"),
        scope: if prefix.is_empty() {
            "top-level".to_string()
        } else {
            prefix.trim_end_matches('.').to_string()
        },
    }
}

fn legacy_pre_version_mappings() -> Vec<LegacyMigrationMapping> {
    ["", "spec."]
        .into_iter()
        .flat_map(|prefix| {
            [
                ("fileSystems", "filesystems"),
                ("swapDevices", "swaps"),
                ("luksDevices", "luks.devices"),
                ("nfsMounts", "nfs.mounts"),
                ("iscsiSessions", "iscsi.sessions"),
            ]
            .into_iter()
            .map(move |(source, target)| legacy_mapping(prefix, source, target))
        })
        .collect()
}

fn version_migration_contracts() -> Vec<VersionMigrationContract> {
    vec![
        VersionMigrationContract {
            source_version: None,
            target_version: SUPPORTED_SPEC_VERSION,
            status: "supported".to_string(),
            mapping_scope: "pre-version legacy aliases to version 1".to_string(),
            field_mappings: legacy_pre_version_mappings(),
            safety_notes: vec![
                "applies only to unversioned documents".to_string(),
                "does not apply storage mutations".to_string(),
                "conflicting legacy and current fields are rejected".to_string(),
            ],
        },
        VersionMigrationContract {
            source_version: Some(SUPPORTED_SPEC_VERSION),
            target_version: SUPPORTED_SPEC_VERSION,
            status: "supported".to_string(),
            mapping_scope: "version 1 metadata normalization".to_string(),
            field_mappings: Vec::new(),
            safety_notes: vec![
                "explicit version 1 documents are validated without legacy alias rewrites"
                    .to_string(),
                "does not apply storage mutations".to_string(),
            ],
        },
    ]
}

fn migration_source_version(value: &Value) -> Result<Option<u64>, AppError> {
    let top_level = optional_version_field(value, "version")?;
    let spec = value
        .get("spec")
        .map(|spec| optional_version_field(spec, "spec.version"))
        .transpose()?
        .flatten();
    if let (Some(top_level), Some(spec)) = (top_level, spec) {
        if top_level != spec {
            return Err(AppError::Message(format!(
                "conflicting disk-nix spec versions: top-level version {top_level}, spec.version {spec}"
            )));
        }
    }
    Ok(top_level.or(spec))
}

fn optional_version_field(value: &Value, location: &str) -> Result<Option<u64>, AppError> {
    let Some(version) = value.get("version") else {
        return Ok(None);
    };
    version.as_u64().map(Some).ok_or_else(|| {
        AppError::Message(format!(
            "disk-nix spec version at {location} must be an integer"
        ))
    })
}

fn ensure_object_version(
    value: &mut Value,
    location: &str,
    target_version: u64,
    changes: &mut Vec<String>,
) -> Result<(), AppError> {
    let Value::Object(object) = value else {
        return Err(AppError::Message(format!(
            "disk-nix spec at {location} must be an object to add version metadata"
        )));
    };
    match object.get("version").and_then(Value::as_u64) {
        Some(version) if version == target_version => Ok(()),
        Some(version) => Err(AppError::Message(format!(
            "unsupported disk-nix spec version {version}; supported migration target is {target_version}"
        ))),
        None => {
            object.insert("version".to_string(), Value::from(target_version));
            changes.push(format!("set {location} to {target_version}"));
            Ok(())
        }
    }
}

fn print_migration_report(output: &mut impl Write, report: &MigrationReport) -> io::Result<()> {
    writeln!(
        output,
        "Migration: {:?} -> {}",
        report.source_version, report.target_version
    )?;
    writeln!(output, "migrated: {}", report.migrated)?;
    writeln!(output, "Changes:")?;
    for change in &report.changes {
        writeln!(output, "- {change}")?;
    }
    writeln!(output, "Version migration contracts:")?;
    for contract in &report.version_migrations {
        writeln!(
            output,
            "- {:?} -> {}: {} ({})",
            contract.source_version,
            contract.target_version,
            contract.status,
            contract.mapping_scope
        )?;
        if contract.field_mappings.is_empty() {
            writeln!(output, "  field mappings: none")?;
        } else {
            writeln!(output, "  field mappings:")?;
            for mapping in &contract.field_mappings {
                writeln!(
                    output,
                    "  - {} -> {} ({})",
                    mapping.source, mapping.target, mapping.scope
                )?;
            }
        }
    }
    writeln!(output, "Legacy mappings:")?;
    for mapping in &report.legacy_mappings {
        writeln!(
            output,
            "- {} -> {} ({})",
            mapping.source, mapping.target, mapping.scope
        )?;
    }
    writeln!(output, "Applied mappings:")?;
    if report.applied_mappings.is_empty() {
        writeln!(output, "- none")?;
    } else {
        for mapping in &report.applied_mappings {
            writeln!(
                output,
                "- {} -> {} ({})",
                mapping.source, mapping.target, mapping.scope
            )?;
        }
    }
    writeln!(output, "Warnings:")?;
    for warning in &report.warnings {
        writeln!(output, "- {warning}")?;
    }
    writeln!(output, "Migrated spec:")?;
    writeln!(
        output,
        "{}",
        serde_json::to_string_pretty(&report.spec).map_err(io::Error::other)?
    )
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
    let script = report
        .to_shell_script()
        .ok_or_else(|| AppError::Message(script_refusal_message(report)))?;
    std::fs::write(path, script)?;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

fn script_refusal_message(report: &ExecutionReport) -> String {
    let graph_dependency_conflict_count =
        report.topology_comparison.as_ref().map_or(0, |comparison| {
            comparison.summary.graph_dependency_conflict_count
        });
    let mut reasons = Vec::new();
    if !report.apply.can_execute() {
        reasons.push(format!(
            "apply policy blocks {} action(s)",
            report.apply.blocked_count
        ));
    }
    if graph_dependency_conflict_count > 0 {
        reasons.push(format!(
            "{graph_dependency_conflict_count} graph dependency conflict(s) require plan splitting or ordering review"
        ));
    }
    if !report.command_summary.all_commands_ready() {
        reasons.push(format!(
            "{} command(s) need desired size, {} need domain command implementation, {} are manual-only",
            report.command_summary.needs_desired_size_count,
            report.command_summary.needs_domain_implementation_count,
            report.command_summary.manual_only_count
        ));
    }
    if reasons.is_empty() {
        reasons.push("report is not in a scriptable dry-run state".to_string());
    }
    format!(
        "script generation requires a policy-allowed, conflict-free command plan: {}",
        reasons.join("; ")
    )
}

fn write_execution_report(path: &str, report: &ExecutionReport) -> Result<(), AppError> {
    let mut report_json = report
        .to_json()
        .map_err(|error| AppError::Message(error.to_string()))?;
    report_json.push('\n');
    std::fs::write(path, report_json)?;
    Ok(())
}

fn apply_receipt(
    command: &str,
    spec_path: &str,
    probe_current: bool,
    execute_requested: bool,
    generated_at_unix_seconds: u64,
    report: &ExecutionReport,
) -> ApplyReceipt {
    ApplyReceipt {
        receipt_version: 1,
        command: command.to_string(),
        spec_path: spec_path.to_string(),
        probe_current,
        execute_requested,
        generated_at_unix_seconds,
        report: report.clone(),
    }
}

fn write_apply_receipt(path: &str, receipt: ApplyReceipt) -> Result<(), AppError> {
    let mut receipt_json = serde_json::to_string_pretty(&receipt)
        .map_err(|error| AppError::Message(error.to_string()))?;
    receipt_json.push('\n');
    std::fs::write(path, receipt_json)?;
    Ok(())
}

fn current_unix_seconds() -> Result<u64, AppError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| {
            AppError::Message(format!("system clock is before the Unix epoch: {error}"))
        })
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
    let matched_ids: BTreeSet<String> = graph
        .nodes
        .iter()
        .filter(|node| predicate(node))
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
    let filtered = StorageGraph { nodes, edges };

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
    depth: usize,
) -> Result<(), AppError> {
    let matched_ids: BTreeSet<String> = graph
        .find_nodes(query)
        .into_iter()
        .map(|node| node.id.0.clone())
        .collect();

    let subgraph = relationship_subgraph(graph, &matched_ids, depth);
    writeln!(
        output,
        "{}",
        subgraph
            .to_json()
            .map_err(|error| AppError::Message(error.to_string()))?
    )?;
    Ok(())
}

fn relationship_subgraph(
    graph: &StorageGraph,
    initial_ids: &BTreeSet<String>,
    depth: usize,
) -> StorageGraph {
    let mut node_ids = initial_ids.clone();
    let mut edge_indexes = BTreeSet::new();
    let mut queue = initial_ids
        .iter()
        .map(|id| (id.clone(), 0_usize))
        .collect::<VecDeque<_>>();

    while let Some((node_id, distance)) = queue.pop_front() {
        if distance >= depth {
            continue;
        }

        for (index, edge) in graph.edges.iter().enumerate() {
            let neighbor = if edge.from.0 == node_id {
                Some(edge.to.0.as_str())
            } else if edge.to.0 == node_id {
                Some(edge.from.0.as_str())
            } else {
                None
            };

            let Some(neighbor) = neighbor else {
                continue;
            };

            edge_indexes.insert(index);
            if node_ids.insert(neighbor.to_string()) {
                queue.push_back((neighbor.to_string(), distance + 1));
            }
        }
    }

    let nodes = graph
        .nodes
        .iter()
        .filter(|node| node_ids.contains(node.id.0.as_str()))
        .cloned()
        .collect();
    let edges = graph
        .edges
        .iter()
        .enumerate()
        .filter(|(index, _)| edge_indexes.contains(index))
        .map(|(_, edge)| edge.clone())
        .collect();

    StorageGraph { nodes, edges }
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

fn collect_probe_preflight_environment() -> ProbePreflightEnvironment {
    let os_release = fs::read_to_string("/etc/os-release")
        .ok()
        .map(|contents| parse_os_release(&contents))
        .unwrap_or_default();
    ProbePreflightEnvironment {
        os_id: os_release
            .iter()
            .find(|(key, _)| key == "ID")
            .map(|(_, value)| value.clone()),
        os_version_id: os_release
            .iter()
            .find(|(key, _)| key == "VERSION_ID")
            .map(|(_, value)| value.clone()),
        os_pretty_name: os_release
            .iter()
            .find(|(key, _)| key == "PRETTY_NAME")
            .map(|(_, value)| value.clone()),
        kernel_release: command_stdout_first_line("uname", &["-r"]).ok(),
        effective_uid: command_stdout_first_line("id", &["-u"]).ok(),
        tool_versions: storage_tool_version_reports(),
    }
}

fn parse_os_release(contents: &str) -> Vec<(String, String)> {
    contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            Some((key.to_string(), unquote_os_release_value(value)))
        })
        .collect()
}

fn unquote_os_release_value(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        value[1..value.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    } else {
        value.to_string()
    }
}

fn storage_tool_version_reports() -> Vec<ToolVersionReport> {
    [
        ("lsblk", &["--version"][..]),
        ("blkid", &["--version"][..]),
        ("findmnt", &["--version"][..]),
        ("parted", &["--version"][..]),
        ("smartctl", &["--version"][..]),
        ("cryptsetup", &["--version"][..]),
        ("dmsetup", &["version"][..]),
        ("lvm", &["version"][..]),
        ("vdo", &["--version"][..]),
        ("zpool", &["--version"][..]),
        ("zfs", &["--version"][..]),
        ("btrfs", &["--version"][..]),
        ("bcachefs", &["version"][..]),
        ("lsscsi", &["--version"][..]),
        ("iscsiadm", &["--version"][..]),
        ("exportfs", &["--version"][..]),
        ("nfsstat", &["--version"][..]),
        ("mdadm", &["--version"][..]),
        ("multipath", &["-h"][..]),
        ("nvme", &["version"][..]),
    ]
    .into_iter()
    .map(|(tool, args)| storage_tool_version_report(tool, args))
    .collect()
}

fn storage_tool_version_report(tool: &str, args: &[&str]) -> ToolVersionReport {
    match command_stdout_first_line(tool, args) {
        Ok(version) => ToolVersionReport {
            tool: tool.to_string(),
            status: ToolVersionStatus::Available,
            version: Some(version),
            message: None,
        },
        Err(message) if message.contains("not found") || message.contains("No such file") => {
            ToolVersionReport {
                tool: tool.to_string(),
                status: ToolVersionStatus::Unavailable,
                version: None,
                message: Some(message),
            }
        }
        Err(message) => ToolVersionReport {
            tool: tool.to_string(),
            status: ToolVersionStatus::Failed,
            version: None,
            message: Some(message),
        },
    }
}

fn probe_preflight_checks(environment: &ProbePreflightEnvironment) -> ProbePreflightChecks {
    let root = environment.effective_uid.as_deref() == Some("0");
    let missing_tools = environment
        .tool_versions
        .iter()
        .filter(|tool| tool.status == ToolVersionStatus::Unavailable)
        .map(|tool| tool.tool.clone())
        .collect::<Vec<_>>();
    let failed_tools = environment
        .tool_versions
        .iter()
        .filter(|tool| tool.status == ToolVersionStatus::Failed)
        .map(|tool| tool.tool.clone())
        .collect::<Vec<_>>();
    let mut remediation = Vec::new();
    if !root {
        remediation.push(
            "run probe-status with privileges when adapter metadata requires root-only kernel or device access"
                .to_string(),
        );
    }
    if !missing_tools.is_empty() {
        remediation.push(format!(
            "install or expose missing storage tool(s): {}",
            missing_tools.join(", ")
        ));
        remediation.push(
            "on NixOS, add the required storage packages to environment.systemPackages or services.disk-nix.toolPackages"
                .to_string(),
        );
    }
    if !failed_tools.is_empty() {
        remediation.push(format!(
            "rerun failed storage tool version probe(s) manually with stderr captured: {}",
            failed_tools.join(", ")
        ));
    }
    let status = if root && missing_tools.is_empty() && failed_tools.is_empty() {
        ProbePreflightCheckStatus::Ready
    } else {
        ProbePreflightCheckStatus::Degraded
    };

    ProbePreflightChecks {
        status,
        root,
        unavailable_tool_count: missing_tools.len(),
        failed_tool_count: failed_tools.len(),
        missing_tools,
        failed_tools,
        adapter_remediation: preflight_adapter_remediation(),
        remediation,
    }
}

fn preflight_adapter_remediation() -> Vec<ProbeAdapterRemediation> {
    [
        "lsblk",
        "blkid",
        "findmnt",
        "udev",
        "parted",
        "smartctl",
        "ext",
        "xfs",
        "btrfs",
        "bcachefs",
        "bcache",
        "cryptsetup",
        "dmsetup",
        "lvm",
        "vdo",
        "vdostats",
        "vdostats-verbose",
        "mdraid",
        "mdadm-scan",
        "mdadm-examine",
        "multipath",
        "nfs",
        "nfs-exports",
        "iscsi",
        "iscsi-nodes",
        "lsscsi",
        "nvme",
        "nvme-list-subsys",
        "nvme-smart-log",
        "nvme-id-ctrl",
        "nvme-id-ns",
        "loop",
        "swaps",
        "zramctl",
        "zfs",
    ]
    .into_iter()
    .map(adapter_remediation)
    .collect()
}

fn command_stdout_first_line(command: &str, args: &[&str]) -> Result<String, String> {
    match ProcessCommand::new(command).args(args).output() {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let line = stdout
                .lines()
                .chain(stderr.lines())
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or("");
            if line.is_empty() {
                Err(format!("{command} {:?} returned no version text", args))
            } else {
                Ok(line.to_string())
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let detail = stderr
                .lines()
                .chain(stdout.lines())
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or("command returned a non-zero status");
            Err(format!(
                "{command} {:?} failed with status {}: {detail}",
                args, output.status
            ))
        }
        Err(error) => Err(format!("{command} not found or failed to run: {error}")),
    }
}

fn print_probe_preflight_environment(
    output: &mut impl Write,
    environment: &ProbePreflightEnvironment,
) -> io::Result<()> {
    writeln!(output, "Preflight environment:")?;
    writeln!(
        output,
        "  os: {}",
        environment.os_pretty_name.as_deref().unwrap_or("-")
    )?;
    writeln!(
        output,
        "  os-id: {}",
        environment.os_id.as_deref().unwrap_or("-")
    )?;
    writeln!(
        output,
        "  os-version-id: {}",
        environment.os_version_id.as_deref().unwrap_or("-")
    )?;
    writeln!(
        output,
        "  kernel: {}",
        environment.kernel_release.as_deref().unwrap_or("-")
    )?;
    writeln!(
        output,
        "  effective-uid: {}",
        environment.effective_uid.as_deref().unwrap_or("-")
    )?;
    writeln!(output, "  storage tools:")?;
    for tool in &environment.tool_versions {
        let status = match tool.status {
            ToolVersionStatus::Available => "available",
            ToolVersionStatus::Unavailable => "unavailable",
            ToolVersionStatus::Failed => "failed",
        };
        let detail = tool
            .version
            .as_deref()
            .or(tool.message.as_deref())
            .unwrap_or("-");
        writeln!(output, "    {:<12} {:<12} {}", tool.tool, status, detail)?;
    }
    writeln!(output)?;
    Ok(())
}

fn print_probe_preflight_checks(
    output: &mut impl Write,
    checks: &ProbePreflightChecks,
) -> io::Result<()> {
    let status = match checks.status {
        ProbePreflightCheckStatus::Ready => "ready",
        ProbePreflightCheckStatus::Degraded => "degraded",
    };
    writeln!(output, "Preflight checks:")?;
    writeln!(output, "  status: {status}")?;
    writeln!(output, "  root: {}", checks.root)?;
    writeln!(
        output,
        "  unavailable-tools: {}",
        checks.unavailable_tool_count
    )?;
    writeln!(output, "  failed-tools: {}", checks.failed_tool_count)?;
    if !checks.missing_tools.is_empty() {
        writeln!(
            output,
            "  missing-tools: {}",
            checks.missing_tools.join(", ")
        )?;
    }
    if !checks.failed_tools.is_empty() {
        writeln!(
            output,
            "  failed-tool-names: {}",
            checks.failed_tools.join(", ")
        )?;
    }
    for remediation in &checks.remediation {
        writeln!(output, "    remediation: {remediation}")?;
    }
    writeln!(output)?;
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
        let category = match report.category() {
            ProbeIssueCategory::None => "none",
            ProbeIssueCategory::MissingTool => "missing-tool",
            ProbeIssueCategory::PermissionDenied => "permission-denied",
            ProbeIssueCategory::CommandFailed => "command-failed",
            ProbeIssueCategory::ParseFailed => "parse-failed",
            ProbeIssueCategory::InaccessibleData => "inaccessible-data",
        };

        if let Some(message) = &report.message {
            writeln!(
                output,
                "  {:<12} {:<12} {:<18} {}",
                report.adapter, status, category, message
            )?;
        } else {
            writeln!(
                output,
                "  {:<12} {:<12} {}",
                report.adapter, status, category
            )?;
        }
        for remediation in report.remediation() {
            writeln!(output, "    remediation: {remediation}")?;
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

fn print_btrfs(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:>12} {:>12} {:>7} {:<18} {:>7} DETAILS",
        "KIND", "NAME", "SIZE", "USED", "FREE", "USE%", "MOUNT", "BACKING"
    )?;
    for node in graph.nodes.iter().filter(|node| is_btrfs_node(node)) {
        let usage = node.usage.as_ref();
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:>12} {:>12} {:>7} {:<18} {:>7} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            human_bytes(usage.and_then(|usage| usage.used_bytes)),
            human_bytes(usage.and_then(|usage| usage.free_bytes)),
            usage_percent(node),
            property_value(node, "btrfs.mount-target")
                .or_else(|| property_value(node, "mountpoint"))
                .unwrap_or("-"),
            backing_count(graph, node),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_bcachefs(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>12} {:>12} {:>12} {:>7} {:<18} {:>7} DETAILS",
        "KIND", "NAME", "SIZE", "USED", "FREE", "USE%", "MOUNT", "MEMBERS"
    )?;
    for node in graph.nodes.iter().filter(|node| is_bcachefs_node(node)) {
        let usage = node.usage.as_ref();
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:>12} {:>12} {:>7} {:<18} {:>7} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            human_bytes(usage.and_then(|usage| usage.used_bytes)),
            human_bytes(usage.and_then(|usage| usage.free_bytes)),
            usage_percent(node),
            property_value(node, "bcachefs.mount-target").unwrap_or("-"),
            member_count(graph, node),
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

fn print_dm(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<38} {:>8} {:<16} {:<16} {:<11} DETAILS",
        "KIND", "NAME", "BACKING", "TARGETS", "STATUS", "MAJOR:MINOR"
    )?;
    for node in graph.nodes.iter().filter(|node| is_dm_node(node)) {
        let major_minor = property_value(node, "dm.major")
            .zip(property_value(node, "dm.minor"))
            .map(|(major, minor)| format!("{major}:{minor}"))
            .unwrap_or_else(|| "-".to_string());
        writeln!(
            output,
            "{:<22} {:<38} {:>8} {:<16} {:<16} {:<11} {}",
            node.kind,
            node.name,
            backing_count(graph, node),
            property_value(node, "dm.table.targets").unwrap_or("-"),
            property_value(node, "dm.status.targets").unwrap_or("-"),
            major_minor,
            usage_details(node)
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
        "{:<22} {:<38} {:>12} {:>12} {:>12} {:>12} {:>7} {:<12} {:<12} DETAILS",
        "KIND", "NAME", "LOGICAL", "PHYSICAL", "USED", "FREE", "USE%", "MODE", "WRITE"
    )?;
    for node in graph.nodes.iter().filter(|node| is_vdo_node(node)) {
        let usage = node.usage.as_ref();
        writeln!(
            output,
            "{:<22} {:<38} {:>12} {:>12} {:>12} {:>12} {:>7} {:<12} {:<12} {}",
            node.kind,
            node.name,
            vdo_logical_display(node),
            vdo_physical_display(node),
            human_bytes(usage.and_then(|usage| usage.used_bytes)),
            human_bytes(usage.and_then(|usage| usage.free_bytes)),
            usage_percent(node),
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

fn print_backing_files(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<44} {:>12} {:>9} {:>7} DETAILS",
        "KIND", "PATH", "SIZE", "CONSUMERS", "USE%"
    )?;
    for node in graph.nodes.iter().filter(|node| is_backing_file_node(node)) {
        writeln!(
            output,
            "{:<22} {:<44} {:>12} {:>9} {:>7} {}",
            node.kind,
            node.path.as_deref().unwrap_or(&node.name),
            human_bytes(node.size_bytes),
            consumer_count(graph, node),
            usage_percent(node),
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

fn print_zram(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<32} {:>12} {:>12} {:>12} {:>12} {:<10} {:<8} {:>12} {:<12} DETAILS",
        "KIND", "NAME", "SIZE", "USED", "FREE", "ALLOC", "ALGO", "RATIO", "MEM-PEAK", "MOUNT"
    )?;
    for node in graph.nodes.iter().filter(|node| is_zram_node(node)) {
        let usage = node.usage.as_ref();
        writeln!(
            output,
            "{:<22} {:<32} {:>12} {:>12} {:>12} {:>12} {:<10} {:<8} {:>12} {:<12} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            human_bytes(usage.and_then(|usage| usage.used_bytes)),
            human_bytes(usage.and_then(|usage| usage.free_bytes)),
            human_bytes(usage.and_then(|usage| usage.allocated_bytes)),
            property_value(node, "zram.algorithm").unwrap_or("-"),
            property_value(node, "zram.compression-ratio").unwrap_or("-"),
            property_value(node, "zram.memory-peak")
                .or_else(|| property_value(node, "zram.memory-used"))
                .unwrap_or("-"),
            property_value(node, "zram.mountpoint").unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_iscsi(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<48} {:>12} {:<22} {:<14} {:>5} {:<18} DETAILS",
        "KIND", "NAME", "SIZE", "PORTAL", "STATE", "LUNS", "PATH"
    )?;
    for node in graph.nodes.iter().filter(|node| is_iscsi_node(node)) {
        writeln!(
            output,
            "{:<22} {:<48} {:>12} {:<22} {:<14} {:>5} {:<18} {}",
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
            node.path.as_deref().unwrap_or("-"),
            usage_details(node)
        )?;
    }
    Ok(())
}

fn print_luns(output: &mut impl Write, graph: &StorageGraph) -> io::Result<()> {
    writeln!(
        output,
        "{:<22} {:<40} {:>12} {:<18} {:<10} {:<18} DETAILS",
        "KIND", "NAME", "SIZE", "PATH", "TRANSPORT", "GENERIC"
    )?;
    for node in graph.nodes.iter().filter(|node| is_lun_node(node)) {
        writeln!(
            output,
            "{:<22} {:<40} {:>12} {:<18} {:<10} {:<18} {}",
            node.kind,
            node.name,
            human_bytes(node.size_bytes),
            node.path
                .as_deref()
                .or_else(|| property_value(node, "scsi.block-device"))
                .or_else(|| property_value(node, "iscsi.attached-disk"))
                .unwrap_or("-"),
            property_value(node, "scsi.transport").unwrap_or("-"),
            property_value(node, "scsi.generic-device").unwrap_or("-"),
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

fn print_inspect(
    output: &mut impl Write,
    graph: &StorageGraph,
    query: &str,
    depth: usize,
) -> io::Result<()> {
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
            if usage.used_bytes.is_some()
                || usage.free_bytes.is_some()
                || usage.allocated_bytes.is_some()
            {
                writeln!(
                    output,
                    "  usage: used={} free={} allocated={} use={}",
                    human_bytes(usage.used_bytes),
                    human_bytes(usage.free_bytes),
                    human_bytes(usage.allocated_bytes),
                    usage_percent(node)
                )?;
            }
        }

        print_identity(output, node)?;
        print_properties(output, node)?;
        print_relationships(output, graph, node, depth)?;
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
    depth: usize,
) -> io::Result<()> {
    let mut initial_ids = BTreeSet::new();
    initial_ids.insert(node.id.0.clone());
    let subgraph = relationship_subgraph(graph, &initial_ids, depth);
    let edges = subgraph.edges.iter().collect::<Vec<_>>();
    if edges.is_empty() {
        return Ok(());
    }

    writeln!(output, "  relationships:")?;
    for edge in edges {
        if depth <= 1 {
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
        } else {
            let from_name = graph
                .nodes
                .iter()
                .find(|candidate| candidate.id == edge.from)
                .map(|candidate| candidate.name.as_str())
                .unwrap_or(edge.from.0.as_str());
            let to_name = graph
                .nodes
                .iter()
                .find(|candidate| candidate.id == edge.to)
                .map(|candidate| candidate.name.as_str())
                .unwrap_or(edge.to.0.as_str());

            writeln!(
                output,
                "    {} ({}) {} {} ({})",
                edge.from.0, from_name, edge.relationship, edge.to.0, to_name
            )?;
        }
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
        "Topology comparison: {} actions, {} matched, {} missing, {} size notes, {} type conflicts, {} already satisfied, {} suppressed, {} graph dependency conflicts",
        comparison.summary.action_count,
        comparison.summary.matched_count,
        comparison.summary.missing_count,
        comparison.summary.size_diagnostic_count,
        comparison.summary.type_conflict_count,
        comparison.summary.already_satisfied_count,
        comparison.summary.suppressed_action_count,
        comparison.summary.graph_dependency_conflict_count
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
            if !report.tool_requirements.is_empty() {
                writeln!(output, "Tool requirements:")?;
                for requirement in &report.tool_requirements {
                    writeln!(
                        output,
                        "- {}: {} commands, {} mutating, {} verification, phases {:?}, availability {:?}",
                        requirement.tool,
                        requirement.command_count,
                        requirement.mutating_count,
                        requirement.verification_count,
                        requirement.phases,
                        requirement.availability
                    )?;
                    writeln!(output, "  {}", requirement.message)?;
                    for remediation in &requirement.remediation {
                        writeln!(output, "  - {remediation}")?;
                    }
                }
            }
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

    if !report.recovery_actions.is_empty() {
        writeln!(output, "Recovery actions:")?;
        for action in &report.recovery_actions {
            writeln!(output, "- {:?}: {}", action.kind, action.summary)?;
            for command in &action.commands {
                let mutation = if command.mutates {
                    "mutating"
                } else {
                    "read-only"
                };
                writeln!(output, "  {mutation}: {}", command.argv.join(" "))?;
                writeln!(output, "    readiness: {:?}", command.readiness)?;
                writeln!(output, "    {}", command.note)?;
            }
            for note in &action.notes {
                writeln!(output, "  note: {note}")?;
            }
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
            | NodeKind::NvmeSubsystem
            | NodeKind::NvmeNamespace
            | NodeKind::LoopDevice
            | NodeKind::BcachefsDevice
            | NodeKind::BackingFile
            | NodeKind::ZramDevice
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

fn is_btrfs_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::BtrfsFilesystem
            | NodeKind::BtrfsSubvolume
            | NodeKind::BtrfsSnapshot
            | NodeKind::BtrfsQgroup
    ) || node
        .properties
        .iter()
        .any(|property| property.key.starts_with("btrfs."))
}

fn is_bcachefs_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::BcachefsFilesystem | NodeKind::BcachefsDevice
    ) || node
        .properties
        .iter()
        .any(|property| property.key.starts_with("bcachefs."))
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

fn is_dm_node(node: &Node) -> bool {
    node.kind == NodeKind::DeviceMapper
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("dm."))
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
    matches!(
        node.kind,
        NodeKind::NvmeSubsystem | NodeKind::NvmeController | NodeKind::NvmeNamespace
    ) || node
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

fn is_backing_file_node(node: &Node) -> bool {
    node.kind == NodeKind::BackingFile
}

fn is_swap_node(node: &Node) -> bool {
    node.kind == NodeKind::Swap
        || node.kind == NodeKind::ZramDevice
        || property_value(node, "zram.swap") == Some("true")
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("swap."))
}

fn is_zram_node(node: &Node) -> bool {
    node.kind == NodeKind::ZramDevice
        || node
            .properties
            .iter()
            .any(|property| property.key.starts_with("zram."))
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

fn is_lun_node(node: &Node) -> bool {
    node.kind == NodeKind::Lun
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
        ("md.uuid", "md-uuid"),
        ("scsi.address", "scsi-address"),
        ("scsi.host", "scsi-host"),
        ("scsi.channel", "scsi-channel"),
        ("scsi.target", "scsi-target"),
        ("scsi.lun", "scsi-lun"),
        ("scsi.peripheral-type", "scsi-type"),
        ("scsi.vendor", "scsi-vendor"),
        ("scsi.model", "scsi-model"),
        ("scsi.revision", "scsi-revision"),
        ("scsi.block-device", "scsi-block"),
        ("scsi.generic-device", "scsi-generic"),
        ("scsi.size", "scsi-size"),
        ("scsi.transport", "scsi-transport"),
        ("scsi.unit-name", "scsi-unit"),
        ("scsi.by-id", "scsi-by-id"),
        ("scsi.wwn", "scsi-wwn"),
        ("scsi.state", "scsi-state"),
        ("scsi.queue-depth", "scsi-queue-depth"),
        ("scsi.queue-type", "scsi-queue-type"),
        ("scsi.scsi-level", "scsi-level"),
        ("scsi.timeout", "scsi-timeout"),
        ("smartctl.svn-revision", "smart-svn"),
        ("smartctl.platform", "smart-platform"),
        ("smartctl.exit-status", "smart-exit-status"),
        ("smartctl.device-name", "smart-device-name"),
        ("smartctl.health.passed", "smart-health-passed"),
        ("smartctl.device-type", "smart-device-type"),
        ("smartctl.protocol", "smart-protocol"),
        ("smartctl.model", "smart-model"),
        ("smartctl.model-family", "smart-family"),
        ("smartctl.vendor", "smart-vendor"),
        ("smartctl.product", "smart-product"),
        ("smartctl.revision", "smart-revision"),
        ("smartctl.firmware-version", "smart-firmware"),
        ("smartctl.serial", "smart-serial"),
        ("smartctl.wwn-naa", "smart-wwn-naa"),
        ("smartctl.wwn-oui", "smart-wwn-oui"),
        ("smartctl.wwn-id", "smart-wwn-id"),
        ("smartctl.user-capacity-bytes", "smart-capacity"),
        ("smartctl.logical-block-size", "smart-logical-block"),
        ("smartctl.physical-block-size", "smart-physical-block"),
        ("smartctl.rotation-rate-rpm", "smart-rpm"),
        ("smartctl.form-factor", "smart-form-factor"),
        ("smartctl.sata-version", "sata-version"),
        (
            "smartctl.interface-speed-current",
            "interface-speed-current",
        ),
        ("smartctl.interface-speed-max", "interface-speed-max"),
        ("smartctl.power-on-hours", "smart-power-on-hours"),
        ("smartctl.power-cycle-count", "smart-power-cycles"),
        (
            "smartctl.temperature-current-celsius",
            "smart-temperature-c",
        ),
        (
            "smartctl.temperature-highest-celsius",
            "smart-temperature-highest-c",
        ),
        (
            "smartctl.temperature-lowest-celsius",
            "smart-temperature-lowest-c",
        ),
        (
            "smartctl.offline-data-collection-status",
            "smart-offline-status",
        ),
        ("smartctl.self-test-status", "smart-self-test"),
        ("smartctl.error-log-summary-count", "smart-error-log-count"),
        ("smartctl.self-test-log-count", "smart-self-test-count"),
        ("smartctl.error-logging-supported", "smart-error-logging"),
        ("smartctl.gp-logging-supported", "smart-gp-logging"),
        ("smartctl.sct-capabilities", "smart-sct-capabilities"),
        (
            "smartctl.scsi-grown-defect-list",
            "smart-scsi-grown-defects",
        ),
        (
            "smartctl.attribute.reallocated-sector-ct.raw",
            "reallocated-sectors",
        ),
        (
            "smartctl.attribute.reallocated-sector-ct.value",
            "reallocated-value",
        ),
        (
            "smartctl.attribute.reallocated-sector-ct.worst",
            "reallocated-worst",
        ),
        (
            "smartctl.attribute.reallocated-sector-ct.threshold",
            "reallocated-threshold",
        ),
        (
            "smartctl.attribute.reallocated-sector-ct.when-failed",
            "reallocated-failed",
        ),
        (
            "smartctl.attribute.current-pending-sector.raw",
            "pending-sectors",
        ),
        (
            "smartctl.attribute.current-pending-sector.value",
            "pending-value",
        ),
        (
            "smartctl.attribute.current-pending-sector.worst",
            "pending-worst",
        ),
        (
            "smartctl.attribute.current-pending-sector.threshold",
            "pending-threshold",
        ),
        (
            "smartctl.attribute.current-pending-sector.when-failed",
            "pending-failed",
        ),
        (
            "smartctl.attribute.offline-uncorrectable.raw",
            "offline-uncorrectable",
        ),
        (
            "smartctl.attribute.offline-uncorrectable.value",
            "offline-uncorrectable-value",
        ),
        (
            "smartctl.attribute.offline-uncorrectable.worst",
            "offline-uncorrectable-worst",
        ),
        (
            "smartctl.attribute.offline-uncorrectable.threshold",
            "offline-uncorrectable-threshold",
        ),
        (
            "smartctl.attribute.offline-uncorrectable.when-failed",
            "offline-uncorrectable-failed",
        ),
        ("nvme.generic-path", "generic"),
        ("nvme.model", "nvme-model"),
        ("nvme.product", "product"),
        ("nvme.firmware", "firmware"),
        ("nvme.index", "ns-index"),
        ("nvme.namespace", "namespace"),
        ("nvme.namespace-id", "nsid"),
        ("nvme.namespace-uuid", "ns-uuid"),
        ("nvme.eui64", "eui64"),
        ("nvme.nguid", "nguid"),
        ("nvme.subsystem", "subsystem"),
        ("nvme.subsystem-name", "subsystem-name"),
        ("nvme.subsystem-nqn", "subsystem-nqn"),
        ("nvme.hostnqn", "hostnqn"),
        ("nvme.controller", "controller"),
        ("nvme.address", "address"),
        ("nvme.transport", "transport"),
        ("nvme.traddr", "traddr"),
        ("nvme.trsvcid", "trsvcid"),
        ("nvme.host-traddr", "host-traddr"),
        ("nvme.host-iface", "host-iface"),
        ("nvme.path-state", "path-state"),
        ("nvme.controller-id", "controller-id"),
        ("nvme.namespace-capacity", "namespace-capacity"),
        ("nvme.lba-format", "lba-format"),
        ("nvme.maximum-lba", "max-lba"),
        ("nvme.sector-size", "sector-size"),
        ("nvme.ana-state", "ana-state"),
        ("nvme.formatted-lba-index", "flba-index"),
        ("nvme.formatted-lba-data-size", "flba-data"),
        ("nvme.formatted-lba-metadata-size", "flba-metadata"),
        (
            "nvme.formatted-lba-relative-performance",
            "flba-relative-performance",
        ),
        ("nvme.id-ns.nsze", "nsze"),
        ("nvme.id-ns.ncap", "ncap"),
        ("nvme.id-ns.nuse", "nuse"),
        ("nvme.id-ns.nsfeat", "nsfeat"),
        ("nvme.id-ns.nlbaf", "nlbaf"),
        ("nvme.id-ns.flbas", "flbas"),
        ("nvme.id-ns.mc", "mc"),
        ("nvme.id-ns.dpc", "dpc"),
        ("nvme.id-ns.dps", "dps"),
        ("nvme.id-ns.nmic", "nmic"),
        ("nvme.id-ns.rescap", "rescap"),
        ("nvme.id-ns.fpi", "fpi"),
        ("nvme.id-ns.dlfeat", "dlfeat"),
        ("nvme.id-ns.nawun", "nawun"),
        ("nvme.id-ns.nawupf", "nawupf"),
        ("nvme.id-ns.nacwu", "nacwu"),
        ("nvme.id-ns.nabsn", "nabsn"),
        ("nvme.id-ns.nabo", "nabo"),
        ("nvme.id-ns.nabspf", "nabspf"),
        ("nvme.id-ns.noiob", "noiob"),
        ("nvme.id-ns.nvmcap", "nvmcap"),
        ("nvme.id-ctrl.vid", "vid"),
        ("nvme.id-ctrl.ssvid", "ssvid"),
        ("nvme.id-ctrl.rab", "rab"),
        ("nvme.id-ctrl.ieee", "ieee"),
        ("nvme.id-ctrl.cmic", "cmic"),
        ("nvme.id-ctrl.mdts", "mdts"),
        ("nvme.id-ctrl.version", "version"),
        ("nvme.id-ctrl.controller-type", "controller-type"),
        ("nvme.id-ctrl.oacs", "optional-admin-commands"),
        ("nvme.id-ctrl.fuses", "fused-operations"),
        ("nvme.id-ctrl.fna", "format-nvm-attributes"),
        ("nvme.id-ctrl.awun", "atomic-write-unit-normal"),
        ("nvme.id-ctrl.awupf", "atomic-write-unit-powerfail"),
        ("nvme.id-ctrl.acwu", "atomic-compare-write-unit"),
        ("nvme.id-ctrl.sgls", "sgl-support"),
        ("nvme.id-ctrl.namespace-set-id-max", "namespace-set-id-max"),
        (
            "nvme.id-ctrl.endurance-group-id-max",
            "endurance-group-id-max",
        ),
        ("nvme.id-ctrl.ana-transition-time", "ana-transition-time"),
        ("nvme.id-ctrl.ana-group-max", "ana-group-max"),
        (
            "nvme.id-ctrl.persistent-event-log-size",
            "persistent-event-log-size",
        ),
        ("nvme.id-ctrl.domain-id", "domain-id"),
        (
            "nvme.id-ctrl.warning-composite-temp",
            "warning-composite-temp",
        ),
        (
            "nvme.id-ctrl.critical-composite-temp",
            "critical-composite-temp",
        ),
        (
            "nvme.id-ctrl.minimum-thermal-management-temp",
            "min-thermal-management-temp",
        ),
        (
            "nvme.id-ctrl.maximum-thermal-management-temp",
            "max-thermal-management-temp",
        ),
        ("nvme.id-ctrl.total-nvm-capacity", "total-nvm-capacity"),
        (
            "nvme.id-ctrl.unallocated-nvm-capacity",
            "unallocated-nvm-capacity",
        ),
        ("nvme.id-ctrl.namespace-count", "namespace-count"),
        ("nvme.id-ctrl.oncs", "oncs"),
        ("nvme.id-ctrl.volatile-write-cache", "volatile-write-cache"),
        (
            "nvme.id-ctrl.sanitize-capabilities",
            "sanitize-capabilities",
        ),
        ("nvme.id-ctrl.ana-capabilities", "ana-capabilities"),
        ("nvme.smart.critical-warning", "critical-warning"),
        ("nvme.smart.temperature-kelvin", "temperature-k"),
        (
            "nvme.smart.available-spare-percent",
            "available-spare-percent",
        ),
        (
            "nvme.smart.spare-threshold-percent",
            "spare-threshold-percent",
        ),
        ("nvme.smart.percent-used", "percent-used"),
        ("nvme.smart.data-units-read", "data-units-read"),
        ("nvme.smart.data-units-written", "data-units-written"),
        ("nvme.smart.host-read-commands", "host-read-commands"),
        ("nvme.smart.host-write-commands", "host-write-commands"),
        ("nvme.smart.controller-busy-time", "controller-busy-time"),
        ("nvme.smart.power-cycles", "power-cycles"),
        ("nvme.smart.power-on-hours", "power-on-hours"),
        ("nvme.smart.unsafe-shutdowns", "unsafe-shutdowns"),
        ("nvme.smart.media-errors", "media-errors"),
        ("nvme.smart.error-log-entries", "error-log-entries"),
        ("nvme.smart.warning-temperature-time", "warning-temp-time"),
        (
            "nvme.smart.critical-composite-temperature-time",
            "critical-temp-time",
        ),
        ("nvme.smart.temperature-sensor-1-kelvin", "temp-sensor-1-k"),
        ("nvme.smart.temperature-sensor-2-kelvin", "temp-sensor-2-k"),
        ("nvme.smart.temperature-sensor-3-kelvin", "temp-sensor-3-k"),
        ("nvme.smart.temperature-sensor-4-kelvin", "temp-sensor-4-k"),
        ("nvme.smart.temperature-sensor-5-kelvin", "temp-sensor-5-k"),
        ("nvme.smart.temperature-sensor-6-kelvin", "temp-sensor-6-k"),
        ("nvme.smart.temperature-sensor-7-kelvin", "temp-sensor-7-k"),
        ("nvme.smart.temperature-sensor-8-kelvin", "temp-sensor-8-k"),
        (
            "nvme.smart.thermal-temp1-transition-count",
            "thermal-temp1-transitions",
        ),
        (
            "nvme.smart.thermal-temp2-transition-count",
            "thermal-temp2-transitions",
        ),
        (
            "nvme.smart.thermal-temp1-total-time",
            "thermal-temp1-total-time",
        ),
        (
            "nvme.smart.thermal-temp2-total-time",
            "thermal-temp2-total-time",
        ),
        ("lsblk.type", "lsblk-type"),
        ("lsblk.logical-sector-size", "logical-sector"),
        ("lsblk.physical-sector-size", "physical-sector"),
        ("lsblk.minimum-io-size", "minimum-io"),
        ("lsblk.optimal-io-size", "optimal-io"),
        ("lsblk.discard-alignment", "discard-alignment"),
        ("lsblk.discard-granularity", "discard-granularity"),
        ("lsblk.discard-max", "discard-max"),
        ("lsblk.discard-zeroes-data", "discard-zeroes"),
        ("lsblk.scheduler", "scheduler"),
        ("lsblk.request-queue-size", "rq-size"),
        ("lsblk.write-same-max", "write-same-max"),
        ("lsblk.zoned", "zoned"),
        ("lsblk.zone-size", "zone-size"),
        ("lsblk.zone-write-granularity", "zone-write-granularity"),
        ("lsblk.zone-append-max", "zone-append-max"),
        ("lsblk.zone-count", "zone-count"),
        ("lsblk.zone-open-max", "zone-open-max"),
        ("lsblk.zone-active-max", "zone-active-max"),
        ("lsblk.dax", "dax"),
        ("lsblk.hotplug", "hotplug"),
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
        ("partition.start-bytes", "start-bytes"),
        ("partition.end", "end"),
        ("partition.end-bytes", "end-bytes"),
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
        ("zram.algorithm", "zram-algorithm"),
        ("zram.streams", "zram-streams"),
        ("zram.disksize", "zram-disksize"),
        ("zram.data", "zram-data"),
        ("zram.compressed", "zram-compressed"),
        ("zram.total", "zram-total"),
        ("zram.memory-limit", "zram-memory-limit"),
        ("zram.memory-used", "zram-memory-used"),
        ("zram.memory-peak", "zram-memory-peak"),
        ("zram.zero-pages", "zram-zero-pages"),
        ("zram.migrated", "zram-migrated"),
        ("zram.compression-ratio", "zram-ratio"),
        ("zram.mountpoint", "zram-mountpoint"),
        ("zram.swap", "zram-swap"),
        ("loop.backing", "loop-backing"),
        ("loop.back-file", "back-file"),
        ("loop.backing-inode", "back-ino"),
        ("loop.backing-major-minor", "back-major-minor"),
        ("loop.major-minor", "major-minor"),
        ("loop.offset", "offset"),
        ("loop.sizelimit", "sizelimit"),
        ("loop.logical-sector-size", "logical-sector"),
        ("loop.autoclear", "autoclear"),
        ("loop.partscan", "partscan"),
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
        ("udev.id-fs-block-size", "udev-fs-block-size"),
        ("udev.id-fs-lastblock", "udev-fs-lastblock"),
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
        ("btrfs.qgroup-parents", "qgroup-parents"),
        ("btrfs.qgroup-children", "qgroup-children"),
        ("btrfs.mount-target", "mount-target"),
        ("btrfs.device-id", "device-id"),
        ("btrfs.device-stat-write-io-errs", "write-io-errs"),
        ("btrfs.device-stat-read-io-errs", "read-io-errs"),
        ("btrfs.device-stat-flush-io-errs", "flush-io-errs"),
        ("btrfs.device-stat-corruption-errs", "corruption-errs"),
        ("btrfs.device-stat-generation-errs", "generation-errs"),
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
        ("vdo.data-blocks-used-bytes", "data-bytes"),
        ("vdo.overhead-blocks-used", "overhead-blocks"),
        ("vdo.overhead-blocks-used-bytes", "overhead-bytes"),
        ("vdo.logical-blocks-used", "logical-blocks"),
        ("vdo.logical-blocks-used-bytes", "logical-bytes"),
        ("dm.name", "dm-name"),
        ("dm.uuid", "dm-uuid"),
        ("dm.major", "dm-major"),
        ("dm.minor", "dm-minor"),
        ("dm.open-count", "open"),
        ("dm.segments", "segments"),
        ("dm.events", "events"),
        ("dm.table.targets", "dm-table-targets"),
        ("dm.table.segment-count", "dm-table-segments"),
        ("dm.table.segment.0.start", "dm-table-start"),
        ("dm.table.segment.0.length", "dm-table-length"),
        ("dm.table.segment.0.target", "dm-table-target"),
        ("dm.table.segment.0.payload", "dm-table-payload"),
        ("dm.table.segment.0.device", "dm-table-device"),
        ("dm.table.segment.0.offset", "dm-table-offset"),
        (
            "dm.table.segment.0.metadata-device",
            "dm-table-metadata-device",
        ),
        ("dm.table.segment.0.data-device", "dm-table-data-device"),
        (
            "dm.table.segment.0.data-block-size",
            "dm-table-data-block-size",
        ),
        (
            "dm.table.segment.0.low-water-mark",
            "dm-table-low-water-mark",
        ),
        ("dm.table.segment.0.pool-device", "dm-table-pool-device"),
        ("dm.table.segment.0.thin-device-id", "dm-table-thin-id"),
        (
            "dm.table.segment.0.external-origin-device",
            "dm-table-external-origin",
        ),
        ("dm.table.segment.0.cache-device", "dm-table-cache-device"),
        ("dm.table.segment.0.origin-device", "dm-table-origin-device"),
        ("dm.table.segment.0.block-size", "dm-table-block-size"),
        ("dm.table.segment.0.cow-device", "dm-table-cow-device"),
        ("dm.table.segment.0.persistence", "dm-table-persistence"),
        ("dm.table.segment.0.chunk-size", "dm-table-chunk-size"),
        ("dm.table.segment.0.stripe-count", "dm-table-stripes"),
        (
            "dm.table.segment.0.stripe.0.device",
            "dm-table-stripe0-device",
        ),
        (
            "dm.table.segment.0.stripe.0.offset",
            "dm-table-stripe0-offset",
        ),
        (
            "dm.table.segment.0.stripe.1.device",
            "dm-table-stripe1-device",
        ),
        (
            "dm.table.segment.0.stripe.1.offset",
            "dm-table-stripe1-offset",
        ),
        ("dm.table.segment.0.crypt.cipher", "dm-crypt-cipher"),
        ("dm.table.segment.0.crypt.iv-offset", "dm-crypt-iv-offset"),
        ("dm.table.segment.0.crypt.device", "dm-crypt-device"),
        ("dm.table.segment.0.crypt.offset", "dm-crypt-offset"),
        ("dm.status.targets", "dm-status-targets"),
        ("dm.status.segment-count", "dm-status-segments"),
        ("dm.status.segment.0.start", "dm-status-start"),
        ("dm.status.segment.0.length", "dm-status-length"),
        ("dm.status.segment.0.target", "dm-status-target"),
        ("dm.status.segment.0.payload", "dm-status-payload"),
        (
            "dm.status.segment.0.metadata-used-blocks",
            "dm-status-metadata-used",
        ),
        (
            "dm.status.segment.0.metadata-total-blocks",
            "dm-status-metadata-total",
        ),
        (
            "dm.status.segment.0.cache-used-blocks",
            "dm-status-cache-used",
        ),
        (
            "dm.status.segment.0.cache-total-blocks",
            "dm-status-cache-total",
        ),
        (
            "dm.status.segment.0.data-used-blocks",
            "dm-status-data-used",
        ),
        (
            "dm.status.segment.0.data-total-blocks",
            "dm-status-data-total",
        ),
        ("dm.status.segment.0.read-hits", "dm-status-read-hits"),
        ("dm.status.segment.0.read-misses", "dm-status-read-misses"),
        ("dm.status.segment.0.write-hits", "dm-status-write-hits"),
        ("dm.status.segment.0.write-misses", "dm-status-write-misses"),
        ("dm.status.segment.0.dirty-blocks", "dm-status-dirty"),
        ("dm.status.segment.0.mode", "dm-status-mode"),
        ("dm.status.segment.0.used-sectors", "dm-status-used-sectors"),
        (
            "dm.status.segment.0.total-sectors",
            "dm-status-total-sectors",
        ),
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
        ("cryptsetup.luks-keyslot-0-salt", "keyslot-0-salt"),
        (
            "cryptsetup.luks-keyslot-0-af-stripes",
            "keyslot-0-af-stripes",
        ),
        (
            "cryptsetup.luks-keyslot-0-area-offset",
            "keyslot-0-area-offset",
        ),
        (
            "cryptsetup.luks-keyslot-0-area-length",
            "keyslot-0-area-length",
        ),
        ("cryptsetup.luks-keyslot-0-digest-id", "keyslot-0-digest"),
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
        ("cryptsetup.luks-keyslot-1-salt", "keyslot-1-salt"),
        (
            "cryptsetup.luks-keyslot-1-af-stripes",
            "keyslot-1-af-stripes",
        ),
        (
            "cryptsetup.luks-keyslot-1-area-offset",
            "keyslot-1-area-offset",
        ),
        (
            "cryptsetup.luks-keyslot-1-area-length",
            "keyslot-1-area-length",
        ),
        ("cryptsetup.luks-keyslot-1-digest-id", "keyslot-1-digest"),
        ("cryptsetup.luks-token-0-type", "token-0"),
        ("cryptsetup.luks-token-0-keyslot", "token-0-keyslot"),
        ("cryptsetup.luks-token-0-keyslots", "token-0-keyslots"),
        ("cryptsetup.luks-token-0-tpm2-pcrs", "token-0-tpm2-pcrs"),
        ("cryptsetup.luks-token-0-tpm2-hash", "token-0-tpm2-hash"),
        ("cryptsetup.luks-token-1-type", "token-1"),
        ("cryptsetup.luks-token-1-keyslot", "token-1-keyslot"),
        ("cryptsetup.luks-token-1-keyslots", "token-1-keyslots"),
        ("cryptsetup.luks-token-1-tpm2-pcrs", "token-1-tpm2-pcrs"),
        ("cryptsetup.luks-token-1-tpm2-hash", "token-1-tpm2-hash"),
        ("cryptsetup.luks-digest-count", "digests"),
        ("cryptsetup.luks-digests", "digest-ids"),
        ("cryptsetup.luks-digest-0-type", "digest-0"),
        ("cryptsetup.luks-digest-0-hash", "digest-0-hash"),
        ("cryptsetup.luks-digest-0-iterations", "digest-0-iterations"),
        ("cryptsetup.luks-digest-0-salt", "digest-0-salt"),
        ("cryptsetup.luks-digest-0-digest", "digest-0-digest"),
        ("cryptsetup.luks-digest-1-type", "digest-1"),
        ("cryptsetup.luks-digest-1-hash", "digest-1-hash"),
        ("cryptsetup.luks-digest-1-iterations", "digest-1-iterations"),
        ("cryptsetup.luks-digest-1-salt", "digest-1-salt"),
        ("cryptsetup.luks-digest-1-digest", "digest-1-digest"),
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
        ("multipath.path-flags", "path-flags"),
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
        ("md.persistence", "persistence"),
        ("md.bitmap", "bitmap-detail"),
        ("md.mdstat-state", "mdstat-state"),
        ("md.mdstat-level", "mdstat-level"),
        ("md.mdstat-blocks", "mdstat-blocks"),
        ("md.mdstat-superblock", "mdstat-superblock"),
        ("md.mdstat-layout", "mdstat-layout"),
        ("md.mdstat-chunk-size", "mdstat-chunk"),
        ("md.mdstat-devices", "mdstat-devices"),
        ("md.mdstat-health", "mdstat-health"),
        ("md.mdstat-progress", "mdstat-progress"),
        ("md.mdstat-progress-percent", "mdstat-progress-percent"),
        ("md.mdstat-progress-blocks", "mdstat-progress-blocks"),
        ("md.mdstat-finish", "mdstat-finish"),
        ("md.mdstat-speed", "mdstat-speed"),
        ("md.mdstat-bitmap", "mdstat-bitmap"),
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
        ("md.mdstat-member-slot", "mdstat-member-slot"),
        ("md.mdstat-member-flags", "mdstat-member-flags"),
        ("md.uuid", "md-uuid"),
        ("iscsi.target", "target"),
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
        ("ext.magic-number", "ext-magic"),
        ("ext.revision", "ext-revision"),
        ("ext.errors-behavior", "errors"),
        ("ext.fs-error-count", "fs-error-count"),
        ("ext.os-type", "os"),
        ("ext.block-count", "blocks"),
        ("ext.reserved-block-count", "reserved-blocks"),
        ("ext.overhead-clusters", "overhead-clusters"),
        ("ext.free-blocks", "free-blocks"),
        ("ext.first-block", "first-block"),
        ("ext.block-size", "block-size"),
        ("ext.fragment-size", "fragment-size"),
        ("ext.blocks-per-group", "blocks-per-group"),
        ("ext.fragments-per-group", "fragments-per-group"),
        ("ext.inode-count", "inodes"),
        ("ext.free-inodes", "free-inodes"),
        ("ext.inodes-per-group", "inodes-per-group"),
        ("ext.raid-stride", "raid-stride"),
        ("ext.raid-stripe-width", "raid-stripe-width"),
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
        ("ext.first-error-time", "first-error-time"),
        ("ext.first-error-function", "first-error-function"),
        ("ext.first-error-line", "first-error-line"),
        ("ext.first-error-inode", "first-error-inode"),
        ("ext.first-error-block", "first-error-block"),
        ("ext.last-error-time", "last-error-time"),
        ("ext.last-error-function", "last-error-function"),
        ("ext.last-error-line", "last-error-line"),
        ("ext.last-error-inode", "last-error-inode"),
        ("ext.last-error-block", "last-error-block"),
        ("ext.checksum-type", "checksum-type"),
        ("ext.checksum", "checksum"),
        ("exfat.guid", "guid"),
        ("exfat.volume-label", "exfat-label"),
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
        ("ntfs.device-name", "ntfs-device"),
        ("ntfs.device-state", "ntfs-device-state"),
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
        ("ntfs.mft-zone-multiplier", "ntfs-mft-zone-multiplier"),
        ("ntfs.mft-zone-start", "ntfs-mft-zone-start"),
        ("ntfs.mft-zone-end", "ntfs-mft-zone-end"),
        ("ntfs.mft-data-position", "ntfs-mft-data-position"),
        ("ntfs.mft-lcn", "ntfs-mft-lcn"),
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
        ("scsi.address", "scsi-address"),
        ("scsi.host", "scsi-host"),
        ("scsi.channel", "scsi-channel"),
        ("scsi.target", "scsi-target"),
        ("scsi.lun", "scsi-lun"),
        ("scsi.peripheral-type", "scsi-type"),
        ("scsi.vendor", "scsi-vendor"),
        ("scsi.model", "scsi-model"),
        ("scsi.revision", "scsi-revision"),
        ("scsi.block-device", "scsi-block"),
        ("scsi.generic-device", "scsi-generic"),
        ("scsi.size", "scsi-size"),
        ("scsi.transport", "scsi-transport"),
        ("scsi.unit-name", "scsi-unit"),
        ("scsi.by-id", "scsi-by-id"),
        ("scsi.wwn", "scsi-wwn"),
        ("scsi.state", "scsi-state"),
        ("scsi.queue-depth", "scsi-queue-depth"),
        ("scsi.queue-type", "scsi-queue-type"),
        ("scsi.scsi-level", "scsi-level"),
        ("scsi.timeout", "scsi-timeout"),
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
        ("bcache.set-average-key-size", "set-average-key-size"),
        ("bcache.set-btree-cache-size", "set-btree-cache-size"),
        (
            "bcache.set-cache-available-percent",
            "set-available-percent",
        ),
        ("bcache.set-congested", "set-congested"),
        (
            "bcache.set-congested-read-threshold-us",
            "set-congested-read-us",
        ),
        (
            "bcache.set-congested-write-threshold-us",
            "set-congested-write-us",
        ),
        ("bcache.set-io-error-halflife", "set-io-error-halflife"),
        ("bcache.set-io-error-limit", "set-io-error-limit"),
        ("bcache.set-journal-delay-ms", "set-journal-delay-ms"),
        ("bcache.set-root-usage-percent", "set-root-usage-percent"),
        ("bcache.label", "label"),
        ("bcache.state", "state"),
        ("bcache.running", "running"),
        ("bcache.cache-available-percent", "available-percent"),
        ("bcache.cache-mode", "cache-mode"),
        ("bcache.cache-replacement-policy", "replacement"),
        ("bcache.congested-read-threshold-us", "congested-read-us"),
        ("bcache.congested-write-threshold-us", "congested-write-us"),
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
        ("bcache.writeback-rate-debug", "writeback-rate-debug"),
        ("bcache.writeback-rate-d-term", "writeback-rate-d-term"),
        (
            "bcache.writeback-rate-i-term-inverse",
            "writeback-rate-i-inverse",
        ),
        ("bcache.writeback-rate-minimum", "writeback-rate-min"),
        (
            "bcache.writeback-rate-p-term-inverse",
            "writeback-rate-p-inverse",
        ),
        (
            "bcache.writeback-rate-update-seconds",
            "writeback-rate-update",
        ),
        ("bcache.writeback-running", "writeback-running"),
        ("zfs.health", "health"),
        ("zfs.pool-capacity", "pool-capacity"),
        ("zfs.pool-dedupratio", "pool-dedupratio"),
        ("zfs.pool-fragmentation", "pool-fragmentation"),
        ("zfs.pool-altroot", "pool-altroot"),
        ("zfs.pool-ashift", "pool-ashift"),
        ("zfs.pool-autotrim", "pool-autotrim"),
        ("zfs.pool-autoexpand", "pool-autoexpand"),
        ("zfs.pool-autoreplace", "pool-autoreplace"),
        ("zfs.pool-bootfs", "pool-bootfs"),
        ("zfs.pool-cachefile", "pool-cachefile"),
        ("zfs.pool-comment", "pool-comment"),
        ("zfs.pool-delegation", "pool-delegation"),
        ("zfs.pool-failmode", "pool-failmode"),
        ("zfs.pool-listsnapshots", "pool-listsnapshots"),
        ("zfs.pool-multihost", "pool-multihost"),
        ("zfs.state", "state"),
        ("zfs.status", "status"),
        ("zfs.action", "action"),
        ("zfs.scan", "scan"),
        ("zfs.errors", "errors"),
        ("zfs.pool-read-errors", "pool-read-errors"),
        ("zfs.pool-write-errors", "pool-write-errors"),
        ("zfs.pool-checksum-errors", "pool-checksum-errors"),
        ("zfs.vdev-role", "vdev-role"),
        ("zfs.vdev-state", "vdev-state"),
        ("zfs.read-errors", "read-errors"),
        ("zfs.write-errors", "write-errors"),
        ("zfs.checksum-errors", "checksum-errors"),
        ("zfs.origin", "origin"),
        ("zfs.userrefs", "userrefs"),
        ("zfs.holds", "holds"),
        ("zfs.compression", "compression"),
        ("zfs.quota", "quota"),
        ("zfs.reservation", "reservation"),
        ("zfs.encryption", "encryption"),
        ("zfs.keystatus", "keystatus"),
        ("zfs.volsize", "volsize"),
        ("zfs.recordsize", "recordsize"),
        ("zfs.dedup", "dedup"),
        ("zfs.checksum", "checksum"),
        ("zfs.copies", "copies"),
        ("zfs.sync", "sync"),
        ("zfs.primarycache", "primarycache"),
        ("zfs.secondarycache", "secondarycache"),
        ("zfs.atime", "atime"),
        ("zfs.relatime", "relatime"),
        ("zfs.snapdir", "snapdir"),
        ("zfs.acltype", "acltype"),
        ("zfs.xattr", "xattr"),
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

fn consumer_count(graph: &StorageGraph, node: &Node) -> usize {
    graph
        .edges
        .iter()
        .filter(|edge| {
            edge.from == node.id
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

fn vdo_logical_display(node: &Node) -> String {
    property_value(node, "vdo.logical-size")
        .or_else(|| property_value(node, "lvm.vdo-logical-size"))
        .map(str::to_string)
        .unwrap_or_else(|| human_bytes(node.size_bytes))
}

fn vdo_physical_display(node: &Node) -> String {
    property_value(node, "vdo.physical-size")
        .or_else(|| property_value(node, "lvm.vdo-physical-size"))
        .map(str::to_string)
        .unwrap_or_else(|| human_bytes(node.usage.as_ref().and_then(|usage| usage.used_bytes)))
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
    use disk_nix_exec::{ExecutionMode, prepare_execution};
    use disk_nix_model::{Edge, Identity, Node, NodeKind, Relationship, StorageGraph, Usage};
    use disk_nix_plan::{compare_plan_with_topology, plan_and_policy_from_json_bytes};
    use disk_nix_probe::{ProbeReport, ProbeStatus};
    use serde_json::Value;

    use super::{
        ProbePreflightEnvironment, ProbeStatusPreflightReport, ToolVersionReport,
        ToolVersionStatus, apply_receipt, command_stdout_first_line, confirmation_file_accepts,
        consumer_count, is_backing_file_node, is_bcachefs_node, is_btrfs_node, is_cache_node,
        is_complex_filesystem_node, is_device_node, is_dm_node, is_encryption_node,
        is_filesystem_node, is_iscsi_node, is_loop_node, is_lun_node, is_lvm_node, is_mapping_node,
        is_multipath_node, is_network_storage_node, is_nfs_node, is_nvme_node, is_partition_node,
        is_pool_node, is_raid_node, is_snapshot_node, is_swap_node, is_vdo_node, is_volume_node,
        is_zfs_node, is_zram_node, iscsi_lun_count, member_count, migration_report_from_json_bytes,
        mount_details, nfs_mount_count, parse_os_release, print_backing_files, print_bcachefs,
        print_btrfs, print_cache, print_complex_filesystems, print_devices, print_dm,
        print_encryption, print_filesystems, print_filtered_json, print_inspect,
        print_inspect_json, print_iscsi, print_loop, print_luns, print_lvm, print_mappings,
        print_migration_report, print_mounts, print_multipath, print_network_storage, print_nfs,
        print_nvme, print_partitions, print_pools, print_probe_preflight_checks,
        print_probe_preflight_environment, print_probe_reports, print_raid, print_snapshots,
        print_swap, print_usage, print_vdo, print_volumes, print_zfs, print_zram,
        probe_preflight_checks, script_refusal_message, snapshot_source,
        storage_tool_version_report, usage_details, usage_percent, zfs_child_count,
    };

    fn assert_mapping(
        mappings: &[super::LegacyMigrationMapping],
        source: &str,
        target: &str,
        scope: &str,
    ) {
        assert!(
            mappings.iter().any(|mapping| {
                mapping.source == source && mapping.target == target && mapping.scope == scope
            }),
            "missing mapping {source} -> {target} ({scope}) in {mappings:?}"
        );
    }

    #[test]
    fn confirmation_file_accepts_exact_token_line() {
        assert!(confirmation_file_accepts("disk-nix confirm\n"));
        assert!(confirmation_file_accepts("# reviewed\ndisk-nix confirm\n"));
        assert!(confirmation_file_accepts("  disk-nix confirm  \n"));
    }

    #[test]
    fn probe_status_output_includes_remediation_hints() {
        let reports = vec![ProbeReport {
            adapter: "lvm".to_string(),
            status: ProbeStatus::Partial,
            message: Some("permission denied while reading device mapper state".to_string()),
        }];
        let mut output = Vec::new();
        print_probe_reports(&mut output, &reports).expect("probe reports should render");
        let output = String::from_utf8(output).expect("probe status output is utf8");
        assert!(output.contains("permission-denied"));
        assert!(output.contains("remediation:"));
        assert!(output.contains("privileges"));
    }

    #[test]
    fn probe_preflight_parses_os_release_fields() {
        let fields = parse_os_release(
            r#"
ID=nixos
VERSION_ID="26.05"
PRETTY_NAME="NixOS 26.05 (Hermetic)"
# ignored
"#,
        );

        assert_eq!(
            fields
                .iter()
                .find(|(key, _)| key == "ID")
                .map(|(_, value)| value.as_str()),
            Some("nixos")
        );
        assert_eq!(
            fields
                .iter()
                .find(|(key, _)| key == "VERSION_ID")
                .map(|(_, value)| value.as_str()),
            Some("26.05")
        );
        assert_eq!(
            fields
                .iter()
                .find(|(key, _)| key == "PRETTY_NAME")
                .map(|(_, value)| value.as_str()),
            Some("NixOS 26.05 (Hermetic)")
        );
    }

    #[test]
    fn probe_preflight_tool_version_reports_missing_tools() {
        let report = storage_tool_version_report(
            "disk-nix-definitely-missing-tool-for-test",
            &["--version"],
        );

        assert_eq!(
            report.tool,
            "disk-nix-definitely-missing-tool-for-test".to_string()
        );
        assert_eq!(report.status, ToolVersionStatus::Unavailable);
        assert!(report.version.is_none());
        assert!(
            report
                .message
                .as_deref()
                .is_some_and(|message| message.contains("not found"))
        );
    }

    #[test]
    fn probe_preflight_command_version_output_handles_common_variants() {
        let stdout = command_stdout_first_line("sh", &["-c", "printf 'tool 1.0\\n'"])
            .expect("stdout version text should parse");
        assert_eq!(stdout, "tool 1.0");

        let stderr = command_stdout_first_line("sh", &["-c", "printf 'tool 2.0\\n' >&2"])
            .expect("stderr version text should parse");
        assert_eq!(stderr, "tool 2.0");

        let empty = command_stdout_first_line("sh", &["-c", ":"])
            .expect_err("empty successful version output should fail preflight");
        assert!(empty.contains("returned no version text"));

        let nonzero =
            command_stdout_first_line("sh", &["-c", "printf 'bad version\\n' >&2; exit 2"])
                .expect_err("nonzero version command should fail preflight");
        assert!(nonzero.contains("failed with status"));
        assert!(nonzero.contains("bad version"));
    }

    #[test]
    fn probe_preflight_human_output_includes_environment_and_tools() {
        let environment = ProbePreflightEnvironment {
            os_id: Some("nixos".to_string()),
            os_version_id: Some("26.05".to_string()),
            os_pretty_name: Some("NixOS 26.05".to_string()),
            kernel_release: Some("6.12.0".to_string()),
            effective_uid: Some("0".to_string()),
            tool_versions: vec![
                ToolVersionReport {
                    tool: "lsblk".to_string(),
                    status: ToolVersionStatus::Available,
                    version: Some("lsblk from util-linux 2.41".to_string()),
                    message: None,
                },
                ToolVersionReport {
                    tool: "zpool".to_string(),
                    status: ToolVersionStatus::Unavailable,
                    version: None,
                    message: Some("zpool not found or failed to run".to_string()),
                },
            ],
        };

        let mut output = Vec::new();
        print_probe_preflight_environment(&mut output, &environment)
            .expect("preflight environment renders");
        let output = String::from_utf8(output).expect("preflight output is utf8");
        assert!(output.contains("Preflight environment:"));
        assert!(output.contains("NixOS 26.05"));
        assert!(output.contains("effective-uid: 0"));
        assert!(output.contains("lsblk"));
        assert!(output.contains("zpool"));
        assert!(output.contains("unavailable"));

        let checks = probe_preflight_checks(&environment);
        let mut output = Vec::new();
        print_probe_preflight_checks(&mut output, &checks).expect("preflight checks render");
        let output = String::from_utf8(output).expect("preflight checks output is utf8");
        assert!(output.contains("Preflight checks:"));
        assert!(output.contains("status: degraded"));
        assert!(output.contains("missing-tools: zpool"));
        assert!(output.contains("remediation:"));
    }

    #[test]
    fn probe_preflight_json_wraps_environment_and_reports() {
        let environment = ProbePreflightEnvironment {
            os_id: Some("nixos".to_string()),
            os_version_id: Some("26.05".to_string()),
            os_pretty_name: Some("NixOS 26.05".to_string()),
            kernel_release: Some("6.12.0".to_string()),
            effective_uid: Some("0".to_string()),
            tool_versions: vec![ToolVersionReport {
                tool: "lsblk".to_string(),
                status: ToolVersionStatus::Available,
                version: Some("lsblk from util-linux 2.41".to_string()),
                message: None,
            }],
        };
        let preflight_checks = probe_preflight_checks(&environment);
        let report = ProbeStatusPreflightReport {
            environment,
            preflight_checks,
            reports: vec![ProbeReport {
                adapter: "lsblk".to_string(),
                status: ProbeStatus::Available,
                message: Some("normalized graph nodes".to_string()),
            }],
        };

        let json = serde_json::to_value(&report).expect("preflight report serializes");
        assert_eq!(json["environment"]["osId"], "nixos");
        assert_eq!(json["environment"]["toolVersions"][0]["tool"], "lsblk");
        assert_eq!(json["preflightChecks"]["status"], "ready");
        assert_eq!(json["preflightChecks"]["root"], true);
        assert_eq!(json["preflightChecks"]["unavailableToolCount"], 0);
        assert!(
            json["preflightChecks"]["adapterRemediation"]
                .as_array()
                .is_some_and(|items| items.iter().any(|item| {
                    item["adapter"] == "nvme-id-ns"
                        && item["canonicalAdapter"] == "nvme"
                        && item["nixPackages"].as_array().is_some_and(|packages| {
                            packages.iter().any(|package| package == "pkgs.nvme-cli")
                        })
                }))
        );
        assert_eq!(json["reports"][0]["adapter"], "lsblk");
        assert_eq!(json["reports"][0]["category"], "none");
    }

    #[test]
    fn apply_receipt_wraps_report_with_invocation_metadata() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "filesystems": {
                  "root": {
                    "mountpoint": "/",
                    "fsType": "ext4",
                    "resizePolicy": "grow-only",
                    "desiredSize": "40G"
                  }
                }
              },
              "apply": {
                "mode": "manual"
              }
            }"#,
        )
        .expect("spec should parse");
        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);
        let receipt = apply_receipt("apply", "/etc/disk-nix/spec.json", true, false, 42, &report);
        let value: Value =
            serde_json::to_value(&receipt).expect("receipt should serialize to JSON");

        assert_eq!(value["receiptVersion"], 1);
        assert_eq!(value["command"], "apply");
        assert_eq!(value["specPath"], "/etc/disk-nix/spec.json");
        assert_eq!(value["probeCurrent"], true);
        assert_eq!(value["executeRequested"], false);
        assert_eq!(value["generatedAtUnixSeconds"], 42);
        assert_eq!(value["report"]["status"], "dry-run");
        assert!(value["report"]["commandSummary"].is_object());
    }

    #[test]
    fn script_refusal_message_mentions_graph_dependency_conflicts() {
        let (plan, policy) = plan_and_policy_from_json_bytes(
            br#"{
              "spec": {
                "filesystems": {
                  "root": {
                    "operation": "grow",
                    "device": "/dev/mapper/cryptroot",
                    "mountpoint": "/",
                    "fsType": "xfs",
                    "resizePolicy": "grow-only",
                    "desiredSize": "200GiB"
                  }
                },
                "luks": {
                  "devices": {
                    "cryptroot": {
                      "operation": "close",
                      "device": "/dev/disk/by-partuuid/root",
                      "target": "cryptroot"
                    }
                  }
                }
              },
              "apply": {
                "allowGrow": true,
                "allowOffline": true
              }
            }"#,
        )
        .expect("spec should parse");
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("luks:cryptroot", NodeKind::LuksContainer, "cryptroot")
                .with_path("/dev/mapper/cryptroot"),
        );
        graph.add_node(
            Node::new("filesystem:/", NodeKind::Filesystem, "root")
                .with_path("/")
                .with_property("filesystem.type", "xfs")
                .with_size_bytes(100 * 1024 * 1024 * 1024),
        );
        graph.add_edge(Edge::new(
            "luks:cryptroot",
            "filesystem:/",
            Relationship::Backs,
        ));
        let plan = compare_plan_with_topology(plan, &graph);
        let report = prepare_execution(&plan, policy, ExecutionMode::DryRun);

        assert!(!report.can_apply());
        let message = script_refusal_message(&report);
        assert!(message.contains("conflict-free command plan"));
        assert!(message.contains("1 graph dependency conflict"));
        assert!(message.contains("plan splitting or ordering review"));
    }

    #[test]
    fn migration_report_adds_current_version_to_direct_specs() {
        let report = migration_report_from_json_bytes(
            br#"{
              "filesystems": {
                "root": {
                  "mountpoint": "/",
                  "fsType": "ext4"
                }
              }
            }"#,
        )
        .expect("versionless spec should migrate");

        assert_eq!(report.source_version, None);
        assert_eq!(report.target_version, 1);
        assert!(report.migrated);
        assert_eq!(report.spec["version"], 1);
        assert!(
            report
                .changes
                .iter()
                .any(|change| change == "set version to 1")
        );
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("does not apply storage mutations"))
        );
        assert_eq!(report.version_migrations.len(), 2);
        let legacy_contract = report
            .version_migrations
            .iter()
            .find(|contract| contract.source_version.is_none())
            .expect("pre-version migration contract should exist");
        assert_eq!(legacy_contract.target_version, 1);
        assert_eq!(legacy_contract.status, "supported");
        assert_eq!(
            legacy_contract.mapping_scope,
            "pre-version legacy aliases to version 1"
        );
        assert_mapping(
            &legacy_contract.field_mappings,
            "fileSystems",
            "filesystems",
            "top-level",
        );
        assert!(
            legacy_contract
                .safety_notes
                .iter()
                .any(|note| note.contains("does not apply storage mutations"))
        );
    }

    #[test]
    fn migration_report_adds_wrapper_and_spec_versions() {
        let report = migration_report_from_json_bytes(
            br#"{
              "spec": {
                "filesystems": {
                  "root": {
                    "mountpoint": "/",
                    "fsType": "ext4"
                  }
                }
              },
              "apply": {
                "allowGrow": true
              }
            }"#,
        )
        .expect("wrapper spec should migrate");

        assert!(report.migrated);
        assert_eq!(report.spec["version"], 1);
        assert_eq!(report.spec["spec"]["version"], 1);
        assert!(
            report
                .changes
                .iter()
                .any(|change| change == "set version to 1")
        );
        assert!(
            report
                .changes
                .iter()
                .any(|change| change == "set spec.version to 1")
        );
    }

    #[test]
    fn migration_report_maps_legacy_pre_version_aliases() {
        let report = migration_report_from_json_bytes(
            br#"{
              "fileSystems": {
                "root": {
                  "mountpoint": "/",
                  "fsType": "ext4"
                }
              },
              "swapDevices": {
                "swap": {
                  "device": "/dev/disk/by-label/swap",
                  "operation": "rescan"
                }
              },
              "luksDevices": {
                "cryptroot": {
                  "device": "/dev/disk/by-id/luks-root",
                  "operation": "open"
                }
              },
              "nfsMounts": {
                "/srv/shared": {
                  "source": "nas.example.com:/srv/shared",
                  "operation": "mount"
                }
              },
              "iscsiSessions": {
                "iqn.2026-06.example:storage.root": {
                  "portal": "192.0.2.10:3260",
                  "operation": "login"
                }
              }
            }"#,
        )
        .expect("legacy aliases should migrate");

        assert!(report.migrated);
        assert_eq!(report.spec["version"], 1);
        assert_eq!(report.legacy_mappings.len(), 10);
        assert_mapping(
            &report.legacy_mappings,
            "fileSystems",
            "filesystems",
            "top-level",
        );
        assert_mapping(
            &report.legacy_mappings,
            "spec.fileSystems",
            "spec.filesystems",
            "spec",
        );
        assert_mapping(
            &report.legacy_mappings,
            "iscsiSessions",
            "iscsi.sessions",
            "top-level",
        );
        assert_eq!(report.applied_mappings.len(), 5);
        assert_mapping(
            &report.applied_mappings,
            "fileSystems",
            "filesystems",
            "top-level",
        );
        assert_mapping(
            &report.applied_mappings,
            "swapDevices",
            "swaps",
            "top-level",
        );
        assert_mapping(
            &report.applied_mappings,
            "luksDevices",
            "luks.devices",
            "top-level",
        );
        assert_mapping(
            &report.applied_mappings,
            "nfsMounts",
            "nfs.mounts",
            "top-level",
        );
        assert_mapping(
            &report.applied_mappings,
            "iscsiSessions",
            "iscsi.sessions",
            "top-level",
        );
        assert!(report.spec.get("fileSystems").is_none());
        assert!(report.spec.get("swapDevices").is_none());
        assert!(report.spec.get("luksDevices").is_none());
        assert!(report.spec.get("nfsMounts").is_none());
        assert!(report.spec.get("iscsiSessions").is_none());
        assert_eq!(report.spec["filesystems"]["root"]["mountpoint"], "/");
        assert_eq!(report.spec["swaps"]["swap"]["operation"], "rescan");
        assert_eq!(
            report.spec["luks"]["devices"]["cryptroot"]["operation"],
            "open"
        );
        assert_eq!(
            report.spec["nfs"]["mounts"]["/srv/shared"]["source"],
            "nas.example.com:/srv/shared"
        );
        assert_eq!(
            report.spec["iscsi"]["sessions"]["iqn.2026-06.example:storage.root"]["operation"],
            "login"
        );
        assert!(
            report
                .changes
                .iter()
                .any(|change| { change == "mapped legacy field fileSystems to filesystems" })
        );
        assert!(
            report
                .changes
                .iter()
                .any(|change| { change == "mapped legacy field luksDevices to luks.devices" })
        );
    }

    #[test]
    fn migration_report_maps_legacy_wrapper_aliases_inside_spec() {
        let report = migration_report_from_json_bytes(
            br#"{
              "spec": {
                "fileSystems": {
                  "root": {
                    "mountpoint": "/",
                    "fsType": "xfs"
                  }
                },
                "nfsMounts": {
                  "/srv/shared": {
                    "source": "nas.example.com:/srv/shared",
                    "operation": "mount"
                  }
                }
              },
              "apply": {
                "allowOffline": true
              }
            }"#,
        )
        .expect("legacy wrapper aliases should migrate");

        assert!(report.migrated);
        assert_eq!(report.spec["version"], 1);
        assert_eq!(report.spec["spec"]["version"], 1);
        assert_eq!(report.legacy_mappings.len(), 10);
        assert_eq!(report.applied_mappings.len(), 2);
        assert_mapping(
            &report.applied_mappings,
            "spec.fileSystems",
            "spec.filesystems",
            "spec",
        );
        assert_mapping(
            &report.applied_mappings,
            "spec.nfsMounts",
            "spec.nfs.mounts",
            "spec",
        );
        assert!(report.spec["spec"].get("fileSystems").is_none());
        assert!(report.spec["spec"].get("nfsMounts").is_none());
        assert_eq!(report.spec["spec"]["filesystems"]["root"]["fsType"], "xfs");
        assert_eq!(
            report.spec["spec"]["nfs"]["mounts"]["/srv/shared"]["source"],
            "nas.example.com:/srv/shared"
        );
        assert!(report.changes.iter().any(|change| {
            change == "mapped legacy field spec.fileSystems to spec.filesystems"
        }));
        assert!(
            report.changes.iter().any(|change| {
                change == "mapped legacy field spec.nfsMounts to spec.nfs.mounts"
            })
        );
    }

    #[test]
    fn migration_report_rejects_conflicting_legacy_aliases() {
        let error = migration_report_from_json_bytes(
            br#"{
              "fileSystems": {
                "legacy": {
                  "mountpoint": "/legacy",
                  "fsType": "ext4"
                }
              },
              "filesystems": {
                "current": {
                  "mountpoint": "/current",
                  "fsType": "xfs"
                }
              }
            }"#,
        )
        .expect_err("conflicting aliases should be rejected");

        assert!(
            error
                .to_string()
                .contains("legacy field fileSystems conflicts with current field filesystems")
        );
    }

    #[test]
    fn migration_report_does_not_rewrite_explicit_current_version_aliases() {
        let report = migration_report_from_json_bytes(
            br#"{
              "version": 1,
              "fileSystems": {
                "legacy": {
                  "mountpoint": "/legacy",
                  "fsType": "ext4"
                }
              }
            }"#,
        )
        .expect("explicit current-version spec should stay metadata-only");

        assert!(!report.migrated);
        assert_eq!(report.legacy_mappings.len(), 10);
        assert!(report.applied_mappings.is_empty());
        assert!(report.spec.get("fileSystems").is_some());
        assert!(report.spec.get("filesystems").is_none());
        assert!(
            report
                .changes
                .iter()
                .any(|change| change.contains("already declares"))
        );
    }

    #[test]
    fn migration_report_keeps_explicit_current_version() {
        let report = migration_report_from_json_bytes(
            br#"{
              "version": 1,
              "filesystems": {
                "root": {
                  "mountpoint": "/",
                  "fsType": "ext4"
                }
              }
            }"#,
        )
        .expect("current version should validate");

        assert_eq!(report.source_version, Some(1));
        assert!(!report.migrated);
        assert_eq!(report.legacy_mappings.len(), 10);
        assert!(report.applied_mappings.is_empty());
        let current_contract = report
            .version_migrations
            .iter()
            .find(|contract| contract.source_version == Some(1))
            .expect("version 1 migration contract should exist");
        assert_eq!(current_contract.target_version, 1);
        assert_eq!(current_contract.status, "supported");
        assert!(current_contract.field_mappings.is_empty());
        assert!(
            current_contract
                .safety_notes
                .iter()
                .any(|note| note.contains("validated without legacy alias rewrites"))
        );
        assert!(
            report
                .changes
                .iter()
                .any(|change| change.contains("already declares"))
        );
    }

    #[test]
    fn migration_report_json_includes_version_migration_contracts() {
        let report = migration_report_from_json_bytes(
            br#"{
              "fileSystems": {
                "root": {
                  "mountpoint": "/",
                  "fsType": "ext4"
                }
              }
            }"#,
        )
        .expect("legacy spec should migrate");

        let json = serde_json::to_value(&report).expect("report should serialize");
        assert_eq!(json["versionMigrations"][0]["sourceVersion"], Value::Null);
        assert_eq!(json["versionMigrations"][0]["targetVersion"], 1);
        assert_eq!(json["versionMigrations"][0]["status"], "supported");
        assert_eq!(
            json["versionMigrations"][0]["mappingScope"],
            "pre-version legacy aliases to version 1"
        );
        assert!(
            json["versionMigrations"][0]["fieldMappings"]
                .as_array()
                .is_some_and(|mappings| mappings.iter().any(|mapping| {
                    mapping["source"] == "fileSystems"
                        && mapping["target"] == "filesystems"
                        && mapping["scope"] == "top-level"
                }))
        );
        assert_eq!(json["versionMigrations"][1]["sourceVersion"], 1);
        assert_eq!(json["versionMigrations"][1]["targetVersion"], 1);
        assert!(
            json["versionMigrations"][1]["fieldMappings"]
                .as_array()
                .is_some_and(Vec::is_empty)
        );
    }

    #[test]
    fn migration_report_rejects_future_and_conflicting_versions() {
        let future = migration_report_from_json_bytes(
            br#"{
              "version": 2,
              "filesystems": {
                "root": {
                  "mountpoint": "/",
                  "fsType": "ext4"
                }
              }
            }"#,
        )
        .expect_err("future version should not migrate implicitly");
        assert!(
            future
                .to_string()
                .contains("unsupported disk-nix spec version 2")
        );

        let conflict = migration_report_from_json_bytes(
            br#"{
              "version": 1,
              "spec": {
                "version": 2
              }
            }"#,
        )
        .expect_err("conflicting versions should be rejected");
        assert!(
            conflict
                .to_string()
                .contains("conflicting disk-nix spec versions")
        );
    }

    #[test]
    fn migration_report_human_output_includes_migrated_spec() {
        let report = migration_report_from_json_bytes(
            br#"{
              "filesystems": {
                "root": {
                  "mountpoint": "/",
                  "fsType": "ext4"
                }
              }
            }"#,
        )
        .expect("spec should migrate");

        let mut output = Vec::new();
        print_migration_report(&mut output, &report).expect("migration report renders");
        let output = String::from_utf8(output).expect("migration output is utf8");
        assert!(output.contains("Migration: None -> 1"));
        assert!(output.contains("migrated: true"));
        assert!(output.contains("Version migration contracts:"));
        assert!(
            output.contains("- None -> 1: supported (pre-version legacy aliases to version 1)")
        );
        assert!(output.contains("- Some(1) -> 1: supported (version 1 metadata normalization)"));
        assert!(output.contains("Legacy mappings:"));
        assert!(output.contains("- fileSystems -> filesystems (top-level)"));
        assert!(output.contains("- spec.fileSystems -> spec.filesystems (spec)"));
        assert!(output.contains("Applied mappings:"));
        assert!(output.contains("- none"));
        assert!(output.contains("Migrated spec:"));
        assert!(output.contains(r#""version": 1"#));
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
        assert!(is_dm_node(
            &Node::new(
                "block:/dev/mapper/cryptroot",
                NodeKind::DeviceMapper,
                "cryptroot"
            )
            .with_property("dm.name", "cryptroot")
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
        assert!(is_bcachefs_node(&bcachefs));
        assert!(is_complex_filesystem_node(&Node::new(
            "btrfs:/mnt/persist",
            NodeKind::BtrfsFilesystem,
            "/mnt/persist"
        )));
        assert!(is_btrfs_node(&Node::new(
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
        assert!(is_lun_node(&Node::new(
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
        assert!(is_backing_file_node(&Node::new(
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
        assert!(is_swap_node(
            &Node::new("block:/dev/zram0", NodeKind::ZramDevice, "/dev/zram0")
                .with_property("zram.swap", "true")
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
    fn focused_json_includes_direct_relationship_neighbors() {
        let mut graph = StorageGraph::empty();
        graph.add_node(Node::new(
            "filesystem:root",
            NodeKind::Filesystem,
            "/dev/mapper/vg-root",
        ));
        graph.add_node(Node::new("mount:/", NodeKind::Mountpoint, "/"));
        graph.add_node(Node::new(
            "block:/dev/nvme0n1",
            NodeKind::PhysicalDisk,
            "/dev/nvme0n1",
        ));
        graph.add_edge(Edge::new(
            "filesystem:root",
            "mount:/",
            Relationship::MountedAt,
        ));

        let mut output = Vec::new();
        print_filtered_json(&mut output, &graph, is_filesystem_node)
            .expect("filtered graph renders");
        let output = String::from_utf8(output).expect("json is utf8");
        let graph: StorageGraph = serde_json::from_str(&output).expect("valid storage graph json");

        assert_eq!(graph.nodes.len(), 2);
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.id.0 == "filesystem:root")
        );
        assert!(graph.nodes.iter().any(|node| node.id.0 == "mount:/"));
        assert!(
            graph
                .nodes
                .iter()
                .all(|node| node.id.0 != "block:/dev/nvme0n1")
        );
        assert_eq!(
            graph.edges,
            vec![Edge::new(
                "filesystem:root",
                "mount:/",
                Relationship::MountedAt
            )]
        );
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
                .with_property("nvme.namespace-id", "1")
                .with_property(
                    "nvme.namespace-uuid",
                    "12345678-1234-1234-1234-123456789abc",
                )
                .with_property("nvme.eui64", "0011223344556677")
                .with_property("nvme.nguid", "00112233445566778899aabbccddeeff")
                .with_property("nvme.subsystem", "nvme-subsys0")
                .with_property("nvme.controller", "nvme0")
                .with_property("nvme.transport", "pcie")
                .with_property("nvme.controller-id", "1")
                .with_property("nvme.namespace-capacity", "900000000000")
                .with_property("nvme.lba-format", "512 B + 0 B")
                .with_property("nvme.maximum-lba", "1953125")
                .with_property("nvme.sector-size", "512")
                .with_property("nvme.ana-state", "optimized")
                .with_property("lsblk.logical-sector-size", "512")
                .with_property("lsblk.physical-sector-size", "4096")
                .with_property("lsblk.minimum-io-size", "4096")
                .with_property("lsblk.optimal-io-size", "1048576")
                .with_property("lsblk.discard-alignment", "0")
                .with_property("lsblk.discard-granularity", "4096")
                .with_property("lsblk.discard-max", "2147483648")
                .with_property("lsblk.discard-zeroes-data", "false")
                .with_property("lsblk.scheduler", "none")
                .with_property("lsblk.request-queue-size", "1023")
                .with_property("lsblk.write-same-max", "0")
                .with_property("lsblk.zoned", "host-managed")
                .with_property("lsblk.zone-size", "268435456")
                .with_property("lsblk.zone-write-granularity", "4096")
                .with_property("lsblk.zone-append-max", "65536")
                .with_property("lsblk.zone-count", "64")
                .with_property("lsblk.zone-open-max", "32")
                .with_property("lsblk.zone-active-max", "48")
                .with_property("lsblk.dax", "false")
                .with_property("lsblk.hotplug", "false")
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
            .with_property("udev.id-fs-block-size", "512")
            .with_property("udev.id-fs-lastblock", "1048575")
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
                .with_property("loop.backing-inode", "12345")
                .with_property("loop.backing-major-minor", "0:45")
                .with_property("loop.offset", "1048576")
                .with_property("loop.autoclear", "true")
                .with_property("loop.partscan", "true")
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
                .with_property("smartctl.svn-revision", "5530")
                .with_property("smartctl.platform", "x86_64-linux")
                .with_property("smartctl.exit-status", "0")
                .with_property("smartctl.device-name", "/dev/sdb")
                .with_property("smartctl.health.passed", "true")
                .with_property("smartctl.device-type", "sat")
                .with_property("smartctl.protocol", "ATA")
                .with_property("smartctl.model", "Example SSD")
                .with_property("smartctl.model-family", "Example SSDs")
                .with_property("smartctl.serial", "SATA123")
                .with_property("smartctl.revision", "A1")
                .with_property("smartctl.firmware-version", "1.2.3")
                .with_property("smartctl.wwn-naa", "5")
                .with_property("smartctl.wwn-oui", "12345")
                .with_property("smartctl.wwn-id", "67890")
                .with_property("smartctl.user-capacity-bytes", "1000204886016")
                .with_property("smartctl.logical-block-size", "512")
                .with_property("smartctl.physical-block-size", "4096")
                .with_property("smartctl.rotation-rate-rpm", "0")
                .with_property("smartctl.form-factor", "2.5 inches")
                .with_property("smartctl.sata-version", "SATA 3.3")
                .with_property("smartctl.interface-speed-current", "6.0")
                .with_property("smartctl.interface-speed-max", "6.0")
                .with_property("smartctl.power-on-hours", "4242")
                .with_property("smartctl.power-cycle-count", "12")
                .with_property("smartctl.temperature-current-celsius", "31")
                .with_property("smartctl.temperature-highest-celsius", "44")
                .with_property("smartctl.temperature-lowest-celsius", "20")
                .with_property(
                    "smartctl.offline-data-collection-status",
                    "was completed without error",
                )
                .with_property("smartctl.self-test-status", "completed without error")
                .with_property("smartctl.error-log-summary-count", "3")
                .with_property("smartctl.self-test-log-count", "2")
                .with_property("smartctl.error-logging-supported", "true")
                .with_property("smartctl.gp-logging-supported", "true")
                .with_property("smartctl.sct-capabilities", "61")
                .with_property("smartctl.scsi-grown-defect-list", "0")
                .with_property("smartctl.attribute.reallocated-sector-ct.raw", "0")
                .with_property("smartctl.attribute.reallocated-sector-ct.value", "100")
                .with_property("smartctl.attribute.reallocated-sector-ct.worst", "100")
                .with_property("smartctl.attribute.reallocated-sector-ct.threshold", "10")
                .with_property(
                    "smartctl.attribute.reallocated-sector-ct.when-failed",
                    "never",
                )
                .with_property("smartctl.attribute.current-pending-sector.raw", "1")
                .with_property("smartctl.attribute.current-pending-sector.value", "99")
                .with_property("smartctl.attribute.current-pending-sector.worst", "98")
                .with_property("smartctl.attribute.current-pending-sector.threshold", "0")
                .with_property(
                    "smartctl.attribute.current-pending-sector.when-failed",
                    "past",
                )
                .with_property("smartctl.attribute.offline-uncorrectable.raw", "2")
                .with_property("smartctl.attribute.offline-uncorrectable.value", "97")
                .with_property("smartctl.attribute.offline-uncorrectable.worst", "96")
                .with_property("smartctl.attribute.offline-uncorrectable.threshold", "0")
                .with_property(
                    "smartctl.attribute.offline-uncorrectable.when-failed",
                    "past",
                )
                .with_property("scsi.address", "1:0:0:0")
                .with_property("scsi.generic-device", "/dev/sg1")
                .with_property("scsi.transport", "sata:5000c500a5a461dc")
                .with_property("scsi.unit-name", "5000c500a5a461dc")
                .with_property("scsi.queue-depth", "32")
                .with_property("multipath.host-path", "2:0:0:1")
                .with_property("major-minor", "8:16")
                .with_property("multipath.path-flags", "ghost")
                .with_property("multipath.path-state", "active ready running ghost"),
        );

        let mut output = Vec::new();
        print_devices(&mut output, &graph).expect("devices table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("DETAILS"));
        assert!(output.contains("model=FastDisk vendor=Acme transport=nvme rotational=false"));
        assert!(output.contains("nvme-model=Example NVMe product=Example Controller firmware=1.0"));
        assert!(output.contains(
            "ns-index=0 namespace=1 nsid=1 ns-uuid=12345678-1234-1234-1234-123456789abc"
        ));
        assert!(output.contains(
            "eui64=0011223344556677 nguid=00112233445566778899aabbccddeeff subsystem=nvme-subsys0 controller=nvme0"
        ));
        assert!(output.contains(
            "transport=pcie controller-id=1 namespace-capacity=900000000000 lba-format=512 B + 0 B"
        ));
        assert!(output.contains("max-lba=1953125 sector-size=512 ana-state=optimized"));
        assert!(output.contains(
            "logical-sector=512 physical-sector=4096 minimum-io=4096 optimal-io=1048576"
        ));
        assert!(output.contains(
            "discard-alignment=0 discard-granularity=4096 discard-max=2147483648 discard-zeroes=false"
        ));
        assert!(output.contains("scheduler=none rq-size=1023 write-same-max=0 zoned=host-managed"));
        assert!(output.contains(
            "zone-size=268435456 zone-write-granularity=4096 zone-append-max=65536 zone-count=64"
        ));
        assert!(output.contains("zone-open-max=32 zone-active-max=48 dax=false hotplug=false"));
        assert!(output.contains("ptable=gpt"));
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
        assert!(output.contains("udev-fs-block-size=512 udev-fs-lastblock=1048575"));
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
            "lsblk-type=loop back-file=/var/lib/images/root.img back-ino=12345 back-major-minor=0:45 offset=1048576 autoclear=true partscan=true dio=true"
        ));
        assert!(output.contains("loop-backing=true"));
        assert!(output.contains("swap-active=true swap-type=partition swap-priority=100"));
        assert!(output.contains("member-state=active sync"));
        assert!(output.contains(
            "smart-svn=5530 smart-platform=x86_64-linux smart-exit-status=0 smart-device-name=/dev/sdb"
        ));
        assert!(output.contains(
            "smart-health-passed=true smart-device-type=sat smart-protocol=ATA smart-model=Example SSD"
        ));
        assert!(output.contains("smart-family=Example SSDs"));
        assert!(output.contains("smart-revision=A1 smart-firmware=1.2.3"));
        assert!(output.contains(
            "smart-serial=SATA123 smart-wwn-naa=5 smart-wwn-oui=12345 smart-wwn-id=67890"
        ));
        assert!(output.contains("smart-capacity=1000204886016 smart-logical-block=512"));
        assert!(output.contains(
            "smart-physical-block=4096 smart-rpm=0 smart-form-factor=2.5 inches sata-version=SATA 3.3"
        ));
        assert!(output.contains("interface-speed-current=6.0 interface-speed-max=6.0"));
        assert!(output.contains("smart-power-on-hours=4242"));
        assert!(
            output.contains(
                "smart-power-cycles=12 smart-temperature-c=31 smart-temperature-highest-c=44 smart-temperature-lowest-c=20"
            )
        );
        assert!(output.contains(
            "smart-offline-status=was completed without error smart-self-test=completed without error"
        ));
        assert!(output.contains(
            "smart-error-log-count=3 smart-self-test-count=2 smart-error-logging=true smart-gp-logging=true"
        ));
        assert!(output.contains("smart-sct-capabilities=61 smart-scsi-grown-defects=0"));
        assert!(output.contains(
            "reallocated-sectors=0 reallocated-value=100 reallocated-worst=100 reallocated-threshold=10 reallocated-failed=never"
        ));
        assert!(output.contains(
            "pending-sectors=1 pending-value=99 pending-worst=98 pending-threshold=0 pending-failed=past"
        ));
        assert!(
            output.contains(
                "offline-uncorrectable=2 offline-uncorrectable-value=97 offline-uncorrectable-worst=96 offline-uncorrectable-threshold=0 offline-uncorrectable-failed=past"
            )
        );
        assert!(output.contains(
            "scsi-address=1:0:0:0 scsi-generic=/dev/sg1 scsi-transport=sata:5000c500a5a461dc"
        ));
        assert!(output.contains("scsi-unit=5000c500a5a461dc scsi-queue-depth=32"));
        assert!(output.contains(
            "host-path=2:0:0:1 major-minor=8:16 path-flags=ghost path-state=active ready running ghost"
        ));
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
            .with_property("partition.start-bytes", "1049000")
            .with_property("partition.end", "538MB")
            .with_property("partition.end-bytes", "538000000")
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
            "fstype=vfat blkid-type=vfat version=FAT32 blkid-block-size=512 usage=filesystem partlabel=EFI System Partition partno=1 start=1049kB start-bytes=1049000 end=538MB end-bytes=538000000 type=fat32 part-name=ESP flags=boot, esp"
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
        let snapshot = snapshot.with_property("zfs.holds", "disk-nix-retain");
        assert_eq!(usage_details(&snapshot), "userrefs=2 holds=disk-nix-retain");

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
            .with_property("ext.magic-number", "0xEF53")
            .with_property("ext.revision", "1 (dynamic)")
            .with_property("ext.errors-behavior", "Continue")
            .with_property("ext.fs-error-count", "2")
            .with_property("ext.os-type", "Linux")
            .with_property("ext.block-count", "122096646")
            .with_property("ext.reserved-block-count", "6104832")
            .with_property("ext.overhead-clusters", "123456")
            .with_property("ext.free-blocks", "73328197")
            .with_property("ext.first-block", "0")
            .with_property("ext.block-size", "4096")
            .with_property("ext.fragment-size", "4096")
            .with_property("ext.blocks-per-group", "32768")
            .with_property("ext.fragments-per-group", "32768")
            .with_property("ext.inode-count", "30531584")
            .with_property("ext.free-inodes", "27187554")
            .with_property("ext.inodes-per-group", "8192")
            .with_property("ext.raid-stride", "128")
            .with_property("ext.raid-stripe-width", "256")
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
            .with_property("ext.first-error-time", "Mon Jun 22 12:30:00 2026")
            .with_property("ext.first-error-function", "ext4_lookup")
            .with_property("ext.first-error-line", "1234")
            .with_property("ext.first-error-inode", "42")
            .with_property("ext.first-error-block", "9001")
            .with_property("ext.last-error-time", "Mon Jun 22 12:45:00 2026")
            .with_property("ext.last-error-function", "ext4_journal_check_start")
            .with_property("ext.last-error-line", "5678")
            .with_property("ext.last-error-inode", "43")
            .with_property("ext.last-error-block", "9002")
            .with_property("ext.checksum-type", "crc32c")
            .with_property("ext.checksum", "0x12345678");
        assert_eq!(
            usage_details(&ext),
            "fstype=ext4 version=1.0 blkid-block-size=4096 usage=filesystem uuid-sub=subvol-uuid ext-state=clean ext-magic=0xEF53 ext-revision=1 (dynamic) errors=Continue fs-error-count=2 os=Linux blocks=122096646 reserved-blocks=6104832 overhead-clusters=123456 free-blocks=73328197 first-block=0 block-size=4096 fragment-size=4096 blocks-per-group=32768 fragments-per-group=32768 inodes=30531584 free-inodes=27187554 inodes-per-group=8192 raid-stride=128 raid-stripe-width=256 features=has_journal extent metadata_csum flags=signed_directory_hash dir-hash=half_md4 dir-hash-seed=11111111-2222-3333-4444-555555555555 default-mount=user_xattr acl created=Mon Jan 01 00:00:00 2024 last-mounted=Mon Jun 22 12:00:00 2026 last-written=Mon Jun 22 12:00:00 2026 mount-count=12 max-mount-count=-1 last-checked=Mon Jan 01 00:00:00 2024 check-interval=0 (<none>) lifetime-writes=189 GB reserved-uid=0 (user root) reserved-gid=0 (group root) first-inode=11 inode-size=256 journal-inode=8 journal-uuid=99999999-aaaa-bbbb-cccc-dddddddddddd journal-backup=inode blocks journal-features=journal_incompat_revoke journal-size=1024M first-error-time=Mon Jun 22 12:30:00 2026 first-error-function=ext4_lookup first-error-line=1234 first-error-inode=42 first-error-block=9001 last-error-time=Mon Jun 22 12:45:00 2026 last-error-function=ext4_journal_check_start last-error-line=5678 last-error-inode=43 last-error-block=9002 checksum-type=crc32c checksum=0x12345678"
        );

        let exfat = Node::new("fs:/dev/sdb1", NodeKind::Filesystem, "exfat")
            .with_property("exfat.guid", "01234567-89ab-cdef-0123-456789abcdef")
            .with_property("exfat.volume-label", "SHARED")
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
            "guid=01234567-89ab-cdef-0123-456789abcdef exfat-label=SHARED exfatprogs=1.2.4 serial=0x6eef953b sectors=3203072 fat-offset=2048 fat-length=448 cluster-heap-offset=4096 clusters=49984 used-clusters=48960 free-clusters=1024 root-cluster=4 sector-bytes=512 sectors-per-cluster=64 cluster-bytes=32768"
        );

        let ntfs = Node::new("fs:/dev/sda1", NodeKind::Filesystem, "ntfs")
            .with_property("ntfs.device-name", "/dev/sda1")
            .with_property("ntfs.device-state", "11")
            .with_property("ntfs.volume-name", "Windows")
            .with_property("ntfs.volume-serial", "01234567-89abcdef")
            .with_property("ntfs.version", "3.1")
            .with_property("ntfs.sector-size", "512")
            .with_property("ntfs.cluster-size", "4096")
            .with_property("ntfs.volume-size-clusters", "262144")
            .with_property("ntfs.mft-record-size", "1024")
            .with_property("ntfs.mft-zone-multiplier", "0")
            .with_property("ntfs.mft-zone-start", "786432")
            .with_property("ntfs.mft-zone-end", "819200")
            .with_property("ntfs.mft-data-position", "786944")
            .with_property("ntfs.mft-lcn", "4");
        assert_eq!(
            usage_details(&ntfs),
            "ntfs-device=/dev/sda1 ntfs-device-state=11 ntfs-name=Windows ntfs-serial=01234567-89abcdef ntfs-version=3.1 ntfs-sector=512 ntfs-cluster=4096 ntfs-clusters=262144 ntfs-mft-record=1024 ntfs-mft-zone-multiplier=0 ntfs-mft-zone-start=786432 ntfs-mft-zone-end=819200 ntfs-mft-data-position=786944 ntfs-mft-lcn=4"
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
            .with_property("bcache.congested-read-threshold-us", "2000")
            .with_property("bcache.congested-write-threshold-us", "20000")
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
            .with_property("bcache.writeback-rate-debug", "rate=1024")
            .with_property("bcache.writeback-rate-d-term", "30")
            .with_property("bcache.writeback-rate-i-term-inverse", "10000")
            .with_property("bcache.writeback-rate-minimum", "4.0k")
            .with_property("bcache.writeback-rate-p-term-inverse", "40")
            .with_property("bcache.writeback-rate-update-seconds", "5")
            .with_property("bcache.writeback-running", "1");
        assert_eq!(
            usage_details(&bcache),
            "role=backing kind=cache-set backing-device=/dev/sdb1 set-uuid=cache-set-uuid label=fast-cache state=clean running=1 available-percent=78 cache-mode=writeback replacement=lru congested-read-us=2000 congested-write-us=20000 discard=true dirty=64.0M io-errors=0 metadata-written=128.0M priority-stats=Unused: 0% Metadata: 1% readahead=0 sequential-cutoff=4.0M written=512.0M writeback-delay=30 writeback-metadata=true writeback-percent=10 writeback-rate=1.0M/sec writeback-rate-debug=rate=1024 writeback-rate-d-term=30 writeback-rate-i-inverse=10000 writeback-rate-min=4.0k writeback-rate-p-inverse=40 writeback-rate-update=5 writeback-running=1"
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
            .with_property("loop.backing-inode", "12345")
            .with_property("loop.backing-major-minor", "0:45")
            .with_property("loop.major-minor", "7:0")
            .with_property("loop.offset", "1048576")
            .with_property("loop.sizelimit", "1073741824")
            .with_property("loop.logical-sector-size", "512")
            .with_property("loop.autoclear", "true")
            .with_property("loop.partscan", "true")
            .with_property("loop.read-only", "false")
            .with_property("loop.direct-io", "true");
        assert_eq!(
            usage_details(&loop_device),
            "back-file=/var/lib/images/root.img back-ino=12345 back-major-minor=0:45 major-minor=7:0 offset=1048576 sizelimit=1073741824 logical-sector=512 autoclear=true partscan=true ro=false dio=true"
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
        .with_property("nvme.namespace-id", "1")
        .with_property(
            "nvme.namespace-uuid",
            "12345678-1234-1234-1234-123456789abc",
        )
        .with_property("nvme.eui64", "0011223344556677")
        .with_property("nvme.nguid", "00112233445566778899aabbccddeeff")
        .with_property("nvme.subsystem", "nvme-subsys0")
        .with_property("nvme.controller", "nvme0")
        .with_property("nvme.address", "0000:01:00.0")
        .with_property("nvme.transport", "pcie")
        .with_property("nvme.controller-id", "1")
        .with_property("nvme.namespace-capacity", "900000000000")
        .with_property("nvme.lba-format", "512 B + 0 B")
        .with_property("nvme.maximum-lba", "1953125")
        .with_property("nvme.sector-size", "512")
        .with_property("nvme.ana-state", "optimized");
        assert_eq!(
            usage_details(&nvme),
            "generic=/dev/ng0n1 nvme-model=Example NVMe product=Example Controller firmware=1.0 ns-index=0 namespace=1 nsid=1 ns-uuid=12345678-1234-1234-1234-123456789abc eui64=0011223344556677 nguid=00112233445566778899aabbccddeeff subsystem=nvme-subsys0 controller=nvme0 address=0000:01:00.0 transport=pcie controller-id=1 namespace-capacity=900000000000 lba-format=512 B + 0 B max-lba=1953125 sector-size=512 ana-state=optimized"
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
    fn inspect_includes_capacity_usage_identity_properties_and_relationships() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "filesystem:/srv/archive",
                NodeKind::Filesystem,
                "/srv/archive",
            )
            .with_path("/srv/archive")
            .with_size_bytes(1024)
            .with_usage(Usage {
                used_bytes: Some(256),
                free_bytes: Some(768),
                allocated_bytes: Some(512),
            })
            .with_identity(Identity {
                uuid: Some("fs-uuid".to_string()),
                partuuid: None,
                label: Some("archive".to_string()),
                serial: None,
                wwn: None,
            })
            .with_property("filesystem.type", "xfs")
            .with_property("mount.source", "/dev/mapper/archive"),
        );
        graph.add_node(Node::new(
            "block:/dev/mapper/archive",
            NodeKind::DeviceMapper,
            "/dev/mapper/archive",
        ));
        graph.add_edge(Edge::new(
            "block:/dev/mapper/archive",
            "filesystem:/srv/archive",
            Relationship::Backs,
        ));

        let mut output = Vec::new();
        print_inspect(&mut output, &graph, "archive", 1).expect("inspect renders");
        let output = String::from_utf8(output).expect("inspect output is utf8");

        assert!(output.contains("filesystem /srv/archive"));
        assert!(output.contains("  path: /srv/archive"));
        assert!(output.contains("  size: 1.0 KiB"));
        assert!(output.contains("  usage: used=256 B free=768 B allocated=512 B use=25.0%"));
        assert!(output.contains("    uuid: fs-uuid"));
        assert!(output.contains("    label: archive"));
        assert!(output.contains("    filesystem.type: xfs"));
        assert!(output.contains("    mount.source: /dev/mapper/archive"));
        assert!(output.contains("    in backs block:/dev/mapper/archive (/dev/mapper/archive)"));
    }

    #[test]
    fn inspect_json_depth_walks_layered_relationships() {
        let mut graph = StorageGraph::empty();
        graph.add_node(Node::new(
            "block:/dev/nvme0n1p2",
            NodeKind::Partition,
            "/dev/nvme0n1p2",
        ));
        graph.add_node(Node::new(
            "block:/dev/mapper/cryptroot",
            NodeKind::LuksContainer,
            "cryptroot",
        ));
        graph.add_node(Node::new(
            "lvm-lv:vg/root",
            NodeKind::LvmLogicalVolume,
            "vg/root",
        ));
        graph.add_node(Node::new("filesystem:/", NodeKind::Filesystem, "/"));
        graph.add_edge(Edge::new(
            "block:/dev/nvme0n1p2",
            "block:/dev/mapper/cryptroot",
            Relationship::Backs,
        ));
        graph.add_edge(Edge::new(
            "block:/dev/mapper/cryptroot",
            "lvm-lv:vg/root",
            Relationship::Backs,
        ));
        graph.add_edge(Edge::new(
            "lvm-lv:vg/root",
            "filesystem:/",
            Relationship::Backs,
        ));

        let mut output = Vec::new();
        print_inspect_json(&mut output, &graph, "/", 2).expect("inspect json renders");
        let output = String::from_utf8(output).expect("json is utf8");
        let graph: StorageGraph = serde_json::from_str(&output).expect("valid storage graph json");

        assert_eq!(graph.nodes.len(), 3);
        assert!(graph.nodes.iter().any(|node| node.id.0 == "filesystem:/"));
        assert!(graph.nodes.iter().any(|node| node.id.0 == "lvm-lv:vg/root"));
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.id.0 == "block:/dev/mapper/cryptroot")
        );
        assert!(
            graph
                .nodes
                .iter()
                .all(|node| node.id.0 != "block:/dev/nvme0n1p2")
        );
        assert_eq!(graph.edges.len(), 2);
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
                .with_property("ext.magic-number", "0xEF53")
                .with_property("ext.revision", "1 (dynamic)")
                .with_property("ext.errors-behavior", "Continue")
                .with_property("ext.fs-error-count", "2")
                .with_property("ext.os-type", "Linux")
                .with_property("ext.block-count", "122096646")
                .with_property("ext.reserved-block-count", "6104832")
                .with_property("ext.overhead-clusters", "123456")
                .with_property("ext.free-blocks", "73328197")
                .with_property("ext.first-block", "0")
                .with_property("ext.block-size", "4096")
                .with_property("ext.blocks-per-group", "32768")
                .with_property("ext.inode-count", "30531584")
                .with_property("ext.free-inodes", "27187554")
                .with_property("ext.inodes-per-group", "8192")
                .with_property("ext.raid-stride", "128")
                .with_property("ext.raid-stripe-width", "256")
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
                .with_property("ext.first-error-function", "ext4_lookup")
                .with_property("ext.first-error-block", "9001")
                .with_property("ext.last-error-function", "ext4_journal_check_start")
                .with_property("ext.last-error-block", "9002")
                .with_property("ext.checksum-type", "crc32c")
                .with_property("ext.checksum", "0x12345678"),
        );
        graph.add_node(
            Node::new("fs:/dev/sdb1", NodeKind::Filesystem, "exfat")
                .with_property("exfat.guid", "01234567-89ab-cdef-0123-456789abcdef")
                .with_property("exfat.volume-label", "SHARED")
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
                .with_property("ntfs.device-name", "/dev/sda1")
                .with_property("ntfs.device-state", "11")
                .with_property("ntfs.volume-name", "Windows")
                .with_property("ntfs.volume-serial", "01234567-89abcdef")
                .with_property("ntfs.version", "3.1")
                .with_property("ntfs.cluster-size", "4096")
                .with_property("ntfs.mft-record-size", "1024")
                .with_property("ntfs.mft-zone-multiplier", "0")
                .with_property("ntfs.mft-zone-start", "786432")
                .with_property("ntfs.mft-zone-end", "819200")
                .with_property("ntfs.mft-data-position", "786944")
                .with_property("ntfs.mft-lcn", "4"),
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
            "fstype=ext4 ext-state=clean ext-magic=0xEF53 ext-revision=1 (dynamic) errors=Continue fs-error-count=2 os=Linux blocks=122096646 reserved-blocks=6104832 overhead-clusters=123456 free-blocks=73328197 first-block=0"
        ));
        assert!(output.contains(
            "first-error-function=ext4_lookup first-error-block=9001 last-error-function=ext4_journal_check_start last-error-block=9002"
        ));
        assert!(output.contains(
            "block-size=4096 blocks-per-group=32768 inodes=30531584 free-inodes=27187554 inodes-per-group=8192 raid-stride=128 raid-stripe-width=256"
        ));
        assert!(output.contains(
            "features=has_journal extent metadata_csum flags=signed_directory_hash dir-hash=half_md4 dir-hash-seed=11111111-2222-3333-4444-555555555555"
        ));
        assert!(output.contains("default-mount=user_xattr acl"));
        assert!(output.contains(
            "mount-count=12 max-mount-count=-1 check-interval=0 (<none>) inode-size=256 journal-inode=8 journal-uuid=99999999-aaaa-bbbb-cccc-dddddddddddd"
        ));
        assert!(output.contains("journal-size=1024M"));
        assert!(output.contains("checksum-type=crc32c checksum=0x12345678"));
        assert!(output.contains(
            "guid=01234567-89ab-cdef-0123-456789abcdef exfat-label=SHARED exfatprogs=1.2.4 serial=0x6eef953b sectors=3203072"
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
                "ntfs-device=/dev/sda1 ntfs-device-state=11 ntfs-name=Windows ntfs-serial=01234567-89abcdef ntfs-version=3.1 ntfs-cluster=4096 ntfs-mft-record=1024"
            )
        );
        assert!(output.contains(
            "ntfs-mft-zone-multiplier=0 ntfs-mft-zone-start=786432 ntfs-mft-zone-end=819200 ntfs-mft-data-position=786944 ntfs-mft-lcn=4"
        ));
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
                .with_property("zfs.keystatus", "available")
                .with_property("zfs.recordsize", "1048576")
                .with_property("zfs.dedup", "off")
                .with_property("zfs.checksum", "sha512")
                .with_property("zfs.primarycache", "metadata"),
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
        assert!(output.contains(
            "compression=zstd encryption=aes-256-gcm keystatus=available recordsize=1048576"
        ));
        assert!(output.contains("dedup=off checksum=sha512 primarycache=metadata"));
        assert!(output.contains("bcachefs-state=rw bcachefs-device-free=8589934592"));
    }

    #[test]
    fn btrfs_table_includes_subvolume_qgroup_and_json_neighbors() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "block:/dev/nvme0n1p2",
                NodeKind::Partition,
                "/dev/nvme0n1p2",
            )
            .with_property("btrfs.device-id", "1")
            .with_property("btrfs.device-stat-write-io-errs", "1")
            .with_property("btrfs.device-stat-read-io-errs", "2")
            .with_property("btrfs.device-stat-flush-io-errs", "3")
            .with_property("btrfs.device-stat-corruption-errs", "4")
            .with_property("btrfs.device-stat-generation-errs", "5"),
        );
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
                "btrfs-subvolume:fs:@home",
                NodeKind::BtrfsSubvolume,
                "@home",
            )
            .with_property("btrfs.id", "257")
            .with_property("btrfs.parent-id", "5")
            .with_property("btrfs.top-level", "5")
            .with_property("btrfs.mount-target", "/mnt/persist/@home"),
        );
        graph.add_node(
            Node::new(
                "btrfs-snapshot:fs:@home-before",
                NodeKind::BtrfsSnapshot,
                "@home-before",
            )
            .with_property("btrfs.id", "258")
            .with_property("btrfs.parent-uuid", "home-subvol")
            .with_property("btrfs.received-uuid", "received-home"),
        );
        graph.add_node(
            Node::new("btrfs-qgroup:0/257", NodeKind::BtrfsQgroup, "0/257")
                .with_property("btrfs.qgroup-id", "0/257")
                .with_property("btrfs.qgroup-parents", "0/5")
                .with_property("btrfs.max-referenced", "25GiB")
                .with_property("btrfs.max-exclusive", "10GiB"),
        );
        graph.add_edge(Edge::new(
            "block:/dev/nvme0n1p2",
            "btrfs:fs-uuid",
            Relationship::Backs,
        ));
        graph.add_edge(Edge::new(
            "btrfs:fs-uuid",
            "btrfs-subvolume:fs:@home",
            Relationship::Contains,
        ));
        graph.add_edge(Edge::new(
            "btrfs-subvolume:fs:@home",
            "btrfs-snapshot:fs:@home-before",
            Relationship::SnapshotOf,
        ));

        let mut output = Vec::new();
        print_btrfs(&mut output, &graph).expect("Btrfs table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("MOUNT"));
        assert!(output.contains("/mnt/persist"));
        assert!(output.contains("500.0 GiB"));
        assert!(output.contains("40.0%"));
        assert!(output.contains("/dev/nvme0n1p2"));
        assert!(output.contains("device-id=1 write-io-errs=1 read-io-errs=2"));
        assert!(output.contains("flush-io-errs=3 corruption-errs=4 generation-errs=5"));
        assert!(output.contains("data-profile=single metadata-profile=DUP"));
        assert!(output.contains("@home"));
        assert!(output.contains("subvol-id=257 parent-id=5 top-level=5"));
        assert!(output.contains("@home-before"));
        assert!(output.contains("parent-uuid=home-subvol received-uuid=received-home"));
        assert!(output.contains("qgroup=0/257 qgroup-parents=0/5"));
        assert!(output.contains("max-rfer=25GiB max-excl=10GiB"));

        let mut json = Vec::new();
        print_filtered_json(&mut json, &graph, is_btrfs_node).expect("Btrfs json renders");
        let json = String::from_utf8(json).expect("json is utf8");
        assert!(json.contains("btrfs:fs-uuid"));
        assert!(json.contains("btrfs-subvolume:fs:@home"));
        assert!(json.contains("btrfs-snapshot:fs:@home-before"));
        assert!(json.contains("block:/dev/nvme0n1p2"));
    }

    #[test]
    fn bcachefs_table_includes_member_usage_and_json_neighbors() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3",
                NodeKind::BcachefsFilesystem,
                "archive",
            )
            .with_size_bytes(10_737_418_240)
            .with_usage(Usage {
                used_bytes: Some(2_147_483_648),
                free_bytes: Some(8_589_934_592),
                allocated_bytes: Some(10_737_418_240),
            })
            .with_property(
                "bcachefs.external-uuid",
                "a2d6fc04-efd0-4e36-aece-2475941d09a3",
            )
            .with_property(
                "bcachefs.internal-uuid",
                "55083d1e-27cf-4929-ada4-3fe6e45cf02c",
            )
            .with_property("bcachefs.mount-target", "/mnt/archive")
            .with_property("bcachefs.device-count", "2")
            .with_property("bcachefs.version", "1.20: (unknown version)")
            .with_property("bcachefs.data-user", "2147483648")
            .with_property("bcachefs.data-cached", "1048576"),
        );
        graph.add_node(
            Node::new(
                "bcachefs-device:a2d6fc04-efd0-4e36-aece-2475941d09a3:0",
                NodeKind::BcachefsDevice,
                "/dev/sdc",
            )
            .with_size_bytes(16_000_900_661_248)
            .with_property("bcachefs.device-label", "hdd.archive")
            .with_property("bcachefs.device-state", "rw")
            .with_property("bcachefs.device-free", "1649975230464")
            .with_property("bcachefs.device-capacity", "16000900661248")
            .with_property("bcachefs.device-data-user", "2147483648"),
        );
        graph.add_edge(Edge::new(
            "bcachefs-device:a2d6fc04-efd0-4e36-aece-2475941d09a3:0",
            "bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3",
            Relationship::MemberOf,
        ));

        let filesystem = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::BcachefsFilesystem)
            .expect("bcachefs filesystem exists");
        assert_eq!(member_count(&graph, filesystem), 1);

        let mut output = Vec::new();
        print_bcachefs(&mut output, &graph).expect("bcachefs table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("MEMBERS"));
        assert!(output.contains("archive"));
        assert!(output.contains("10.0 GiB"));
        assert!(output.contains("20.0%"));
        assert!(output.contains("/mnt/archive"));
        assert!(output.contains("bcachefs-uuid=a2d6fc04-efd0-4e36-aece-2475941d09a3"));
        assert!(output.contains("bcachefs-internal=55083d1e-27cf-4929-ada4-3fe6e45cf02c"));
        assert!(output.contains("bcachefs-version=1.20: (unknown version)"));
        assert!(output.contains("bcachefs-user=2147483648 bcachefs-cached=1048576"));
        assert!(output.contains("hdd.archive"));
        assert!(output.contains("14.6 TiB"));
        assert!(output.contains("bcachefs-label=hdd.archive bcachefs-state=rw"));
        assert!(output.contains("bcachefs-device-free=1649975230464"));
        assert!(output.contains("bcachefs-device-user=2147483648"));

        let mut json = Vec::new();
        print_filtered_json(&mut json, &graph, is_bcachefs_node).expect("bcachefs json renders");
        let json = String::from_utf8(json).expect("json is utf8");
        assert!(json.contains("bcachefs:a2d6fc04-efd0-4e36-aece-2475941d09a3"));
        assert!(json.contains("bcachefs-device:a2d6fc04-efd0-4e36-aece-2475941d09a3:0"));
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
                .with_property("zfs.pool-ashift", "12")
                .with_property("zfs.pool-autotrim", "on")
                .with_property("zfs.pool-autoexpand", "off")
                .with_property("zfs.pool-cachefile", "/etc/zfs/zpool.cache")
                .with_property("zfs.pool-failmode", "wait")
                .with_property("zfs.status", "some devices need attention")
                .with_property("zfs.action", "replace the faulted device")
                .with_property("zfs.scan", "scrub repaired 0B")
                .with_property("zfs.errors", "No known data errors")
                .with_property("zfs.pool-read-errors", "3")
                .with_property("zfs.pool-write-errors", "4")
                .with_property("zfs.pool-checksum-errors", "5"),
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
                .with_property("zfs.keystatus", "available")
                .with_property("zfs.recordsize", "1048576")
                .with_property("zfs.dedup", "off")
                .with_property("zfs.checksum", "sha512")
                .with_property("zfs.copies", "2")
                .with_property("zfs.sync", "disabled")
                .with_property("zfs.primarycache", "metadata")
                .with_property("zfs.secondarycache", "all")
                .with_property("zfs.atime", "off")
                .with_property("zfs.relatime", "on")
                .with_property("zfs.snapdir", "visible")
                .with_property("zfs.acltype", "posixacl")
                .with_property("zfs.xattr", "sa"),
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
            "pool-ashift=12 pool-autotrim=on pool-autoexpand=off pool-cachefile=/etc/zfs/zpool.cache pool-failmode=wait"
        ));
        assert!(output.contains(
            "status=some devices need attention action=replace the faulted device scan=scrub repaired 0B errors=No known data errors pool-read-errors=3 pool-write-errors=4 pool-checksum-errors=5"
        ));
        assert!(
            output
                .contains("data vdev-state=ONLINE read-errors=0 write-errors=1 checksum-errors=2")
        );
        assert!(output.contains("tank/home"));
        assert!(output.contains(
            "compression=zstd quota=500G reservation=10G encryption=aes-256-gcm keystatus=available"
        ));
        assert!(output.contains("recordsize=1048576 dedup=off checksum=sha512 copies=2"));
        assert!(output.contains("sync=disabled primarycache=metadata secondarycache=all"));
        assert!(output.contains("atime=off relatime=on snapdir=visible acltype=posixacl xattr=sa"));
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
            .with_property("lvm.pv-missing", "missing")
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
                .with_property("lvm.missing-pv-count", "1")
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
        assert!(output.contains("pe-start=1.00m pv-missing=missing pv-extents=128000"));
        assert!(output.contains("pv-extents-used=102400 pv-mda-free=1020.00k"));
        assert!(output.contains("pv-device-id=wwn-0x1234"));
        assert!(output.contains("vg-format=lvm2"));
        assert!(output.contains("permissions=writeable"));
        assert!(output.contains("vg-autoactivation=enabled allocation=normal"));
        assert!(output.contains("system-id=host-a lock-type=none"));
        assert!(output.contains("extent=4.00m extents=262144 free-extents=5120"));
        assert!(output.contains("pvs=2 missing-pvs=1 lvs=5 snapshots=1 seqno=17"));
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
            .with_property("iscsi.target", "iqn.2026-06.example:storage")
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
            .with_path("/dev/sdb")
            .with_size_bytes(1_073_741_824)
            .with_property("iscsi.attached-disk", "sdb")
            .with_property("scsi.address", "4:0:0:0")
            .with_property("scsi.transport", "iscsi")
            .with_property("scsi.generic-device", "/dev/sg2")
            .with_property("scsi.state", "running")
            .with_property("scsi.queue-depth", "64"),
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
        assert!(output.contains("PATH"));
        assert!(output.contains("iscsi-session:12"));
        assert!(output.contains("10.0.0.10:3260,1"));
        assert!(output.contains("LOGGED IN"));
        assert!(output.lines().any(|line| {
            line.contains("lun") && line.contains("0") && line.contains("/dev/sdb")
        }));
        assert!(output.contains("target=iqn.2026-06.example:storage"));
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
        assert!(output.contains("scsi-address=4:0:0:0 scsi-generic=/dev/sg2"));
        assert!(output.contains("scsi-transport=iscsi scsi-state=running scsi-queue-depth=64"));
    }

    #[test]
    fn luns_table_includes_scsi_path_and_json_neighbors() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "iscsi-target:iqn.2026-06.example:storage",
                NodeKind::IscsiTarget,
                "iqn.2026-06.example:storage",
            )
            .with_property("iscsi.node-portal", "10.0.0.10:3260,1"),
        );
        graph.add_node(
            Node::new(
                "iscsi-lun:iqn.2026-06.example:storage:0",
                NodeKind::Lun,
                "0",
            )
            .with_path("/dev/sdb")
            .with_size_bytes(1_073_741_824)
            .with_property("iscsi.attached-disk", "sdb")
            .with_property("iscsi.attached-disk-state", "running")
            .with_property("scsi.address", "4:0:0:0")
            .with_property("scsi.host", "4")
            .with_property("scsi.channel", "0")
            .with_property("scsi.target", "0")
            .with_property("scsi.lun", "0")
            .with_property("scsi.transport", "iscsi")
            .with_property("scsi.generic-device", "/dev/sg2")
            .with_property("scsi.state", "running")
            .with_property("scsi.queue-depth", "64"),
        );
        graph.add_node(Node::new(
            "block:/dev/sdb",
            NodeKind::PhysicalDisk,
            "/dev/sdb",
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

        let mut output = Vec::new();
        print_luns(&mut output, &graph).expect("LUN table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("TRANSPORT"));
        assert!(output.contains("GENERIC"));
        assert!(output.contains("1.0 GiB"));
        assert!(output.contains("/dev/sdb"));
        assert!(output.contains("iscsi"));
        assert!(output.contains("/dev/sg2"));
        assert!(output.contains("scsi-address=4:0:0:0 scsi-host=4 scsi-channel=0"));
        assert!(output.contains("scsi-target=0 scsi-lun=0"));
        assert!(output.contains("attached-disk=sdb attached-disk-state=running"));

        let mut json = Vec::new();
        print_filtered_json(&mut json, &graph, is_lun_node).expect("LUN json renders");
        let json = String::from_utf8(json).expect("json is utf8");
        assert!(json.contains("iscsi-lun:iqn.2026-06.example:storage:0"));
        assert!(json.contains("iscsi-target:iqn.2026-06.example:storage"));
        assert!(json.contains("block:/dev/sdb"));
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
            .with_property("zfs.holds", "disk-nix-retain")
            .with_property("zfs.compression", "zstd")
            .with_property("zfs.encryption", "aes-256-gcm")
            .with_property("zfs.keystatus", "available")
            .with_property("zfs.checksum", "sha512")
            .with_property("zfs.copies", "2"),
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
                .contains("userrefs=2 holds=disk-nix-retain compression=zstd encryption=aes-256-gcm keystatus=available")
        );
        assert!(output.contains("checksum=sha512 copies=2"));
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
                .with_property("btrfs.qgroup-parents", "0/5")
                .with_property("btrfs.qgroup-children", "1/257")
                .with_property("btrfs.max-referenced", "25GiB"),
        );
        graph.add_node(
            Node::new("md:/dev/md/root", NodeKind::MdRaid, "/dev/md/root")
                .with_property("md.version", "1.2")
                .with_property("md.uuid", "aaaa:bbbb:cccc:dddd")
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
        assert!(output.contains("qgroup=0/257 qgroup-parents=0/5 qgroup-children=1/257"));
        assert!(output.contains("max-rfer=25GiB"));
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
            .with_property("cryptsetup.luks-keyslot-0-salt", "00 11 22 33")
            .with_property("cryptsetup.luks-keyslot-0-af-stripes", "4000")
            .with_property("cryptsetup.luks-keyslot-0-area-offset", "32768 [bytes]")
            .with_property("cryptsetup.luks-keyslot-0-area-length", "258048 [bytes]")
            .with_property("cryptsetup.luks-keyslot-0-digest-id", "0")
            .with_property("cryptsetup.luks-keyslot-1-type", "luks2")
            .with_property("cryptsetup.luks-keyslot-1-priority", "ignored")
            .with_property("cryptsetup.luks-token-0-type", "systemd-tpm2")
            .with_property("cryptsetup.luks-token-0-keyslot", "0")
            .with_property("cryptsetup.luks-token-0-keyslots", "0")
            .with_property("cryptsetup.luks-token-0-tpm2-pcrs", "0+7")
            .with_property("cryptsetup.luks-token-0-tpm2-hash", "sha256")
            .with_property("cryptsetup.luks-digest-count", "1")
            .with_property("cryptsetup.luks-digests", "0")
            .with_property("cryptsetup.luks-digest-0-type", "pbkdf2")
            .with_property("cryptsetup.luks-digest-0-hash", "sha256")
            .with_property("cryptsetup.luks-digest-0-iterations", "1000")
            .with_property("cryptsetup.luks-digest-0-salt", "aa bb cc dd")
            .with_property("cryptsetup.luks-digest-0-digest", "ee ff 00 11"),
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
        assert!(output.contains("keyslot-0-salt=00 11 22 33 keyslot-0-af-stripes=4000"));
        assert!(output.contains(
            "keyslot-0-area-offset=32768 [bytes] keyslot-0-area-length=258048 [bytes] keyslot-0-digest=0"
        ));
        assert!(output.contains(
            "keyslot-1=luks2 keyslot-1-priority=ignored token-0=systemd-tpm2 token-0-keyslot=0"
        ));
        assert!(
            output.contains("token-0-keyslots=0 token-0-tpm2-pcrs=0+7 token-0-tpm2-hash=sha256")
        );
        assert!(output.contains("digests=1 digest-ids=0 digest-0=pbkdf2"));
        assert!(output.contains("digest-0-hash=sha256 digest-0-iterations=1000"));
        assert!(output.contains("digest-0-salt=aa bb cc dd digest-0-digest=ee ff 00 11"));
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
                .with_property("bcache.set-average-key-size", "16.0k")
                .with_property("bcache.set-root-usage-percent", "3")
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
        assert!(output.contains("set-average-key-size=16.0k set-root-usage-percent=3"));
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
                .with_size_bytes(1_099_511_627_776)
                .with_usage(Usage {
                    used_bytes: Some(268_435_456_000),
                    free_bytes: Some(805_306_368_000),
                    allocated_bytes: Some(1_073_741_824_000),
                })
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
                .with_property("vdo.data-blocks-used-bytes", "268435456")
                .with_property("vdo.overhead-blocks-used", "4096")
                .with_property("vdo.overhead-blocks-used-bytes", "16777216")
                .with_property("vdo.logical-blocks-used", "262144")
                .with_property("vdo.logical-blocks-used-bytes", "1073741824"),
        );
        graph.add_node(
            Node::new(
                "lvm-seg:vg0/archive:0",
                NodeKind::LvmSegment,
                "vg0/archive:0",
            )
            .with_size_bytes(10 * 1024 * 1024 * 1024)
            .with_usage(Usage {
                used_bytes: Some(8 * 1024 * 1024 * 1024),
                free_bytes: None,
                allocated_bytes: None,
            })
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
        assert!(output.contains("USED"));
        assert!(output.contains("FREE"));
        assert!(output.contains("USE%"));
        assert!(output.contains("WRITE"));
        assert!(output.contains("archive"));
        assert!(output.contains("          1T"));
        assert!(output.contains("        250G"));
        assert!(output.contains("   250.0 GiB"));
        assert!(output.contains("   750.0 GiB"));
        assert!(output.contains("  24.4%"));
        assert!(output.contains("normal"));
        assert!(output.contains("sync"));
        assert!(output.contains("backing=/dev/sdb logical=1T physical=250G"));
        assert!(output.contains("stats-size=268435456 stats-used=134217728"));
        assert!(output.contains("vdo-use=50% saving=75%"));
        assert!(output.contains("recovery=100% write-policy=sync configured-write-policy=auto"));
        assert!(output.contains("index-memory=0.25 block-map-cache=128M"));
        assert!(output.contains("compression=enabled deduplication=enabled"));
        assert!(output.contains("vdo-version=47 vdo-release=133524"));
        assert!(output.contains("data-blocks=65536 data-bytes=268435456"));
        assert!(output.contains("overhead-blocks=4096 overhead-bytes=16777216"));
        assert!(output.contains("logical-blocks=262144 logical-bytes=1073741824"));
        assert!(output.contains("vg0/archive:0"));
        assert!(output.contains("    10.0 GiB      8.0 GiB      8.0 GiB"));
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
                .with_property("multipath.path-flags", "ghost")
                .with_property("multipath.path-state", "active ready running ghost"),
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
                .with_property("multipath.path-flags", "faulty shaky")
                .with_property("multipath.path-state", "active ready running faulty shaky"),
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
        assert!(
            output.contains(
                "dm-state=active checker-state=ready online-state=running path-flags=ghost"
            )
        );
        assert!(output.contains("path-state=active ready running ghost"));
        assert!(output.contains("path-flags=faulty shaky"));
        assert!(output.contains("path-state=active ready running faulty shaky"));
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
            Node::new("nvme-controller:nvme0", NodeKind::NvmeController, "nvme0")
                .with_path("/dev/nvme0")
                .with_identity(Identity {
                    serial: Some("SERIAL123".to_string()),
                    ..Identity::default()
                })
                .with_property("nvme.controller", "nvme0")
                .with_property("nvme.model", "Example NVMe")
                .with_property("nvme.firmware", "1.0")
                .with_property("nvme.subsystem", "nqn.2014-08.org.nvmexpress:uuid:12345678")
                .with_property("nvme.controller-id", "1")
                .with_property("nvme.id-ctrl.vid", "5197")
                .with_property("nvme.id-ctrl.ssvid", "5197")
                .with_property("nvme.id-ctrl.mdts", "9")
                .with_property("nvme.id-ctrl.controller-type", "1")
                .with_property("nvme.id-ctrl.oacs", "31")
                .with_property("nvme.id-ctrl.fuses", "1")
                .with_property("nvme.id-ctrl.fna", "4")
                .with_property("nvme.id-ctrl.awun", "255")
                .with_property("nvme.id-ctrl.awupf", "0")
                .with_property("nvme.id-ctrl.acwu", "0")
                .with_property("nvme.id-ctrl.sgls", "131073")
                .with_property("nvme.id-ctrl.namespace-set-id-max", "32")
                .with_property("nvme.id-ctrl.endurance-group-id-max", "8")
                .with_property("nvme.id-ctrl.ana-transition-time", "10")
                .with_property("nvme.id-ctrl.ana-group-max", "4")
                .with_property("nvme.id-ctrl.persistent-event-log-size", "4096")
                .with_property("nvme.id-ctrl.domain-id", "2")
                .with_property("nvme.id-ctrl.warning-composite-temp", "343")
                .with_property("nvme.id-ctrl.critical-composite-temp", "353")
                .with_property("nvme.id-ctrl.minimum-thermal-management-temp", "273")
                .with_property("nvme.id-ctrl.maximum-thermal-management-temp", "358")
                .with_property("nvme.id-ctrl.total-nvm-capacity", "1000000000")
                .with_property("nvme.id-ctrl.unallocated-nvm-capacity", "500000000")
                .with_property("nvme.id-ctrl.namespace-count", "16")
                .with_property("nvme.id-ctrl.oncs", "95")
                .with_property("nvme.id-ctrl.volatile-write-cache", "1")
                .with_property("nvme.id-ctrl.sanitize-capabilities", "7")
                .with_property("nvme.id-ctrl.ana-capabilities", "3")
                .with_property("nvme.smart.critical-warning", "0")
                .with_property("nvme.smart.temperature-kelvin", "301")
                .with_property("nvme.smart.available-spare-percent", "100")
                .with_property("nvme.smart.percent-used", "2")
                .with_property("nvme.smart.data-units-read", "123456")
                .with_property("nvme.smart.data-units-written", "654321")
                .with_property("nvme.smart.power-on-hours", "1200")
                .with_property("nvme.smart.unsafe-shutdowns", "3")
                .with_property("nvme.smart.media-errors", "0")
                .with_property("nvme.smart.error-log-entries", "4")
                .with_property("nvme.smart.temperature-sensor-1-kelvin", "300")
                .with_property("nvme.smart.temperature-sensor-2-kelvin", "302")
                .with_property("nvme.smart.temperature-sensor-3-kelvin", "303")
                .with_property("nvme.smart.temperature-sensor-4-kelvin", "304")
                .with_property("nvme.smart.thermal-temp1-transition-count", "5")
                .with_property("nvme.smart.thermal-temp2-transition-count", "6")
                .with_property("nvme.smart.thermal-temp1-total-time", "70")
                .with_property("nvme.smart.thermal-temp2-total-time", "80"),
        );
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
            .with_property("nvme.namespace-id", "1")
            .with_property(
                "nvme.namespace-uuid",
                "12345678-1234-1234-1234-123456789abc",
            )
            .with_property("nvme.eui64", "0011223344556677")
            .with_property("nvme.nguid", "00112233445566778899aabbccddeeff")
            .with_property("nvme.subsystem", "nvme-subsys0")
            .with_property("nvme.controller", "nvme0")
            .with_property("nvme.address", "0000:01:00.0")
            .with_property("nvme.transport", "pcie")
            .with_property("nvme.controller-id", "1")
            .with_property("nvme.namespace-capacity", "900000000000")
            .with_property("nvme.lba-format", "512 B + 0 B")
            .with_property("nvme.maximum-lba", "1953125")
            .with_property("nvme.sector-size", "512")
            .with_property("nvme.ana-state", "optimized")
            .with_property("nvme.formatted-lba-index", "0")
            .with_property("nvme.formatted-lba-data-size", "512")
            .with_property("nvme.formatted-lba-metadata-size", "0")
            .with_property("nvme.formatted-lba-relative-performance", "0")
            .with_property("nvme.id-ns.nsze", "1953125")
            .with_property("nvme.id-ns.ncap", "1800000")
            .with_property("nvme.id-ns.nuse", "900000")
            .with_property("nvme.id-ns.nsfeat", "0")
            .with_property("nvme.id-ns.nlbaf", "1")
            .with_property("nvme.id-ns.flbas", "0")
            .with_property("nvme.id-ns.nmic", "1")
            .with_property("nvme.id-ns.nvmcap", "1000000000"),
        );

        let mut output = Vec::new();
        print_nvme(&mut output, &graph).expect("nvme table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("SERIAL"));
        assert!(output.contains("CONTROLLER"));
        assert!(output.contains("USE%"));
        assert!(output.contains("nvme-controller"));
        assert!(output.contains("nvme0"));
        assert!(output.contains("nqn.2014-08.org.nvmexpress:uuid:12345678"));
        assert!(output.contains("vid=5197 ssvid=5197 mdts=9 controller-type=1"));
        assert!(
            output
                .contains("optional-admin-commands=31 fused-operations=1 format-nvm-attributes=4")
        );
        assert!(output.contains(
            "atomic-write-unit-normal=255 atomic-write-unit-powerfail=0 atomic-compare-write-unit=0 sgl-support=131073"
        ));
        assert!(output.contains(
            "namespace-set-id-max=32 endurance-group-id-max=8 ana-transition-time=10 ana-group-max=4"
        ));
        assert!(output.contains("persistent-event-log-size=4096 domain-id=2"));
        assert!(output.contains(
            "warning-composite-temp=343 critical-composite-temp=353 min-thermal-management-temp=273 max-thermal-management-temp=358"
        ));
        assert!(
            output.contains("total-nvm-capacity=1000000000 unallocated-nvm-capacity=500000000")
        );
        assert!(output.contains("namespace-count=16 oncs=95 volatile-write-cache=1"));
        assert!(output.contains("sanitize-capabilities=7 ana-capabilities=3"));
        assert!(
            output.contains("critical-warning=0 temperature-k=301 available-spare-percent=100")
        );
        assert!(output.contains("percent-used=2 data-units-read=123456"));
        assert!(output.contains("data-units-written=654321"));
        assert!(output.contains("power-on-hours=1200 unsafe-shutdowns=3 media-errors=0"));
        assert!(output.contains("error-log-entries=4 temp-sensor-1-k=300 temp-sensor-2-k=302"));
        assert!(output.contains("temp-sensor-3-k=303 temp-sensor-4-k=304"));
        assert!(output.contains(
            "thermal-temp1-transitions=5 thermal-temp2-transitions=6 thermal-temp1-total-time=70 thermal-temp2-total-time=80"
        ));
        assert!(output.contains("/dev/nvme0n1"));
        assert!(output.contains("SERIAL123"));
        assert!(output.contains("nvme0"));
        assert!(output.contains("40.0%"));
        assert!(output.contains("generic=/dev/ng0n1 nvme-model=Example NVMe"));
        assert!(output.contains("product=Example Controller firmware=1.0"));
        assert!(output.contains(
            "ns-index=0 namespace=1 nsid=1 ns-uuid=12345678-1234-1234-1234-123456789abc"
        ));
        assert!(output.contains(
            "eui64=0011223344556677 nguid=00112233445566778899aabbccddeeff subsystem=nvme-subsys0"
        ));
        assert!(output.contains("controller=nvme0 address=0000:01:00.0"));
        assert!(output.contains(
            "transport=pcie controller-id=1 namespace-capacity=900000000000 lba-format=512 B + 0 B"
        ));
        assert!(output.contains("max-lba=1953125 sector-size=512 ana-state=optimized"));
        assert!(
            output
                .contains("flba-index=0 flba-data=512 flba-metadata=0 flba-relative-performance=0")
        );
        assert!(output.contains("nsze=1953125 ncap=1800000 nuse=900000 nsfeat=0"));
        assert!(output.contains("nlbaf=1 flbas=0 nmic=1 nvmcap=1000000000"));
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
                .with_property("md.uuid", "aaaa:bbbb:cccc:dddd")
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
                .with_property("md.intent-bitmap", "Internal")
                .with_property("md.persistence", "Superblock is persistent")
                .with_property("md.bitmap", "0/8 pages [0KB], 65536KB chunk")
                .with_property("md.mdstat-state", "active")
                .with_property("md.mdstat-level", "raid1")
                .with_property("md.mdstat-devices", "2/1")
                .with_property("md.mdstat-health", "U_")
                .with_property("md.mdstat-progress", "recovery")
                .with_property("md.mdstat-progress-percent", "20.0%")
                .with_property("md.mdstat-progress-blocks", "209305/1046528")
                .with_property("md.mdstat-finish", "1.2min")
                .with_property("md.mdstat-speed", "12345K/sec")
                .with_property("md.mdstat-bitmap", "0/8 pages [0KB], 65536KB chunk"),
        );
        graph.add_node(
            Node::new("md:/dev/md/root", NodeKind::MdRaid, "/dev/md/root")
                .with_path("/dev/md/root")
                .with_identity(Identity {
                    uuid: Some("eeee:ffff:1111:2222".to_string()),
                    ..Identity::default()
                })
                .with_property("md.scan-metadata", "1.2")
                .with_property("md.uuid", "eeee:ffff:1111:2222")
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
                .with_property("md.member-state", "active sync")
                .with_property("md.mdstat-member-slot", "1")
                .with_property("md.mdstat-member-flags", "F"),
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
        assert!(output.contains("md-uuid=aaaa:bbbb:cccc:dddd"));
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
        assert!(output.contains(
            "persistence=Superblock is persistent bitmap-detail=0/8 pages [0KB], 65536KB chunk"
        ));
        assert!(output.contains("mdstat-state=active mdstat-level=raid1"));
        assert!(output.contains("mdstat-devices=2/1 mdstat-health=U_"));
        assert!(output.contains("mdstat-progress=recovery mdstat-progress-percent=20.0%"));
        assert!(output.contains("mdstat-progress-blocks=209305/1046528"));
        assert!(output.contains("mdstat-finish=1.2min mdstat-speed=12345K/sec"));
        assert!(output.contains("mdstat-bitmap=0/8 pages [0KB], 65536KB chunk"));
        assert!(output.contains("/dev/md/root"));
        assert!(output.contains("md-uuid=eeee:ffff:1111:2222"));
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
        assert!(output.contains("mdstat-member-slot=1 mdstat-member-flags=F"));
    }

    #[test]
    fn loop_table_includes_mapping_and_backing_details() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("block:/dev/loop0", NodeKind::LoopDevice, "/dev/loop0")
                .with_path("/dev/loop0")
                .with_property("loop.back-file", "/var/lib/images/root.img")
                .with_property("loop.backing-inode", "12345")
                .with_property("loop.backing-major-minor", "0:45")
                .with_property("loop.major-minor", "7:0")
                .with_property("loop.offset", "1048576")
                .with_property("loop.sizelimit", "0")
                .with_property("loop.logical-sector-size", "512")
                .with_property("loop.autoclear", "true")
                .with_property("loop.partscan", "true")
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
        assert!(output.contains(
            "back-file=/var/lib/images/root.img back-ino=12345 back-major-minor=0:45 major-minor=7:0"
        ));
        assert!(
            output.contains("logical-sector=512 autoclear=true partscan=true ro=false dio=true")
        );
        assert!(output.contains("loop-backing=true"));
        assert!(output.contains("/dev/loop1"));
        assert!(output.contains("1.0 GiB"));
        assert!(output.contains("/dev/disk/by-id/nvme-loop-backing"));
        assert!(output.contains("sizelimit=1073741824"));
    }

    #[test]
    fn backing_files_table_includes_consumers_and_json_neighbors() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new(
                "file:/var/lib/images/root.img",
                NodeKind::BackingFile,
                "/var/lib/images/root.img",
            )
            .with_path("/var/lib/images/root.img")
            .with_size_bytes(4_294_967_296)
            .with_usage(Usage {
                used_bytes: Some(1_073_741_824),
                free_bytes: Some(3_221_225_472),
                allocated_bytes: Some(4_294_967_296),
            })
            .with_property("loop.backing", "true"),
        );
        graph.add_node(
            Node::new("block:/dev/loop0", NodeKind::LoopDevice, "/dev/loop0")
                .with_path("/dev/loop0")
                .with_property("loop.back-file", "/var/lib/images/root.img")
                .with_property("loop.offset", "0")
                .with_property("loop.read-only", "false"),
        );
        graph.add_edge(Edge::new(
            "file:/var/lib/images/root.img",
            "block:/dev/loop0",
            Relationship::Backs,
        ));

        let file = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "file:/var/lib/images/root.img")
            .expect("backing file exists");
        assert_eq!(consumer_count(&graph, file), 1);

        let mut output = Vec::new();
        print_backing_files(&mut output, &graph).expect("backing files table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("CONSUMERS"));
        assert!(output.contains("/var/lib/images/root.img"));
        assert!(output.contains("4.0 GiB"));
        assert!(output.contains("25.0%"));
        assert!(output.contains("loop-backing=true"));
        assert!(!output.contains("/dev/loop0"));

        let mut json = Vec::new();
        print_filtered_json(&mut json, &graph, is_backing_file_node)
            .expect("backing files json renders");
        let json = String::from_utf8(json).expect("json is utf8");
        assert!(json.contains("file:/var/lib/images/root.img"));
        assert!(json.contains("block:/dev/loop0"));
        assert!(json.contains("\"relationship\":\"backs\""));
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
        graph.add_node(
            Node::new("block:/dev/zram0", NodeKind::ZramDevice, "/dev/zram0")
                .with_path("/dev/zram0")
                .with_size_bytes(8_589_934_592)
                .with_usage(Usage {
                    used_bytes: Some(2_147_483_648),
                    free_bytes: Some(6_442_450_944),
                    allocated_bytes: Some(805_306_368),
                })
                .with_property("zram.algorithm", "zstd")
                .with_property("zram.streams", "8")
                .with_property("zram.compressed", "715827882")
                .with_property("zram.total", "805306368")
                .with_property("zram.memory-used", "900000000")
                .with_property("zram.memory-peak", "900000000")
                .with_property("zram.compression-ratio", "2.67")
                .with_property("zram.mountpoint", "[SWAP]")
                .with_property("zram.swap", "true"),
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
        assert!(output.contains("/dev/zram0"));
        assert!(output.contains("zram-algorithm=zstd zram-streams=8 zram-compressed=715827882"));
        assert!(output.contains(
            "zram-total=805306368 zram-memory-used=900000000 zram-memory-peak=900000000"
        ));
        assert!(output.contains("zram-ratio=2.67 zram-mountpoint=[SWAP] zram-swap=true"));
        assert!(output.contains("0.0%"));
    }

    #[test]
    fn zram_table_includes_compressed_swap_memory_accounting() {
        let mut graph = StorageGraph::empty();
        graph.add_node(
            Node::new("block:/dev/zram0", NodeKind::ZramDevice, "/dev/zram0")
                .with_path("/dev/zram0")
                .with_size_bytes(8_589_934_592)
                .with_usage(Usage {
                    used_bytes: Some(2_147_483_648),
                    free_bytes: Some(6_442_450_944),
                    allocated_bytes: Some(805_306_368),
                })
                .with_property("zram.algorithm", "zstd")
                .with_property("zram.streams", "8")
                .with_property("zram.compressed", "715827882")
                .with_property("zram.data", "2147483648")
                .with_property("zram.total", "805306368")
                .with_property("zram.memory-limit", "0")
                .with_property("zram.memory-used", "900000000")
                .with_property("zram.memory-peak", "900000000")
                .with_property("zram.compression-ratio", "2.67")
                .with_property("zram.mountpoint", "[SWAP]")
                .with_property("zram.swap", "true"),
        );
        graph.add_node(
            Node::new("swap:/dev/sda3", NodeKind::Swap, "/dev/sda3")
                .with_path("/dev/sda3")
                .with_property("swap.type", "partition"),
        );

        let mut output = Vec::new();
        print_zram(&mut output, &graph).expect("zram table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("ALGO"));
        assert!(output.contains("RATIO"));
        assert!(output.contains("MEM-PEAK"));
        assert!(output.contains("/dev/zram0"));
        assert!(output.contains("8.0 GiB"));
        assert!(output.contains("2.0 GiB"));
        assert!(output.contains("768.0 MiB"));
        assert!(output.contains("zstd"));
        assert!(output.contains("2.67"));
        assert!(output.contains("900000000"));
        assert!(output.contains("[SWAP]"));
        assert!(output.contains("zram-compressed=715827882"));
        assert!(output.contains("zram-memory-limit=0"));
        assert!(output.contains("zram-memory-peak=900000000"));
        assert!(!output.contains("/dev/sda3"));

        let mut json = Vec::new();
        print_filtered_json(&mut json, &graph, is_zram_node).expect("zram json renders");
        let json = String::from_utf8(json).expect("json is utf8");
        assert!(json.contains("block:/dev/zram0"));
        assert!(!json.contains("swap:/dev/sda3"));
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
            .with_property("dm.table.targets", "crypt")
            .with_property("dm.table.segment-count", "1")
            .with_property("dm.table.segment.0.start", "0")
            .with_property("dm.table.segment.0.length", "2097152")
            .with_property("dm.table.segment.0.target", "crypt")
            .with_property("dm.table.segment.0.crypt.cipher", "aes-xts-plain64")
            .with_property("dm.table.segment.0.crypt.device", "259:2")
            .with_property("dm.table.segment.0.crypt.offset", "4096")
            .with_property("dm.status.targets", "crypt")
            .with_property("dm.status.segment-count", "1")
            .with_property("dm.status.segment.0.target", "crypt")
            .with_property("dm.status.segment.0.payload", "0 2097152")
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
            Node::new(
                "block:/dev/mapper/cachevol",
                NodeKind::DeviceMapper,
                "cachevol",
            )
            .with_path("/dev/mapper/cachevol")
            .with_property("dm.name", "cachevol")
            .with_property("dm.table.targets", "cache")
            .with_property("dm.table.segment-count", "1")
            .with_property("dm.table.segment.0.target", "cache")
            .with_property("dm.table.segment.0.metadata-device", "253:10")
            .with_property("dm.table.segment.0.cache-device", "253:11")
            .with_property("dm.table.segment.0.origin-device", "253:12")
            .with_property("dm.table.segment.0.block-size", "128")
            .with_property("dm.status.targets", "cache")
            .with_property("dm.status.segment-count", "1")
            .with_property("dm.status.segment.0.target", "cache")
            .with_property("dm.status.segment.0.metadata-used-blocks", "64")
            .with_property("dm.status.segment.0.metadata-total-blocks", "256")
            .with_property("dm.status.segment.0.cache-used-blocks", "32")
            .with_property("dm.status.segment.0.cache-total-blocks", "1024")
            .with_property("dm.status.segment.0.read-hits", "900")
            .with_property("dm.status.segment.0.read-misses", "100")
            .with_property("dm.status.segment.0.write-hits", "700")
            .with_property("dm.status.segment.0.write-misses", "50")
            .with_property("dm.status.segment.0.dirty-blocks", "4"),
        );
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
        assert!(output.contains(
            "dm-table-targets=crypt dm-table-segments=1 dm-table-start=0 dm-table-length=2097152 dm-table-target=crypt"
        ));
        assert!(output.contains(
            "dm-crypt-cipher=aes-xts-plain64 dm-crypt-device=259:2 dm-crypt-offset=4096"
        ));
        assert!(output.contains(
            "dm-status-targets=crypt dm-status-segments=1 dm-status-target=crypt dm-status-payload=0 2097152"
        ));
        assert!(output.contains("cachevol"));
        assert!(output.contains(
            "dm-name=cachevol dm-table-targets=cache dm-table-segments=1 dm-table-target=cache"
        ));
        assert!(output.contains(
            "dm-table-metadata-device=253:10 dm-table-cache-device=253:11 dm-table-origin-device=253:12 dm-table-block-size=128"
        ));
        assert!(
            output.contains("dm-status-targets=cache dm-status-segments=1 dm-status-target=cache")
        );
        assert!(output.contains(
            "dm-status-metadata-used=64 dm-status-metadata-total=256 dm-status-cache-used=32 dm-status-cache-total=1024"
        ));
        assert!(output.contains(
            "dm-status-read-hits=900 dm-status-read-misses=100 dm-status-write-hits=700 dm-status-write-misses=50 dm-status-dirty=4"
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
    fn dm_table_includes_table_status_and_json_neighbors() {
        let mut graph = StorageGraph::empty();
        graph.add_node(Node::new(
            "block:/dev/nvme0n1p2",
            NodeKind::Partition,
            "/dev/nvme0n1p2",
        ));
        graph.add_node(
            Node::new(
                "block:/dev/mapper/cryptroot",
                NodeKind::DeviceMapper,
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
            .with_property("dm.table.targets", "crypt")
            .with_property("dm.table.segment-count", "1")
            .with_property("dm.table.segment.0.start", "0")
            .with_property("dm.table.segment.0.length", "2097152")
            .with_property("dm.table.segment.0.target", "crypt")
            .with_property("dm.table.segment.0.crypt.cipher", "aes-xts-plain64")
            .with_property("dm.table.segment.0.crypt.device", "259:2")
            .with_property("dm.table.segment.0.crypt.offset", "4096")
            .with_property("dm.status.targets", "crypt")
            .with_property("dm.status.segment-count", "1")
            .with_property("dm.status.segment.0.target", "crypt")
            .with_property("dm.status.segment.0.payload", "0 2097152"),
        );
        graph.add_edge(Edge::new(
            "block:/dev/nvme0n1p2",
            "block:/dev/mapper/cryptroot",
            Relationship::Backs,
        ));
        graph.add_node(
            Node::new(
                "block:/dev/mapper/cachevol",
                NodeKind::DeviceMapper,
                "cachevol",
            )
            .with_path("/dev/mapper/cachevol")
            .with_property("dm.name", "cachevol")
            .with_property("dm.table.targets", "cache")
            .with_property("dm.table.segment-count", "1")
            .with_property("dm.table.segment.0.target", "cache")
            .with_property("dm.table.segment.0.metadata-device", "253:10")
            .with_property("dm.table.segment.0.cache-device", "253:11")
            .with_property("dm.table.segment.0.origin-device", "253:12")
            .with_property("dm.table.segment.0.block-size", "128")
            .with_property("dm.status.targets", "cache")
            .with_property("dm.status.segment-count", "1")
            .with_property("dm.status.segment.0.target", "cache")
            .with_property("dm.status.segment.0.metadata-used-blocks", "64")
            .with_property("dm.status.segment.0.metadata-total-blocks", "256")
            .with_property("dm.status.segment.0.cache-used-blocks", "32")
            .with_property("dm.status.segment.0.cache-total-blocks", "1024")
            .with_property("dm.status.segment.0.read-hits", "900")
            .with_property("dm.status.segment.0.read-misses", "100")
            .with_property("dm.status.segment.0.write-hits", "700")
            .with_property("dm.status.segment.0.write-misses", "50")
            .with_property("dm.status.segment.0.dirty-blocks", "4"),
        );

        let mut output = Vec::new();
        print_dm(&mut output, &graph).expect("dm table renders");
        let output = String::from_utf8(output).expect("table is utf8");

        assert!(output.contains("TARGETS"));
        assert!(output.contains("STATUS"));
        assert!(output.contains("MAJOR:MINOR"));
        assert!(output.contains("cryptroot"));
        assert!(output.contains("crypt"));
        assert!(output.contains("253:0"));
        assert!(output.contains(
            "dm-name=cryptroot dm-uuid=CRYPT-LUKS2-crypt-uuid-cryptroot dm-major=253 dm-minor=0 open=1 segments=1 events=0"
        ));
        assert!(output.contains(
            "dm-table-targets=crypt dm-table-segments=1 dm-table-start=0 dm-table-length=2097152 dm-table-target=crypt"
        ));
        assert!(output.contains(
            "dm-crypt-cipher=aes-xts-plain64 dm-crypt-device=259:2 dm-crypt-offset=4096"
        ));
        assert!(output.contains(
            "dm-status-targets=crypt dm-status-segments=1 dm-status-target=crypt dm-status-payload=0 2097152"
        ));
        assert!(output.contains("cachevol"));
        assert!(output.contains("cache"));
        assert!(output.contains(
            "dm-table-metadata-device=253:10 dm-table-cache-device=253:11 dm-table-origin-device=253:12 dm-table-block-size=128"
        ));
        assert!(output.contains(
            "dm-status-read-hits=900 dm-status-read-misses=100 dm-status-write-hits=700 dm-status-write-misses=50 dm-status-dirty=4"
        ));

        let mut json = Vec::new();
        print_filtered_json(&mut json, &graph, is_dm_node).expect("dm json renders");
        let json = String::from_utf8(json).expect("json is utf8");
        assert!(json.contains("block:/dev/mapper/cryptroot"));
        assert!(json.contains("block:/dev/nvme0n1p2"));
        assert!(json.contains("\"relationship\":\"backs\""));
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
